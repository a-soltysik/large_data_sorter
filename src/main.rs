mod file_reader;
mod merge_sorter;
mod thread_pool;

use std::{env};
use std::thread::available_parallelism;
use std::time::Instant;
use crate::merge_sorter::file::ExecPolicy;

fn dispatch_task(args: &[String]) {
    if args.len() < 2 {
        println!("Too few arguments!");
        return;
    }

    const GENERATOR_FLAG: &str = "-g";
    const SORTER_FLAG: &str = "-s";

    match args[1].as_str() {
        GENERATOR_FLAG => {
            if args.len() < 4 {
                println!("Too few arguments for generator!");
                return;
            }
            match args[2].parse::<usize>() {
                Ok(number) => {
                    handle_generator(number, &args[3]);
                }
                Err(_) => print!("{} is not a number!", args[2]),
            };
        }
        SORTER_FLAG => {
            if args.len() < 4 {
                println!("Too few arguments for generator!");
                return;
            }
            let threads_count = get_threads_count(args);
            println!("Using {} threads", threads_count);
            handle_sorter(&args[2], &args[3], threads_count);
        }
        _ => {
            println!("Unknown flag: {}", args[1]);
        }
    }
}

fn get_threads_count(args: &[String]) -> usize {
    if args.len() > 4 {
        match args[4].parse::<usize>() {
            Ok(number) =>
                return number,
            Err(_) => {
                print!("{} is not a number! Using default number of threads", args[2]);
            }
        };
    };
    match available_parallelism() {
        Ok(number) => number.get(),
        Err(e) => {
            println!("Error: {}", e);
            1
        }
    }
}

fn handle_generator(numbers_count: usize, file_path: &str) {
    let result = file_reader::write_random_data(&file_path, numbers_count);
    match result {
        Err(err) => println!("{}", err),
        _ => {}
    };
}

fn handle_sorter(file_in: &str, file_out: &str, threads_count: usize) {
    let now = Instant::now();

    //let data = file_reader::load_file_to_vec::<usize>(&file_in);
    merge_sorter::file::merge_sort::<usize>(file_in, file_out, 50000000, 16, ExecPolicy::FileSeqRamPar);
    //let sorted = merge_sorter::ram::merge_sort(&data, threads_count);
    println!("File has been sorted in {} s", now.elapsed().as_secs());

    /*let result = file_reader::write_from_vec(
        &file_out,
        &sorted,
        "\n",
    );
    match result {
        Err(err) => println!("{}", err),
        _ => {}
    };*/
}

fn main() {
    dispatch_task(&env::args().collect::<Vec<String>>())
}
