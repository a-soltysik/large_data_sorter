use crate::thread_pool::ThreadPool;
use std::{
    sync::{Arc, Mutex},
};

pub fn merge_sort<T: Send + Sync + Clone + PartialOrd + 'static>(
    slice: &[T],
    threads_count: usize,
) -> Vec<T> {
    match threads_count {
        0 | 1 => merge_sort_seq(slice),
        _ => merge_sort_par(slice, Arc::new(Mutex::new(ThreadPool::new(threads_count))), 0),
    }
}

pub fn merge_sort_seq<T: Clone + PartialOrd>(slice: &[T]) -> Vec<T> {
    match slice.len() {
        0..=1 => slice.to_vec(),
        2 => {
            if slice[0] > slice[1] {
                return vec![slice[1].clone(), slice[0].clone()];
            }
            slice.to_vec()
        }
        _ => {
            let middle = slice.len() / 2;
            let left_sorted = merge_sort_seq(&slice[0..middle]);
            let right_sorted = merge_sort_seq(&slice[middle..]);

            merge(&left_sorted, &right_sorted)
        }
    }
}

pub fn merge_sort_par<T: Send + Sync + Clone + PartialOrd>(slice: &[T], pool: Arc<Mutex<ThreadPool<Vec<T>>>>,
                                                           level: usize) -> Vec<T> {
    if level >= pool.lock().unwrap().size().ilog2() as usize {
        merge_sort_seq(slice)
    } else {
        merge_sort_par_unchecked(slice, pool, level)
    }
}

fn merge_sort_par_unchecked<T: Send + Sync + Clone + PartialOrd>(slice: &[T], pool: Arc<Mutex<ThreadPool<Vec<T>>>>,
                                                                 level: usize) -> Vec<T> {
    match slice.len() {
        0..=1 => slice.to_vec(),
        2 => {
            if slice[0] > slice[1] {
                return vec![slice[1].clone(), slice[0].clone()];
            }
            slice.to_vec()
        }
        _ => {
            let middle = slice.len() / 2;

            let left = slice[0..middle].to_vec();
            let new_pool = Arc::clone(&pool);

            let result = pool.lock().unwrap().execute(move || merge_sort_par(&left, new_pool, level + 1));
            let right_sorted = merge_sort_par(&slice[middle..], Arc::clone(&pool), level + 1);
            let left_sorted = result.recv().unwrap();

            merge(&left_sorted, &right_sorted)
        }
    }
}

fn merge<T: PartialOrd + Clone>(left: &[T], right: &[T]) -> Vec<T> {
    let mut left_pos = 0;
    let mut right_pos = 0;
    let mut merged: Vec<T> = Vec::new();
    merged.reserve(left.len() + right.len());

    while left_pos != left.len() && right_pos < right.len() {
        if left[left_pos] < right[right_pos] {
            merged.push(left[left_pos].clone());
            left_pos += 1;
        } else {
            merged.push(right[right_pos].clone());
            right_pos += 1;
        }
    }

    if left_pos < left.len() {
        merged.extend_from_slice(&left[left_pos..]);
    }

    if right_pos < right.len() {
        merged.extend_from_slice(&right[right_pos..]);
    }

    return merged;
}

#[cfg(test)]
mod tests {
    use std::thread::available_parallelism;
    use super::*;

    #[test]
    fn test_merge() {
        let merged = merge([1, 4, 8, 10].as_slice(), [2, 3, 8, 9].as_slice());
        assert_eq!(merged, vec![1, 2, 3, 4, 8, 8, 9, 10]);
    }

    #[test]
    fn test_merge_sort_seq() {
        let unsorted = vec![5, 1, 9, 10, 3, 45, 2, 4, 4, 12];
        let mut sorted = unsorted.clone();
        sorted.sort();
        assert_eq!(merge_sort(&unsorted, 1), sorted)
    }

    #[test]
    fn test_merge_sort_par() {
        let unsorted = vec![5, 1, 9, 10, 3, 45, 2, 4, 4, 12];
        let mut sorted = unsorted.clone();
        sorted.sort();
        assert_eq!(merge_sort(&unsorted, available_parallelism().unwrap().get()), sorted)
    }
}
