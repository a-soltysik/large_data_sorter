mod file_reader;
mod merge_sorter;
mod thread_pool;
mod checker;

use argh::FromArgs;
use std::thread::available_parallelism;
use std::time::Instant;
use crate::merge_sorter::file::ExecPolicy;

#[derive(FromArgs, PartialEq, Debug)]
/// Configuration
struct Config {
    #[argh(subcommand)]
    mode: Mode,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Mode {
    Generator(Generator),
    Sorter(Sorter),
    Checker(Checker)
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "generator")]
/// generates a file with random u32 numbers
struct Generator {
    /// output path for generator
    #[argh(option, short = 'o')]
    output_path: String,

    /// u32 numbers count to be generated
    #[argh(option, short = 'n')]
    numbers_count: usize,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "sorter")]
/// sorts a file using merge-sort algorithm
struct Sorter {
    /// path for input data to sort
    #[argh(option, short = 'i')]
    input_path: String,

    /// path for sorted output
    #[argh(option, short = 'o')]
    output_path: String,

    /// maximum threads count to be used during sorting
    #[argh(option, short = 't', default = "available_threads()")]
    threads_count: usize,

    /// maximum size of the file that can be sorted in ram
    #[argh(option, short = 's', default = "default_ram()")]
    max_size: usize,

    /// available values:                                          |
    /// FullPar - sorting both files and in ram is parallel        |
    /// FilePar - only sorting a file is parallel                  |
    /// RamPar - only sorting in ram is parallel                   |
    #[argh(option, short = 'e', default = "ExecPolicy::FullPar")]
    exec_policy: ExecPolicy,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand, name = "checker")]
/// checks if the given file is sorted
struct Checker {
    /// path of file to be checked
    #[argh(option, short = 'i')]
    input_path: String,
}

fn available_threads() -> usize {
    match available_parallelism() {
        Ok(number) => number.get(),
        Err(e) => {
            println!("Error: {}. Using only 1 thread", e);
            1
        }
    }
}

fn default_ram() -> usize {
    (sys_info::mem_info().expect("Failed to get ram size. Need to specify it manually").total * 1000 / 4) as usize
}

fn dispatch_task(config: Config) {
    match config.mode {
        Mode::Generator(generator) => {
            let now = Instant::now();
            let result = file_reader::write_random_data(&generator.output_path, generator.numbers_count);
            println!("File has been generated in {} ms", now.elapsed().as_millis());
            match result {
                Err(err) => println!("{}", err),
                _ => {}
            };
        }
        Mode::Sorter(sorter) => {
            let now = Instant::now();
            merge_sorter::file::merge_sort::<u32>(&sorter.input_path, &sorter.output_path, sorter.max_size, sorter.threads_count, sorter.exec_policy);
            println!("File has been sorted in {} ms", now.elapsed().as_millis());
        }
        Mode::Checker(checker) => {
            match checker::is_sorted::<u32>(&checker.input_path) {
                Some(result) => {
                    if result {
                        println!("File {} is sorted", checker.input_path);
                    } else {
                        println!("File {} is not sorted", checker.input_path);
                    }
                }
                None => {
                    println!("Error occured while reading {}", checker.input_path);
                }
            };
        }
    }
}

fn main() {
    dispatch_task(argh::from_env());
}
