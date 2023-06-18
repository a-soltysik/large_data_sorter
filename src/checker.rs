use std::fs::File;
use std::io::{BufRead, BufReader};
use std::str::FromStr;

pub fn is_sorted<T: FromStr + PartialOrd>(path: &str) -> Option<bool> {
    let mut file = BufReader::with_capacity(10_000_000, File::open(path).unwrap());

    let mut first_line = String::new();
    let _ = file.read_line(&mut first_line);

    let mut prev = match first_line.trim().parse::<T>() {
        Ok(line) => line,
        Err(_) => {
            return None;
        }
    };
    for (i, line) in file.lines().enumerate() {
        match line.unwrap().trim().parse::<T>() {
            Ok(current) => {
                if i != 0 {
                    if prev > current{
                        return Some(false);
                    }
                } else {
                    prev = current;
                }
            }
            Err(_) => {
                return None;
            }
        }
    };
    return Some(true);
}