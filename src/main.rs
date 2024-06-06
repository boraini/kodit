use std::{fs::File, io::{self, BufRead}};

use clap::Parser;
use kodit::lexing_specification::{v0::LexingSpecificationV0, LexingSpecification};

mod kodit;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[arg(short, long)]
    lexer: Vec<String>,

    file_name: Option<String>,
}

fn main() {
    let args = Cli::parse();

    if args.file_name.is_none() {
        println!("Usage: kodit [--lex <lexing file name>] <entry file name>");
    }

    let mut lexing_specification : Vec<Box<dyn LexingSpecification>> = args.lexer.iter().map(|file_name| {
        Box::new(LexingSpecificationV0::from_file(file_name).unwrap()) as _
    }).collect();

    lexing_specification.push(
        Box::new(LexingSpecificationV0::from_file("lexing-specifications/en.yml").unwrap())
    );

    let path = args.file_name.as_ref().unwrap();

    let file = File::open(path).unwrap();

    let lines = io::BufReader::new(file).lines();

    let mut vm = kodit::vm::VM::new();

    let lines: Vec<String> = lines.map(|line| {line.unwrap()}).collect();

    // We don't remove empty lines because possible debugging would require the exact line number.
    let raw_lines = kodit::line::decompose_lines(&lines).unwrap();

    let code = kodit::lexer::lex(&raw_lines, &lexing_specification).unwrap();

    vm.evaluate(path, &code);
}
