use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};
use std::sync::atomic::{AtomicBool, Ordering};

pub trait Channel: Send + Sync + 'static {}
impl<T: Send + Sync + 'static> Channel for T {}

pub struct ThreadPool<T: Channel> {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<JobData<T>>>,
}

type Job<T> = Box<dyn FnOnce() -> T + Send + Sync + 'static>;

struct JobData<T: Channel> {
    job: Job<T>,
    callback: mpsc::Sender<T>,
}

impl<T: Channel> ThreadPool<T> {
    pub fn new(size: usize) -> ThreadPool<T> {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for _ in 0..size {
            workers.push(Worker::new(Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender) }
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }

    pub fn execute<F>(&self, f: F) -> mpsc::Receiver<T>
        where
            F: FnOnce() -> T + Channel,
    {
        let (callback_sender, callback_receiver) = mpsc::channel();
        let job_data = JobData {
            job: Box::new(f),
            callback: callback_sender,
        };
        self.sender.as_ref().unwrap().send(job_data).unwrap();

        callback_receiver
    }

    pub fn available_workers(&self) -> usize {
        self.workers.iter().filter(|&worker| worker.is_available.load(Ordering::Relaxed)).count()
    }

    pub fn is_available(&self) -> bool {
        self.available_workers() > 0
    }
}

impl<T: Channel> Drop for ThreadPool<T> {
    fn drop(&mut self) {
        drop(self.sender.take());
        for worker in &mut self.workers {
            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    is_available: Arc<AtomicBool>,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new<T: Channel>(receiver: Arc<Mutex<mpsc::Receiver<JobData<T>>>>) -> Worker {
        let is_available = Arc::new(AtomicBool::new(true));
        let is_available_copy = Arc::clone(&is_available);
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();
            is_available_copy.store(false, Ordering::Relaxed);

            match message {
                Ok(job_data) => {
                    let job_result = (job_data.job)();
                    job_data.callback.send(job_result).unwrap();
                }
                Err(_) => break
            };
            is_available_copy.store(true, Ordering::Relaxed);
        });
        Worker { is_available, thread: Some(thread) }
    }
}
