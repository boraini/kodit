use std::collections::HashMap;

use crate::kodit::environment::{Environment, Label};
use crate::kodit::line::{LineItem, LineItemType, decompose_line};

pub struct VM {
    pub environment: Vec<Environment>,
    pub labels: HashMap<String, Label>,
}

impl VM {
    pub fn new() -> Self {
        VM { environment: Vec::new(), labels: HashMap::new() }
    }

    pub fn evaluate_lines(&mut self, file: &String, lines: Vec<String>) {
        let mut code: Vec<Vec<LineItem>> = Vec::new();

        // We don't remove empty lines because possible debugging would require the exact line number.
        for line in lines {
            let decomposed = decompose_line(&line);

            code.push(decomposed);
        }

        self.evaluate(file, &code);
    }

    pub fn evaluate(&mut self, file: &String, code: &Vec<Vec<LineItem>>) {
        let mut current_line_number = 0;

        // Record all the label positions.
        for current_line in code {
            if (current_line.is_empty()) {
                current_line_number += 1;
                continue;
            }

            match current_line[0].item_type {
                LineItemType::COMMAND => match current_line[0].string_value.to_lowercase().as_str() {
                    "label" | "function" => self.add_label(match current_line[1].item_type {
                        LineItemType::LABEL => current_line[1].string_value.to_owned(),
                        _ => panic!("Not valid "),
                    }, file.to_owned(), current_line_number),
                    _ => (),
                },
                _ => panic!("Line parsing error: The first item in the line is supposed to be a command."),
            }

            current_line_number += 1;
        }

        current_line_number = 0;

        // Execute.
        while current_line_number < code.len() {
            let current_line = &code[current_line_number];

            if (current_line.is_empty()) {
                current_line_number += 1;
                continue;
            }

            match current_line[0].item_type {
                LineItemType::COMMAND => match current_line[0].string_value.to_lowercase().as_str() {
                    "label" => {
                        current_line_number += 1;
                    },
                    "function" => {
                        current_line_number += 1;
                    },
                    "goto" => {
                        let label_text = match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let label = match self.labels.get(label_text) {
                            Some(label) => label,
                            None => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        current_line_number = label.line_number;
                    }
                    "say" => {
                        self.say(match current_line[1].item_type {
                            LineItemType::NUMBER => current_line[1].number_value.to_string(),
                            LineItemType::STRING => current_line[1].string_value.to_owned(),
                            LineItemType::LABEL => todo!(),
                            LineItemType::COMMAND => panic!("Unexpected argument of type command."),
                        });
                        current_line_number += 1;
                    },
                    _ => {
                        current_line_number += 1;
                    }
                },
                _ => panic!("Line parsing error: The first item in the line is supposed to be a command."),
            }
        }
    }

    pub fn add_label(&mut self, name: String, file: String, line_number: usize) {
        self.labels.insert(name, Label {file, line_number});
    }

    pub fn say(&self, message: String) {
        print!("{}", message);
    }
}