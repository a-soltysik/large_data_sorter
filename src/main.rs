mod file_reader;
mod merge_sorter;

use std::env;

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
            handle_sorter(&args[2], &args[3])
        }
        _ => {
            println!("Unknown flag: {}", args[1]);
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

fn handle_sorter(file_in: &str, file_out: &str) {
    let result = file_reader::write_from_vec(
        &file_out,
        &merge_sorter::merge_sort(&file_reader::load_file_to_vec::<usize>(&file_in)),
        "\n",
    );
    match result {
        Err(err) => println!("{}", err),
        _ => {}
    };
}

fn main() {
    dispatch_task(&env::args().collect::<Vec<String>>())
}
