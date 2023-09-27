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

    let mut vm = kodit::VM::VM::new();

    vm.evaluate_lines(path, lines.map(|line| {line.unwrap()}).collect());
}
