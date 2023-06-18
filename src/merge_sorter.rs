pub mod ram {
    use crate::thread_pool::{Channel, ThreadPool};
    use std::sync::{Arc, Mutex};

    pub trait Sort: Clone + PartialOrd + std::fmt::Debug + Ord {}

    impl<T: Clone + PartialOrd + std::fmt::Debug + Ord> Sort for T {}

    pub fn merge_sort<T: Sort + Channel>(slice: &[T], threads_count: usize) -> Vec<T> {
        match threads_count {
            0 | 1 => merge_sort_seq(slice),
            _ => merge_sort_par(slice, threads_count),
        }
    }

    pub fn merge_sort_seq<T: Sort>(slice: &[T]) -> Vec<T> {
        match slice.len() {
            0..=1 => slice.to_vec(),
            2 => {
                if slice[0] > slice[1] {
                    return vec![slice[1].clone(), slice[0].clone()];
                }
                slice.to_vec()
            }
            3..=100 => {
                let mut result = slice.to_vec();
                result.sort();
                result
            }
            _ => {
                let middle = slice.len() / 2;
                let left_sorted = merge_sort_seq(&slice[0..middle]);
                let right_sorted = merge_sort_seq(&slice[middle..]);
                merge(&left_sorted, &right_sorted)
            }
        }
    }

    pub fn merge_sort_par<T: Sort + Channel>(slice: &[T], threads_count: usize) -> Vec<T> {
        merge_sort_par_helper_from_pool(&slice, Arc::new(Mutex::new(ThreadPool::new(threads_count))))
    }

    pub fn merge_sort_par_helper_from_pool<T: Sort + Channel>(slice: &[T], pool: Arc<Mutex<ThreadPool<()>>>) -> Vec<T> {
        if pool.lock().unwrap().is_available() {
            merge_sort_par_helper_unchecked(slice, pool)
        } else {
            merge_sort_seq(slice)
        }
    }

    fn merge_sort_par_helper_unchecked<T: Sort + Channel>(slice: &[T], pool: Arc<Mutex<ThreadPool<()>>>) -> Vec<T> {
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
                let left_sorted = Arc::new(Mutex::new(Vec::<T>::new()));
                let left_sorted_copy = Arc::clone(&left_sorted);
                let result = pool.lock().unwrap().execute(move || *left_sorted_copy.lock().unwrap() = merge_sort_par_helper_from_pool(&left, new_pool));
                let right_sorted = merge_sort_par_helper_from_pool(&slice[middle..], Arc::clone(&pool));
                result.recv().unwrap();
                let left_sorted = Arc::try_unwrap(left_sorted).unwrap().into_inner().unwrap();

                merge(&left_sorted, &right_sorted)
            }
        }
    }

    fn merge<T: Sort>(left: &[T], right: &[T]) -> Vec<T> {
        let mut left_pos = 0;
        let mut right_pos = 0;
        let mut merged: Vec<T> = Vec::with_capacity(left.len() + right.len());

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
    use std::fs::File;
    use std::io::{BufRead, BufReader, BufWriter, Write};
    use std::path::MAIN_SEPARATOR_STR;
    use std::str::FromStr;
    use std::sync::{Arc, Mutex};
    use crate::file_reader;
    use crate::file_reader::get_lines_count;

    use crate::merge_sorter::ram;
    use crate::thread_pool::{Channel, ThreadPool};

    pub trait Sort: ram::Sort + FromStr + ToString + std::fmt::Debug {}

    impl<T: ram::Sort + FromStr + ToString + std::fmt::Debug> Sort for T {}

    #[derive(Debug, PartialEq)]
    pub enum ExecPolicy {
        FullPar,
        FilePar,
        RamPar,
    }

    impl FromStr for ExecPolicy {
        type Err = &'static str;

        fn from_str(input: &str) -> Result<ExecPolicy, Self::Err> {
            match input {
                "FullPar" => Ok(ExecPolicy::FullPar),
                "FilePar" => Ok(ExecPolicy::FilePar),
                "RamPar" => Ok(ExecPolicy::RamPar),
                _ => Err("Wrong value"),
            }
        }
    }

    #[derive(Debug)]
    struct FileData {
        file: File,
        path: String,
        lines_count: usize,
    }

    pub fn merge_sort<T: Sort + Channel>(input: &str, output: &str, max_size_in_ram: usize, threads_count: usize, exec_policy: ExecPolicy) {
        match threads_count {
            0 | 1 => merge_sort_seq::<T>(input, output, max_size_in_ram),
            _ => merge_sort_par::<T>(input, output, max_size_in_ram, threads_count, exec_policy)
        }
    }

    pub fn merge_sort_seq<T: Sort + Channel>(input: &str, output_path: &str, max_size_in_ram: usize) {
        let (dir_name, prepared_input) = prepare_input(&input);
        let result = merge_sort_seq_helper::<T>(prepared_input, max_size_in_ram);

        clean(&result.path, &output_path, &dir_name);
    }

    fn merge_sort_seq_helper<T: Sort + Channel>(input: FileData, max_size_in_ram: usize) -> FileData {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram_seq::<T>(&input.path);
        }

        let tmp_output_path = String::from(&input.path) + "m";

        match split_file(input) {
            Ok(files) => {
                let left_sorted = merge_sort_seq_helper::<T>(files.0, max_size_in_ram);
                let right_sorted = merge_sort_seq_helper::<T>(files.1, max_size_in_ram);

                let tmp_output = File::create(&tmp_output_path).expect(format!("Couldn't open the file: {}", &tmp_output_path).as_str());
                merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path, lines_count: 0 })
            }
            Err(unit_file) => unit_file
        }
    }

    pub fn merge_sort_par<T: Sort + Channel>(input: &str, output: &str, max_size_in_ram: usize, threads_count: usize, exec_policy: ExecPolicy) {
        let (dir_name, prepared_input) = prepare_input(&input);

        let result = match exec_policy {
            ExecPolicy::FullPar => {
                merge_sort_full_par_helper::<T>(prepared_input, max_size_in_ram, Arc::new(Mutex::new(ThreadPool::new(threads_count))))
            }
            ExecPolicy::FilePar => {
                merge_sort_file_par_helper::<T>(prepared_input, max_size_in_ram, Arc::new(Mutex::new(ThreadPool::new(threads_count))))
            }
            ExecPolicy::RamPar => {
                merge_sort_ram_par_helper::<T>(prepared_input, max_size_in_ram, Arc::new(Mutex::new(ThreadPool::new(threads_count))))
            }
        };

        clean(&result.path, &output, &dir_name);
    }

    fn merge_sort_full_par_helper<T: Sort + Channel>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        if pool.lock().unwrap().is_available() {
            merge_sort_full_par_helper_unchecked::<T>(input, max_size_in_ram, pool)
        } else {
            merge_sort_seq_helper::<T>(input, max_size_in_ram)
        }
    }

    fn merge_sort_full_par_helper_unchecked<T: Sort + Channel>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram_par::<T>(&input.path, pool);
        }

        merge_sort_file_par::<T>(merge_sort_full_par_helper::<T>, input, max_size_in_ram, pool)
    }

    fn merge_sort_file_par_helper<T: Sort + Channel>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        if pool.lock().unwrap().is_available() {
            merge_sort_file_par_helper_unchecked::<T>(input, max_size_in_ram, pool)
        } else {
            merge_sort_seq_helper::<T>(input, max_size_in_ram)
        }
    }

    fn merge_sort_file_par_helper_unchecked<T: Sort + Channel>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram_seq::<T>(&input.path);
        }

        merge_sort_file_par::<T>(merge_sort_file_par_helper::<T>, input, max_size_in_ram, pool)
    }

    fn merge_sort_ram_par_helper<T: Sort + Channel>(input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        if input.file.metadata().unwrap().len() < max_size_in_ram as u64 {
            return compute_in_ram_par::<T>(&input.path, pool);
        }

        let tmp_output_path = String::from(&input.path) + "m";

        match split_file(input) {
            Ok(files) => {
                let left_sorted = merge_sort_ram_par_helper::<T>(files.0, max_size_in_ram, Arc::clone(&pool));
                let right_sorted = merge_sort_ram_par_helper::<T>(files.1, max_size_in_ram, pool);

                let tmp_output = File::create(&tmp_output_path).expect(format!("Couldn't open the file: {}", &tmp_output_path).as_str());
                merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path, lines_count: 0 })
            }
            Err(unit_file) => unit_file
        }
    }

    fn merge_sort_file_par<T: Sort>(func: fn(FileData, usize, Arc<Mutex<ThreadPool<()>>>) -> FileData, input: FileData, max_size_in_ram: usize, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        let tmp_output_path = String::from(&input.path) + "m";

        match split_file(input) {
            Ok(files) => {
                let new_pool = Arc::clone(&pool);
                let left_sorted = Arc::new(Mutex::new(None));
                let left_sorted_copy = Arc::clone(&left_sorted);

                let left_task = pool.lock().unwrap().execute(move || {
                    let _ = (*left_sorted_copy.lock().unwrap()).insert(func(files.0, max_size_in_ram, new_pool));
                });
                let right_sorted = func(files.1, max_size_in_ram, Arc::clone(&pool));
                left_task.recv().unwrap();

                let left_sorted = Arc::try_unwrap(left_sorted).unwrap().into_inner().unwrap().take().unwrap();

                let tmp_output = File::create(&tmp_output_path).expect(format!("Couldn't open the file: {}", &tmp_output_path).as_str());
                merge::<T>(left_sorted, right_sorted, FileData { file: tmp_output, path: tmp_output_path, lines_count: 0 })
            }
            Err(unit_file) => unit_file
        }
    }

    fn compute_in_ram_par<T: Sort + Channel>(input_path: &str, pool: Arc<Mutex<ThreadPool<()>>>) -> FileData {
        let data = file_reader::load_file_to_vec::<T>(&input_path);
        let output_path = String::from(input_path) + "w";
        let sorted = ram::merge_sort_par_helper_from_pool(&data, pool);
        let _ = file_reader::write_from_vec(&output_path, &sorted, "\n");
        let _ = fs::remove_file(input_path);
        FileData { file: File::open(&output_path).unwrap(), path: output_path, lines_count: 0 }
    }

    fn compute_in_ram_seq<T: Sort>(input_path: &str) -> FileData {
        let data = file_reader::load_file_to_vec::<T>(&input_path);
        let output_path = String::from(input_path) + "w";
        let sorted = ram::merge_sort_seq(&data);
        let _ = file_reader::write_from_vec(&output_path, &sorted, "\n");
        let _ = fs::remove_file(input_path);
        FileData { file: File::open(&output_path).unwrap(), path: output_path, lines_count: 0 }
    }

    fn split_file(input: FileData) -> Result<(FileData, FileData), FileData> {
        if input.lines_count < 2 {
            return Err(input);
        }

        let file1_path = input.path.clone() + "1";
        let file2_path = input.path.clone() + "2";

        let file1 = File::create(&file1_path).expect(format!("Couldn't open the file: {}", &file1_path).as_str());
        let file2 = File::create(&file2_path).expect(format!("Couldn't open the file: {}", &file2_path).as_str());

        let mut buffer1 = BufWriter::new(file1);
        let mut buffer2 = BufWriter::new(file2);
        let input_buff = BufReader::new(input.file);

        let mut lines_count1 = 0 as usize;
        let mut lines_count2 = 0 as usize;

        for (nr, line) in input_buff.lines().enumerate() {
            if nr < input.lines_count / 2 {
                write_line(&mut buffer1, &line.unwrap());
                lines_count1 += 1;
            } else {
                write_line(&mut buffer2, &line.unwrap());
                lines_count2 += 1;
            }
        }

        let _ = fs::remove_file(input.path);

        Ok((FileData { file: File::open(&file1_path).expect(format!("Couldn't open the file: {}", &file1_path).as_str()), path: file1_path, lines_count: lines_count1 },
            FileData { file: File::open(&file2_path).expect(format!("Couldn't open the file: {}", &file2_path).as_str()), path: file2_path, lines_count: lines_count2 }))
    }

    fn merge<T: Sort>(left: FileData, right: FileData, output: FileData) -> FileData {
        let mut output_buff = BufWriter::new(output.file);
        let mut left_buff = BufReader::new(left.file);
        let mut right_buff = BufReader::new(right.file);

        let mut left_el = get_next::<T>(&mut left_buff);
        let mut right_el = get_next::<T>(&mut right_buff);
        let mut lines_count = 0 as usize;

        loop {
            if left_el.is_some() && right_el.is_some() {
                if left_el < right_el {
                    write_line(&mut output_buff, &left_el.unwrap().clone().to_string());
                    left_el = get_next::<T>(&mut left_buff);
                } else {
                    write_line(&mut output_buff, &right_el.unwrap().clone().to_string());
                    right_el = get_next::<T>(&mut right_buff);
                }
                lines_count += 1;
            } else if left_el.is_some() {
                write_line(&mut output_buff, &left_el.unwrap().clone().to_string());
                lines_count += 1;
                break;
            } else if right_el.is_some() {
                write_line(&mut output_buff, &right_el.unwrap().clone().to_string());
                lines_count += 1;
                break;
            }
        }

        lines_count += write_whole_to(&mut left_buff, &mut output_buff);
        lines_count += write_whole_to(&mut right_buff, &mut output_buff);

        let _ = fs::remove_file(left.path);
        let _ = fs::remove_file(right.path);

        let result = File::open(&output.path).expect(format!("Couldn't open the file: {}", &output.path).as_str());
        FileData { file: result, path: output.path, lines_count }
    }

    fn get_next<T: FromStr>(buffer: &mut BufReader<File>) -> Option<T> {
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

    fn write_whole_to(input: &mut BufReader<File>, output: &mut BufWriter<File>) -> usize {
        let mut lines_count = 0 as usize;
        loop {
            let mut line = String::new();
            match input.read_line(&mut line) {
                Ok(size) => {
                    if size < 2 {
                        break;
                    }
                    let _ = output.write(line.as_bytes());
                    lines_count += 1;
                }
                _ => break
            }
        }
        lines_count
    }

    fn write_line(output: &mut BufWriter<File>, to_write: &str) {
        let _ = output.write(to_write.as_bytes());
        let _ = output.write("\n".as_bytes());
    }

    fn prepare_input(input: &str) -> (&'static str, FileData) {
        let dir_name = "__tmp_merge__";
        let _ = fs::create_dir(&dir_name);

        let copied_input = String::from(dir_name) + MAIN_SEPARATOR_STR + input;
        let _ = fs::copy(&input, &copied_input);
        let lines_count = get_lines_count(&copied_input).unwrap();

        (dir_name, FileData {
            file: File::open(&copied_input).expect(format!("Couldn't open the file: {}", &copied_input).as_str()),
            path: copied_input,
            lines_count,
        })
    }

    fn clean(result_path: &str, output_path: &str, tmp_dir: &str) {
        let _ = fs::rename(&result_path, &output_path);
        let _ = fs::remove_dir(&tmp_dir);
    }
}