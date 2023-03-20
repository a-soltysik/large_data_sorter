use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool<T: Send + Sync + 'static> {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<JobData<T>>>,
}

type Job<T> = Box<dyn FnOnce() -> T + Send + Sync + 'static>;

struct JobData<T: Send + Sync> {
    job: Job<T>,
    callback: mpsc::Sender<T>,
}

impl<T: Send + Sync> ThreadPool<T> {
    pub fn new(size: usize) -> ThreadPool<T> {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();
        let receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&receiver)));
        }

        ThreadPool { workers, sender: Some(sender)}
    }

    pub fn size(&self) -> usize {
        self.workers.len()
    }

    pub fn execute<F>(&self, f: F) -> mpsc::Receiver<T>
    where
        F: FnOnce() -> T + Send + Sync + 'static,
    {
        let (callback_sender,callback_receiver) = mpsc::channel();
        let job_data = JobData {
            job: Box::new(f),
            callback: callback_sender
        };
        self.sender.as_ref().unwrap().send(job_data).unwrap();

        callback_receiver
    }
}

impl<T: Send + Sync> Drop for ThreadPool<T> {
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
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    fn new<T: Send + Sync + 'static>(id: usize, receiver: Arc<Mutex<mpsc::Receiver<JobData<T>>>>) -> Worker {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();
            match message {
                Ok(job_data) => {
                    let job_result = (job_data.job)();
                    job_data.callback.send(job_result).unwrap();
                }
                Err(_) => break
            };
        });
        Worker { id, thread: Some(thread), }
    }
}
