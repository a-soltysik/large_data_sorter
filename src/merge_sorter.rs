pub mod ram {
    use crate::thread_pool::ThreadPool;
    use std::{
        sync::{Arc, Mutex},
    };

    pub fn merge_sort<T>(slice: &[T], threads_count: usize) -> Vec<T>
        where
            T: Send + Sync + Clone + PartialOrd + 'static
    {
        match threads_count {
            0 | 1 => merge_sort_seq(slice),
            _ => merge_sort_par(slice, Arc::new(Mutex::new(ThreadPool::new(threads_count))), 0),
        }
    }

    pub fn merge_sort_seq<T>(slice: &[T]) -> Vec<T>
        where
            T: Clone + PartialOrd
    {
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

    pub fn merge_sort_par<T>(slice: &[T], pool: Arc<Mutex<ThreadPool<Vec<T>>>>, level: usize) -> Vec<T>
        where
            T: Send + Sync + PartialOrd + Clone
    {
        if level >= pool.lock().unwrap().size().ilog2() as usize {
            merge_sort_seq(slice)
        } else {
            merge_sort_par_unchecked(slice, pool, level)
        }
    }

    fn merge_sort_par_unchecked<T>(slice: &[T], pool: Arc<Mutex<ThreadPool<Vec<T>>>>, level: usize) -> Vec<T>
        where
            T: Send + Sync + Clone + PartialOrd
    {
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

    fn merge<T>(left: &[T], right: &[T]) -> Vec<T>
        where
            T: PartialOrd + Clone
    {
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
}

pub mod file {
    use std::fs;
    use std::fs::{File};
    use std::io::{BufRead, BufReader, BufWriter, Write};
    use std::path::{MAIN_SEPARATOR_STR};
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use std::time::Instant;
    use crate::file_reader;

    use crate::merge_sorter::ram;
    use crate::thread_pool::ThreadPool;

    pub enum ExecPolicy {
        FullPar,
        FileParRamSeq,
        FileSeqRamPar,
    }

    struct FileData {
        file: File,
        path: String,
    }

    pub fn merge_sort<T>(input: &str, output: &str, max_size_in_ram: usize, threads_count: usize, exec_policy: ExecPolicy)
        where
            T: FromStr + Clone + PartialOrd + ToString + Send + Sync + 'static
    {
        match threads_count {
            0 | 1 => merge_sort_seq::<T>(input, output, max_size_in_ram),
            _ => merge_sort_par::<T>(input, output, max_size_in_ram, threads_count, exec_policy)
        }
    }

    pub fn merge_sort_seq<T>(input: &str, output_path: &str, max_size_in_ram: usize)
        where
            T: FromStr + Clone + PartialOrd + ToString
    {
        let (dir_name, prepared_input) = prepare_input(&input);
        let result = merge_sort_seq_helper::<T>(prepared_input, max_size_in_ram);

        clean(&result.path, &output_path, &dir_name)
    }

    fn prepare_input(input: &str) -> (&'static str, FileData) {
        let dir_name = "__tmp_merge__";
        let _ = fs::create_dir(&dir_name);

        let copied_input = String::from(dir_name) + MAIN_SEPARATOR_STR + input;
        let _ = fs::copy(&input, &copied_input);

        (dir_name, FileData { file: File::open(&copied_input).expect("Couldn't open the file"), path: copied_input })
    }

    fn clean(result_path: &str, output_path: &str, tmp_dir: &str) {
        let _ = fs::rename(&result_path, &output_path);
        let _ = fs::remove_dir(&tmp_dir);
    }

    fn merge_sort_seq_helper<T>(input: FileData, max_size_in_ram: usize) -> FileData
        where
            T: FromStr + Clone + PartialOrd + ToString
    {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            let data = file_reader::load_file_to_vec::<T>(&input.path);
            let output_path = String::from(&input.path) + "w";
            let _ = file_reader::write_from_vec(&output_path, &ram::merge_sort_seq(&data), "\n");
            let _ = fs::remove_file(input.path);
            return FileData { file: File::open(&output_path).unwrap(), path: output_path };
        }

        let tmp_output_path = String::from(&input.path) + "m";

        let (left, right) = split_file(input);
        let left_sorted = merge_sort_seq_helper::<T>(left, max_size_in_ram);
        let right_sorted = merge_sort_seq_helper::<T>(right, max_size_in_ram);

        let tmp_output = File::create(&tmp_output_path).expect("Couldn't open the file");
        merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path })
    }

    fn merge_sort_par<T>(input: &str, output: &str, max_size_in_ram: usize, threads_count: usize, exec_policy: ExecPolicy)
        where
            T: FromStr + Clone + PartialOrd + ToString + Send + Sync + 'static
    {
        let (dir_name, prepared_input) = prepare_input(&input);

        let result = match exec_policy {
            ExecPolicy::FullPar => {
                merge_sort_full_par_helper::<T>(prepared_input, max_size_in_ram, Arc::new(Mutex::new(ThreadPool::new(threads_count))), 0)
            }
            ExecPolicy::FileParRamSeq => {
                merge_sort_file_par_ram_seq_helper::<T>(prepared_input, max_size_in_ram, Arc::new(Mutex::new(ThreadPool::new(threads_count))), 0)
            }
            ExecPolicy::FileSeqRamPar => {
                merge_sort_file_seq_ram_par_helper::<T>(prepared_input, max_size_in_ram, threads_count)
            }
        };

        clean(&result.path, &output, &dir_name);
    }

    fn merge_sort_full_par_helper<T>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<FileData>>>,
                                     level: usize) -> FileData
        where
            T: FromStr + Clone + PartialOrd + ToString + Send + Sync + 'static
    {
        if pool.lock().unwrap().available_workers() == 0 {
            merge_sort_seq_helper::<T>(input, max_size_in_ram)
        } else {
            merge_sort_full_par_helper_unchecked::<T>(input, max_size_in_ram, pool, level)
        }
    }

    fn merge_sort_full_par_helper_unchecked<T>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<FileData>>>,
                                               level: usize) -> FileData
        where
            T: FromStr + Clone + PartialOrd + ToString + Send + Sync + 'static
    {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram::<T>(&input.path, pool.lock().unwrap().available_workers());
        }

        let tmp_output_path = String::from(&input.path) + "m";

        let (left, right) = split_file(input);

        let new_pool = Arc::clone(&pool);
        let result = pool.lock().unwrap().execute(move || merge_sort_full_par_helper::<T>(left, max_size_in_ram, new_pool, level + 1));
        let right_sorted = merge_sort_full_par_helper::<T>(right, max_size_in_ram, Arc::clone(&pool), level + 1);
        let left_sorted = result.recv().unwrap();

        let tmp_output = File::create(&tmp_output_path)
            .expect("Couldn't open the file");
        merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path })
    }

    fn merge_sort_file_par_ram_seq_helper<T>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<FileData>>>,
                                             level: usize) -> FileData
        where
            T: FromStr + Clone + PartialOrd + ToString + Send + Sync + 'static
    {
        let now = Instant::now();
        if pool.lock().unwrap().available_workers() == 0 {
            println!("Pool in {} ms", now.elapsed().as_millis());
            merge_sort_seq_helper::<T>(input, max_size_in_ram)
        } else {
            println!("Pool in {} ms", now.elapsed().as_millis());
            merge_sort_file_par_ram_seq_helper_unchecked::<T>(input, max_size_in_ram, pool, level)
        }
    }

    fn compute_in_ram<T>(input_path: &str, threads_count: usize) -> FileData
        where
            T: FromStr + Send + Sync + Clone + PartialOrd + ToString + 'static
    {
        let data = file_reader::load_file_to_vec::<T>(&input_path);
        let output_path = String::from(input_path) + "w";
        let _ = file_reader::write_from_vec(&output_path, &ram::merge_sort(&data, threads_count), "\n");
        let _ = fs::remove_file(input_path);
        FileData { file: File::open(&output_path).unwrap(), path: output_path }
    }

    fn merge_sort_file_par_ram_seq_helper_unchecked<T>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<FileData>>>,
                                                       level: usize) -> FileData
        where
            T: FromStr + Send + Sync + Clone + PartialOrd + ToString + 'static
    {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram::<T>(&input.path, 1);
        }

        let tmp_output_path = String::from(&input.path) + "m";

        let (left, right) = split_file(input);

        let new_pool = Arc::clone(&pool);
        let result = pool.lock().unwrap().execute(move || merge_sort_file_par_ram_seq_helper::<T>(left, max_size_in_ram, new_pool, level + 1));
        let right_sorted = merge_sort_file_par_ram_seq_helper::<T>(right, max_size_in_ram, Arc::clone(&pool), level + 1);
        let left_sorted = result.recv().unwrap();

        let tmp_output = File::create(&tmp_output_path)
            .expect("Couldn't open the file");
        merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path })
    }

    fn merge_sort_file_seq_ram_par_helper<T>(input: FileData, max_size_in_ram: usize,
                                             threads_count: usize) -> FileData
        where
            T: FromStr + Send + Sync + Clone + PartialOrd + ToString + 'static
    {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram::<T>(&input.path, threads_count);
        }

        let tmp_output_path = String::from(&input.path) + "m";

        let (left, right) = split_file(input);

        let left_sorted = merge_sort_file_seq_ram_par_helper::<T>(left, max_size_in_ram, threads_count);
        let right_sorted = merge_sort_file_seq_ram_par_helper::<T>(right, max_size_in_ram, threads_count);

        let tmp_output = File::create(&tmp_output_path)
            .expect("Couldn't open the file");
        merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path })
    }

    fn split_file(input: FileData) -> (FileData, FileData) {
        let lines_count = get_lines_count(&input.path);

        let file1_path = input.path.clone() + "1";
        let file2_path = input.path.clone() + "2";

        let file1 = File::create(file1_path.as_str()).expect("Couldn't open the file");
        let file2 = File::create(file2_path.as_str()).expect("Couldn't open the file");

        let mut buffer1 = BufWriter::new(file1);
        let mut buffer2 = BufWriter::new(file2);
        let input_buff = BufReader::new(input.file);

        for (nr, line) in input_buff.lines().enumerate() {
            if nr < lines_count / 2 {
                write_line(&mut buffer1, &line.unwrap());
            } else {
                write_line(&mut buffer2, &line.unwrap());
            }
        }

        let _ = fs::remove_file(input.path);

        (FileData { file: File::open(file1_path.as_str()).expect("Couldn't open the file"), path: file1_path },
         FileData { file: File::open(file2_path.as_str()).expect("Couldn't open the file"), path: file2_path })
    }

    fn get_lines_count(path: &str) -> usize {
        let new_file = File::open(&path).expect("Couldn't open the file");
        BufReader::new(new_file).lines().count()
    }

    fn merge<T>(left: FileData, right: FileData, output: FileData) -> FileData
        where
            T: PartialOrd + Clone + ToString + FromStr
    {
        let mut output_buff = BufWriter::new(output.file);
        let mut left_buff = BufReader::new(left.file);
        let mut right_buff = BufReader::new(right.file);

        let mut left_el = get_next::<T>(&mut left_buff);
        let mut right_el = get_next::<T>(&mut right_buff);

        loop {
            if left_el.is_some() && right_el.is_some() {
                if left_el < right_el {
                    write_line(&mut output_buff, &left_el.unwrap().clone().to_string());
                    left_el = get_next::<T>(&mut left_buff);
                } else {
                    write_line(&mut output_buff, &right_el.unwrap().clone().to_string());
                    right_el = get_next::<T>(&mut right_buff);
                }
            } else if left_el.is_some() {
                write_line(&mut output_buff, &left_el.unwrap().clone().to_string());
                break;
            } else if right_el.is_some() {
                write_line(&mut output_buff, &right_el.unwrap().clone().to_string());
                break;
            }
        }

        write_whole_to::<T>(&mut left_buff, &mut output_buff);
        write_whole_to::<T>(&mut right_buff, &mut output_buff);

        let _ = fs::remove_file(left.path);
        let _ = fs::remove_file(right.path);

        let result = File::open(&output.path).expect("Couldn't open the file");
        FileData { file: result, path: output.path }
    }

    fn get_next<T>(buffer: &mut BufReader<File>) -> Option<T>
        where
            T: FromStr
    {
        let mut data = String::new();
        match buffer.read_line(&mut data) {
            Ok(_) => {
                match data.trim().parse::<T>() {
                    Ok(number) => Option::from(number),
                    Err(_) => None
                }
            }
            Err(_) => None
        }
    }

    fn write_whole_to<T>(input: &mut BufReader<File>, output: &mut BufWriter<File>) {
        loop {
            let mut line = String::new();
            match input.read_line(&mut line) {
                Ok(size) => {
                    if size < 2 {
                        break;
                    }
                    let _ = output.write(line.as_bytes());
                }
                _ => break
            }
        }
    }

    fn write_line(output: &mut BufWriter<File>, to_write: &str) {
        let _ = output.write(to_write.as_bytes());
        let _ = output.write("\n".as_bytes());
    }
}