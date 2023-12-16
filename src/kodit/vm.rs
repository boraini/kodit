use std::collections::HashMap;
use std::io::{self, Write};

use super::environment::{Environment, Label};
use super::line::{LineItem, LineItemType, decompose_line, self};
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
                    "label" | "function" | "for" => self.add_label(match current_line[1].item_type {
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
                    "call" => {
                        let label_text = match current_line[1].item_type {
                            LineItemType::LABEL | LineItemType::STRING => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot call function with a non-label and non-string qualifier.", current_line_number),
                        };
                        let jump_target = match self.labels.get(label_text) {
                            Some(l) => l,
                            None => panic!("Error at line {}: Function not known.", current_line_number),
                        };
                        match self.call_function(&code[jump_target.line_number], current_line, file, jump_target.line_number, current_line_number) {
                            Ok(l) => current_line_number = l.line_number,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };
                    },
                    "return" => {
                        current_line_number = match self.return_from_function(current_line, current_line_number) {
                            Ok(l) => l.line_number,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        }
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
                    "if" => {
                        let cond = match self.line_item_as_value(&current_line[1]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };

                        let which_label = if cond.as_boolean() {2usize} else {3usize};
                        let label_text = match current_line[which_label].item_type {
                            LineItemType::LABEL => &current_line[which_label].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let label = match self.labels.get(label_text) {
                            Some(label) => label,
                            None => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        current_line_number = label.line_number;
                    }
                    "for" => {
                        let label_start_text = match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let label_end_text = match current_line[2].item_type {
                            LineItemType::LABEL => &current_line[2].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let label_start = self.labels.get(label_start_text).unwrap();
                        let label_end = match self.labels.get(label_end_text) {
                            Some(label) => label,
                            None => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        let variable_name = match current_line[3].item_type {
                            LineItemType::LABEL => &current_line[3].string_value,
                            _ => panic!("Error at line {}: Loop variable invalid.", current_line_number),
                        };

                        let end_value = match self.line_item_as_value(&current_line[4]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };

                        let value = match self.read_variable(variable_name) {
                            Some(v) => v,
                            None => panic!("Error at line {}: For loop variable not found.", current_line_number),
                        };

                        let current = match value.value_type {
                            ValueType::Number => value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line_number),
                        };

                        let compare = match end_value.value_type {
                            ValueType::Number => end_value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line_number),
                        };

                        if current < compare {
                            current_line_number += 1;
                        } else {
                            current_line_number = label_end.line_number;
                        }
                    }
                    "continue" => {
                        let label_text = match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let label = match self.labels.get(label_text) {
                            Some(label) => label,
                            None => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        current_line_number = label.line_number;

                        let current_line = &code[current_line_number];

                        if !matches!(current_line[0].item_type, LineItemType::COMMAND) && current_line[0].string_value == "for" {
                            panic!("Error at line {}: Can only continue to a for line.", label.line_number);
                        }

                        let variable_name = match current_line[3].item_type {
                            LineItemType::LABEL => &current_line[3].string_value,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line_number),
                        };

                        let value = match self.read_variable(variable_name) {
                            Some(v) => v,
                            None => panic!("Error at line {}: For loop variable not found.", label.line_number),
                        };

                        let new_value = match value.value_type {
                            ValueType::Number => value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line_number),
                        }.round() as i32 + 1;

                        self.assign_variable(variable_name, Value::number_value(new_value as f64));
                    }
                    "sum" => {
                        let operand = match current_line[2].item_type {
                            LineItemType::LABEL => &current_line[2].string_value,
                            _ => panic!("Error at line {}: Operand must be a label.", current_line_number),
                        };
                        let v1 = match self.line_item_as_value(&current_line[1]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };
                        let v2 = match self.line_item_as_value(&current_line[3]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };
                        let sum = match VM::do_sum(&v1, &v2, &operand) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };

                        self.assign_variable(&"@save".to_string(), sum);
                        current_line_number += 1;
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
                    "table" => {
                        let dimensions = current_line[2..].iter().map(
                            |item| match self.line_item_as_value(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line_number, e),
                            }
                        ).collect::<Vec<usize>>();

                        self.create_table(match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot assign to such variable.", current_line_number),
                        }, &dimensions);
                        current_line_number += 1;
                    },
                    "get" => {
                        let variable_name = match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot read such variable.", current_line_number),
                        };
                        let table = match self.read_variable(variable_name) {
                            Some(v) => match v.value_type {
                                ValueType::Table => v,
                                _ => panic!("Error at line {}: Variable is not a table.", current_line_number),
                            },
                            None => panic!("Error at line {}: Variable {} not found.", current_line_number, variable_name),
                        };
                        let dimensions = current_line[2..].iter().map(
                            |item| match self.line_item_as_value(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line_number, e),
                            }
                        ).collect::<Vec<usize>>();
                        let value = match self.table_manager.get(table, &dimensions) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: Error reading from table. {}", current_line_number, e),
                        }.clone();
                        self.assign_variable(&"@save".to_string(), value);
                        current_line_number += 1;
                    }
                    "put" => {
                        let variable_name = match current_line[1].item_type {
                            LineItemType::LABEL => &current_line[1].string_value,
                            _ => panic!("Error at line {}: Cannot read such variable.", current_line_number),
                        };
                        let table = match self.read_variable(variable_name) {
                            Some(v) => {match v.value_type {
                                ValueType::Table => v,
                                _ => panic!("Error at line {}: Variable is not a table.", current_line_number),
                            }},
                            None => panic!("Error at line {}: Variable {} not found.", current_line_number, variable_name),
                        }.to_owned();
                        let table_dimensions = match self.table_manager.get_dimensions(&table) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: Error reading from table. {}", current_line_number, e)
                        };
                        let dimensions = current_line[2..(2 + table_dimensions.len())].iter().map(
                            |item| match self.line_item_as_value(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line_number, e),
                            }
                        ).collect::<Vec<usize>>();
                        let value = match self.line_item_as_value(&current_line[2 + table_dimensions.len()]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line_number, e),
                        };
                        match self.table_manager.put(&table, &dimensions, value) {
                            Ok(()) => (),
                            Err(e) => panic!("Error at line {}: Error writing to table. {}", current_line_number, e),
                        };
                        current_line_number += 1;
                    },
                    _ => {
                        current_line_number += 1;
                    },
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

    pub fn call_function(&mut self, function_line: &Vec<LineItem>, call_line: &Vec<LineItem>, file: &String, fun: usize, ret: usize) -> Result<Label, &str> {
        // The number 2 is because both the call and function commands begin with two items: command name and function name
        let num_arguments = function_line.len() - 2;
        if num_arguments > call_line.len() - 2 {
            return Err("Not enough arguments are supplied.")
        }
        let new_env = Environment::that_returns_to(Some(Label { file: file.clone(), line_number: ret + 1}));
        self.environment.push(new_env);

        let mut i = 2;
        while i < num_arguments + 2 {
            let value = match self.line_item_as_value(&call_line[i]) {
                Ok(v) => v,
                Err(e) => panic!("Error at line {}: {}", ret, e),
            };
            self.assign_variable(&function_line[i].string_value, value);
            i += 1;
        }
        
        Ok(Label { file: file.clone(), line_number: fun })
    }

    pub fn return_from_function(&mut self, return_line: &Vec<LineItem>, line_number: usize) -> Result<Label, &str> {
        if (self.environment.len() <= 1) {
            return Err("Cannot return from root.");
        }

        let return_value = if return_line.len() > 1 {
            match self.line_item_as_value(&return_line[1]) {
                Ok(v) => v,
                Err(e) => panic!("Error at line {}: {}", line_number, e),
            }
        } else {
            Value::uninitialized()
        };

        let ret = self.environment.pop().unwrap().return_address.unwrap();

        if (return_line.len() > 1) {
            self.assign_variable(&"@save".to_string(), return_value);
        }

        Ok(ret)
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
                return env.variables.get(name);
            }
        }

        None
    }

    pub fn do_sum(v1: &Value, v2: &Value, operand: &String) -> Result<Value, &'static str> {
        let t = (v1.value_type.clone(), v2.value_type.clone());
        match operand.as_str() {
            "+" => match t {
                (ValueType::String, ValueType::String) => Ok(Value::string_value(&format!("{}{}", v1.string_value.as_ref().unwrap(), v2.string_value.as_ref().unwrap()))),
                (ValueType::String, ValueType::Number) => Ok(Value::string_value(&format!("{}{}", v1.string_value.as_ref().unwrap(), v2.number_value))),
                (ValueType::Number, ValueType::String) => Ok(Value::string_value(&format!("{}{}", v1.number_value, v2.string_value.as_ref().unwrap()))),
                (ValueType::Number, ValueType::Number) => Ok(Value::number_value(v1.number_value + v2.number_value)),
                _ => Err("Addition with tables not supported."),
            },
            "==" => match t {
                (ValueType::String, ValueType::String) => Ok(Value::boolean_value(v1.string_value.as_ref().unwrap() == v2.string_value.as_ref().unwrap())),
                (ValueType::Number, ValueType::Number) => Ok(Value::boolean_value(v1.number_value == v2.number_value)),
                _ => Err("Two strings or two numbers can be compared only."),
            }
            "-" | "*" | "/" | "%" => match t {
                (ValueType::Number, ValueType::Number) => Ok(Value::number_value(match operand.as_str() {
                    "-" => v1.number_value - v2.number_value,
                    "*" => v1.number_value * v2.number_value,
                    "/" => v1.number_value / v2.number_value,
                    "%" => v1.number_value % v2.number_value,
                    _ => todo!(),
                })),
                _ => Err("Numerical operation used with non-number arguments."),
            },
            "<" | ">" | "<=" | ">=" => match t {
                (ValueType::Number, ValueType::Number) => Ok(Value::boolean_value(match operand.as_str() {
                    "<" => v1.number_value < v2.number_value,
                    ">" => v1.number_value > v2.number_value,
                    "<=" => v1.number_value <= v2.number_value,
                    ">=" => v1.number_value >= v2.number_value,
                    _ => todo!(),
                })),
				(ValueType::String, ValueType::String) => Ok(Value::boolean_value(match operand.as_str() {
					"<" => v1.string_value.as_ref().unwrap() < v2.string_value.as_ref().unwrap(),
                    ">" => v1.string_value.as_ref().unwrap() > v2.string_value.as_ref().unwrap(),
                    "<=" => v1.string_value.as_ref().unwrap() <= v2.string_value.as_ref().unwrap(),
                    ">=" => v1.string_value.as_ref().unwrap() >= v2.string_value.as_ref().unwrap(),
                    _ => todo!(),
				})),
                _ => Err("Numerical operation used with non-number arguments."),
            },
            _ => Err("Unsupported operand."),
        }
    }

    pub fn say(&self, message: String) {
        print!("{}", message);
    }

    pub fn create_table(&mut self, name: &String, dimensions: &Vec<usize>) {
        let value = self.table_manager.create_table(dimensions);

        self.assign_variable(name, value);

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