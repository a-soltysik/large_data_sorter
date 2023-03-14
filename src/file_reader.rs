use rand::distributions::Distribution;
use rand::distributions::Uniform;
use std::fs::{File, OpenOptions};
use std::io;
use std::io::{BufReader, BufWriter, Read, Write};
use std::str::FromStr;

pub fn load_file(file_path: &str) -> String {
    let file = File::open(file_path);
    match file {
        Ok(file) => {
            let mut buffer = String::new();
            let mut reader = BufReader::new(file);
            match reader.read_to_string(&mut buffer) {
                Ok(_) => buffer,
                Err(_) => String::new(),
            }
        }
        Err(_) => String::new(),
    }
}

pub fn load_file_to_vec<T: FromStr>(file_path: &str) -> Vec<T> {
    read_from_string(&load_file(file_path))
}

pub fn read_from_string<T: FromStr>(data: &str) -> Vec<T> {
    let mut result = Vec::<T>::new();
    for number in data.split_whitespace() {
        match number.parse::<T>() {
            Ok(value) => result.push(value),
            _ => {}
        }
    }
    result
}

pub fn write_from_vec<T: ToString + Sized>(
    file_path: &str,
    data: &[T],
    delimiter: &str,
) -> io::Result<()> {
    let file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(file_path);

    if file.is_err() {
        return Err(file.err().unwrap());
    }

    let elems: Vec<String> = data.iter().map(|elem| elem.to_string()).collect();

    let result = BufWriter::new(file.unwrap()).write_all(elems.join(delimiter).as_bytes());
    if result.is_err() {
        return Err(result.err().unwrap());
    }
    Ok(())
}

pub fn write_random_data(file_path: &str, numbers_count: usize) -> io::Result<()> {
    let file = OpenOptions::new()
        .truncate(true)
        .create(true)
        .write(true)
        .open(file_path);

    if file.is_err() {
        return Err(file.err().unwrap());
    }

    const AVERAGE_BYTES_PER_NUMBER: usize = 3 * 4;
    const CHUNK_SIZE_IN_BYTES: usize = 500_000_000;
    let iters_count = usize::max(
        1,
        numbers_count * AVERAGE_BYTES_PER_NUMBER / CHUNK_SIZE_IN_BYTES,
    );
    let number_count_per_iter = numbers_count / iters_count;

    let mut writer = BufWriter::new(file.unwrap());

    for _ in 0..iters_count {
        let numbers: Vec<String> = Uniform::new(u32::MIN, u32::MAX)
            .sample_iter(&mut rand::thread_rng())
            .take(number_count_per_iter)
            .map(|number| number.to_string())
            .collect();

        let result = writer.write_all(numbers.join("\n").as_bytes());
        if result.is_err() {
            return Err(result.err().unwrap());
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_from_string() {
        let data = "5\n7\t2a12 6 3 7 167 3\n7";
        assert_eq!(
            read_from_string::<u32>(data),
            vec![5, 7, 6, 3, 7, 167, 3, 7]
        );
    }
}
