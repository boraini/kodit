use std::collections::HashMap;
use std::io::{self, Write};

use super::environment::{Environment, Label};
use super::line::{LineItem, LineItemType, decompose_line};
use super::value::{Value, ValueType};

use super::table::*;

pub struct VM {
    pub environment: Vec<Environment>,
    pub labels: HashMap<String, Label>,
    pub table_manager: TableManager,
    pub table_creations: u32,
}

impl VM {
    pub fn new() -> Self {
        VM {
            environment: vec![Environment::that_returns_to(None)],
            labels: HashMap::new(),
            table_manager: TableManager::new(),
            table_creations: 0,
        }
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
                        self.say(self.line_item_to_string(&current_line[1], &current_line_number));
                        current_line_number += 1;
                    },
                    "ask" => {
                        // flush the output also so the user is not confused.
                        self.say(self.line_item_to_string(&current_line[1], &current_line_number));
                        io::stdout().flush().unwrap();

                        let mut buf = String::new();
                        io::stdin().read_line(&mut buf).unwrap();

                        // pop the trailing newline in the string.
                        let trimmed = buf.strip_suffix("\r\n")
                            .or(buf.strip_suffix("\n"))
                            .unwrap_or(buf.as_str());

                        self.assign_variable(&"@save".to_string(), Value {
                            value_type: ValueType::String,
                            string_value: Some(trimmed.to_string()),
                            number_value: 0f64,
                            table_index: 0,
                        });

                        current_line_number += 1;
                    }
                    "set" => {
                        self.assign_variable(match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot assign to such variable.", current_line_number),
                        }, match self.line_item_as_value(&current_line[2]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        });
                        current_line_number += 1;
                    }
                    _ => {
                        current_line_number += 1;
                    }
                },
                _ => panic!("Line parsing error: The first item in the line is supposed to be a command."),
            }
        }
    }

    pub fn line_item_as_value(&self, item: &LineItem) -> Result<Value, String> {
        match item.item_type {
            LineItemType::NUMBER => Ok(Value {
                value_type: super::value::ValueType::Number,
                string_value: None,
                table_index: usize::MAX,
                number_value: item.number_value,
            }),
            LineItemType::STRING => Ok(Value {
                value_type: super::value::ValueType::String,
                string_value: Some(item.string_value.clone()),
                table_index: usize::MAX,
                number_value: 0f64,
            }),
            LineItemType::LABEL => match self.read_variable(&item.string_value) {
                Some(v) => Ok(v.clone()),
                None => Err(format!("Variable {} not found.", item.string_value)),
            },
            LineItemType::COMMAND => panic!("Unexpected argument of type command."),
        }
    }

    pub fn line_item_to_string(&self, item: &LineItem, current_line_number: &usize) -> String {
        match item.item_type {
            LineItemType::NUMBER => item.number_value.to_string(),
            LineItemType::STRING => item.string_value.to_owned(),
            LineItemType::LABEL => {
                let value = match self.line_item_as_value(&item) {
                    Ok(v) => v,
                    Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                };

                match value.value_type {
                    ValueType::Number => value.number_value.to_string(),
                    ValueType::String => value.string_value.unwrap(),
                    ValueType::Table => panic!("Error at line {}: Cannot say a table value.", current_line_number),
                }
            },
            LineItemType::COMMAND => panic!("Unexpected argument of type command."),
        }
    }

    pub fn add_label(&mut self, name: String, file: String, line_number: usize) {
        self.labels.insert(name, Label {file, line_number});
    }

    pub fn assign_variable(&mut self, name: &String, value: Value) {
        let env_index = self.environment.len() - 1;

        self.environment
            .get_mut(env_index).unwrap()
            .variables.insert(name.clone(), value);
    }

    pub fn read_variable(&self, name: &String) -> Option<&Value> {
        for env in self.environment.iter().rev() {
            if env.variables.contains_key(name) {
                return Some(env.variables.get(name).unwrap());
            }
        }

        None
    }

    pub fn say(&self, message: String) {
        print!("{}", message);
    }

    pub fn create_table(&mut self, name: String, dimensions: &Vec<usize>) {
        let value = self.table_manager.create_table(dimensions);
        self.assign_variable(&"@save".to_string(), value);

        self.table_creations += 1;
        if self.table_creations > 20 {
            self.collect_garbage();
            self.table_creations = 0;
        }
    }

    pub fn collect_garbage(&mut self) {
        self.table_manager.mark(&self.environment);
        self.table_manager.sweep();
    }
}