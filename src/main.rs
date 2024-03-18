use std::{env, fs::File, io::{self, BufRead}};

mod kodit;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        println!("Usage: kodit <entry file name>");
        return;
    }

    let path = &args[1];

    let file = File::open(path).unwrap();

    let lines = io::BufReader::new(file).lines();

    let mut vm = kodit::vm::VM::new();

    let lines: Vec<String> = lines.map(|line| {line.unwrap()}).collect();
    vm.evaluate_lines(path, &lines);
}
