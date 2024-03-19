use core::panic;
use std::collections::HashMap;
use std::io::{self, Write};

use super::environment::{Environment, Label};
use super::line::{decompose_lines, Command, Line, LineItem};
use super::value::{Value, ValueType};

use super::table::*;

pub struct VM {
    pub environment: Vec<Environment>,
    pub labels: LabelMap,
    pub table_manager: TableManager,
    pub table_creations: u32,
}

pub struct LabelMap(HashMap<String, Label>);

impl LabelMap {
    pub fn get_line_number_with_inferred(&self, label_name: &str, current_line_number: &usize) -> Option<usize> {
        if label_name == "next" {
            Some(current_line_number + 1)
        } else {
            self.get(label_name).map(|l| l.line_number)
        }
    }
    pub fn get(&self, label_name: &str) -> Option<&Label> {
        self.0.get(label_name)
    }

    pub fn insert(&mut self, label_name: String, value: Label) {
        self.0.insert(label_name, value);
    }
}

impl VM {
    pub fn new() -> Self {
        VM {
            environment: vec![Environment::that_returns_to(None)],
            labels: LabelMap(HashMap::new()),
            table_manager: TableManager::new(),
            table_creations: 0,
        }
    }

    pub fn evaluate_lines(&mut self, file: &String, lines: &[String]) {
        // We don't remove empty lines because possible debugging would require the exact line number.
        let code = decompose_lines(lines).unwrap();

        self.evaluate(file, &code);
    }

    pub fn evaluate(&mut self, file: &String, code: &Vec<Line>) {
        let mut current_line_number = 0;

        // Record all the label positions.
        for current_line in code {
            match &current_line.items[0] {
                LineItem::Command(command) => match command {
                    Command::FOR | Command::LABEL | Command::FUNCTION => self.add_label(match &current_line.items[1] {
                        LineItem::Label(label) => label.to_owned(),
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

            match &current_line.items[0] {
                LineItem::Command(command) => match command {
                    Command::LABEL => {
                        current_line_number += 1;
                    },
                    Command::FUNCTION => {
                        current_line_number += 1;
                    },
                    Command::CALL => {
                        let label_text = match &current_line.items[1] {
                            LineItem::Label(label) | LineItem::String(label) => label, // Rust is cool!
                            _ => panic!("Error at line {}: Cannot call function with a non-label and non-string qualifier.", current_line.line_number),
                        };
                        let jump_target = match self.labels.get(label_text) {
                            Some(l) => l,
                            None => panic!("Error at line {}: Function not known.", current_line.line_number),
                        };
                        match self.call_function(&code[jump_target.line_number], current_line, file, jump_target.line_number, current_line_number) {
                            Ok(l) => current_line_number = l.line_number,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };
                    },
                    Command::RETURN => {
                        current_line_number = match self.return_from_function(current_line, current_line_number) {
                            Ok(l) => l.line_number,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        }
                    },
                    Command::GOTO => {
                        let label_text = match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line.line_number),
                        };

                        current_line_number = match self.labels.get_line_number_with_inferred(label_text, &current_line_number) {
                            Some(n) if n < code.len() => n,
                            _ => panic!("Error at line {}: Label not known.", current_line_number),
                        };
                    }
                    Command::IF => {
                        let cond = match self.line_item_as_value(&current_line.items[1]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };

                        let which_label = if cond.as_boolean() {2usize} else {3usize};
                        let label_text = match &current_line.items[which_label] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line.line_number),
                        };

                        current_line_number = match self.labels.get_line_number_with_inferred(label_text, &current_line_number) {
                            Some(n) if n < code.len() => n,
                            _ => panic!("Error at line {}: Label not known.", current_line_number),
                        };
                    }
                    Command::FOR => {
                        let end_value = match self.line_item_as_value(&current_line.items[4]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };

                        let label_end_text = match &current_line.items[2] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line.line_number),
                        };

                        let label_end_line_number = match self.labels.get_line_number_with_inferred(label_end_text, &current_line_number) {
                            Some(n) if n < code.len() => n,
                            _ => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        let variable_name = match &current_line.items[3] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Loop variable invalid.", current_line.line_number),
                        };

                        let value = match self.read_variable(variable_name) {
                            Some(v) => v,
                            None => panic!("Error at line {}: For loop variable not found.", current_line.line_number),
                        };

                        let current = match value.value_type {
                            ValueType::Number => value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line.line_number),
                        };

                        let compare = match end_value.value_type {
                            ValueType::Number => end_value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line.line_number),
                        };

                        if current < compare {
                            current_line_number += 1;
                        } else {
                            current_line_number = label_end_line_number;
                        }
                    }
                    Command::CONTINUE => {
                        let label_text = match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line.line_number),
                        };

                        let label_line_number = match self.labels.get_line_number_with_inferred(label_text, &current_line_number) {
                            Some(n) if n < code.len() => n,
                            _ => panic!("Error at line {}: Label not known.", current_line_number),
                        };

                        current_line_number = label_line_number;

                        let current_line = &code[current_line_number];

                        if !matches!(current_line.items[0], LineItem::Command(Command::FOR)) {
                            panic!("Error at line {}: Can only continue to a for line.", label_line_number);
                        }

                        let variable_name = match &current_line.items[3] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot go to non-label.", current_line.line_number),
                        };

                        let value = match self.read_variable(variable_name) {
                            Some(v) => v,
                            None => panic!("Error at line {}: For loop variable not found.", label_line_number),
                        };

                        let new_value = match value.value_type {
                            ValueType::Number => value.number_value,
                            _ => panic!("Error at line {}: Can only iterate with numbers.", current_line.line_number),
                        }.round() as i32 + 1;

                        self.assign_variable(variable_name, Value::number_value(new_value as f64));
                    }
                    Command::SUM => {
                        let operand = match &current_line.items[2] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Operand must be a label.", current_line.line_number),
                        };
                        let v1 = match self.line_item_as_value(&current_line.items[1]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };
                        let v2 = match self.line_item_as_value(&current_line.items[3]) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };
                        let sum = match VM::do_sum(&v1, &v2, &operand) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };

                        self.assign_variable(&"@save".to_string(), sum);
                        current_line_number += 1;
                    }
                    Command::SAY => {
                        let str = self.line_item_to_string(&current_line.items[1], &current_line.line_number);
                        self.say(str);
                        current_line_number += 1;
                    },
                    Command::ASK => {
                        // flush the output also so the user is not confused.
                        let str = self.line_item_to_string(&current_line.items[1], &current_line.line_number);
                        self.say(str);
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
                            slice_offset: None,
                        });

                        current_line_number += 1;
                    }
                    Command::SET => {
                        let value = match self.line_item_as_value(&current_line.items[2]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };

                        self.assign_variable(match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot assign to such variable.", current_line.line_number),
                        }, value);

                        current_line_number += 1;
                    }
                    Command::TABLE => {
                        let dimensions = current_line.items[2..].iter().map(
                            |item| match self.line_item_as_value(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line.line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line.line_number, e),
                            }
                        ).collect::<Vec<usize>>();

                        self.create_table(match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot assign to such variable.", current_line.line_number),
                        }, &dimensions);
                        current_line_number += 1;
                    },
                    Command::GET => {
                        let variable_name = match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot read such variable.", current_line.line_number),
                        };
                        let table = match self.read_variable(variable_name) {
                            Some(v) => match v.value_type {
                                ValueType::Table => v,
                                _ => panic!("Error at line {}: Variable is not a table.", current_line.line_number),
                            },
                            None => panic!("Error at line {}: Variable {} not found.", current_line.line_number, variable_name),
                        };
                        let dimensions = current_line.items[2..].iter().map(
                            |item| match self.line_item_as_value_pure(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line.line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line.line_number, e),
                            }
                        ).collect::<Vec<usize>>();
                        let value = match self.table_manager.get(table, dimensions) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: Error reading from table. {}", current_line.line_number, e),
                        }.clone();
                        self.assign_variable(&"@save".to_string(), value);
                        current_line_number += 1;
                    }
                    Command::PUT => {
                        let variable_name = match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot read such variable.", current_line.line_number),
                        };
                        let table = match self.read_variable(variable_name) {
                            Some(v) => {match v.value_type {
                                ValueType::Table => v,
                                _ => panic!("Error at line {}: Variable is not a table.", current_line.line_number),
                            }},
                            None => panic!("Error at line {}: Variable {} not found.", current_line.line_number, variable_name),
                        }.to_owned();
                        let table_dimensions = match self.table_manager.get_dimensions(&table) {
                            Ok(v) => v,
                            Err(e) => panic!("Error at line {}: Error reading from table. {}", current_line.line_number, e)
                        };
                        let dimensions = current_line.items[2..(2 + table_dimensions.len())].iter().map(
                            |item| match self.line_item_as_value_pure(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line.line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line.line_number, e),
                            }
                        ).collect::<Vec<usize>>();
                        let value = match self.line_item_as_value(&current_line.items[2 + table_dimensions.len()]) {
                            Ok(v) => v.clone(),
                            Err(e) => panic!("Error at line {}: {}", current_line.line_number, e),
                        };
                        match self.table_manager.put(&table, dimensions, value) {
                            Ok(()) => (),
                            Err(e) => panic!("Error at line {}: Error writing to table. {}", current_line.line_number, e),
                        };
                        current_line_number += 1;
                    },
                    Command::SLICE => {
                        let variable_name = match &current_line.items[1] {
                            LineItem::Label(label) => label,
                            _ => panic!("Error at line {}: Cannot read such variable.", current_line.line_number),
                        };

                        let table = match self.read_variable(variable_name) {
                            Some(v) => match v.value_type {
                                ValueType::Table => v,
                                _ => panic!("Error at line {}: Variable is not a table.", current_line.line_number),
                            },
                            None => panic!("Error at line {}: Variable {} not found.", current_line.line_number, variable_name),
                        };

                        let n_dimensions = match table.value_type {
                            ValueType::Table if !table.is_uninitialized() => table.slice_offset.as_ref().unwrap().len(),
                            _ => panic!("Error at line {}: Value is not a table.", current_line.line_number),
                        };

                        let offset_to_add = current_line.items[2..2 + n_dimensions].iter().map(
                            |item| match self.line_item_as_value_pure(item) {
                                Ok(value) => match value.value_type {
                                    ValueType::Number => value.number_value.round() as usize,
                                    _ => panic!("Error at line {}: Invalid dimension. Each dimension must be a number.", current_line.line_number)
                                },
                                Err(e) => panic!("Error at line {}: Invalid dimension. {}", current_line.line_number, e),
                            }
                        ).collect::<Vec<usize>>();

                        let new_value = match self.slice_table(table, &offset_to_add) {
                            Ok(v) => v,
                            Err(err) => panic!("Error at line {}: {}", current_line.line_number, err),
                        };

                        self.assign_variable(&"@save".to_string(), new_value);

                        current_line_number += 1;
                    }
                    Command::NOOP => {
                        current_line_number += 1;
                    },
                },
                _ => panic!("Line parsing error: The first item in the line is supposed to be a command."),
            }
        }
    }

    pub fn line_item_as_value(&mut self, item: &LineItem) -> Result<Value, String> {
        match item {
            
            LineItem::Table(dimensions, data) => {
                let value = self.table_manager.create_table(dimensions);
                let values_to_write: Vec<_> = data.iter().map(|it| self.line_item_as_value(it).unwrap()).collect();
                self.table_manager.write_raw(&value, &values_to_write)?;
                Ok(value)
            },
            LineItem::Array(_) => panic!("Arrays statements are not supported"),
            _ => self.line_item_as_value_pure(item),
        }
    }

    pub fn line_item_as_value_pure(&self, item: &LineItem) -> Result<Value, String> {
        match item {
            LineItem::Number(number_value) => Ok(Value {
                value_type: super::value::ValueType::Number,
                string_value: None,
                table_index: usize::MAX,
                number_value: number_value.clone(),
                slice_offset: None,
            }),
            LineItem::String(string_value) => Ok(Value {
                value_type: super::value::ValueType::String,
                string_value: Some(string_value.clone()),
                table_index: usize::MAX,
                number_value: 0f64,
                slice_offset: None,
            }),
            LineItem::Label(name) => match self.read_variable(name) {
                Some(v) => Ok(v.clone()),
                None => Err(format!("Variable {} not found.", name)),
            },
            LineItem::Command(_) => panic!("Unexpected argument of type command."),
            _ => panic!("Attempt to convert non-const statement to a value."),
        }
    }

    pub fn line_item_to_string(&mut self, item: &LineItem, current_line_number: &usize) -> String {
        match item {
            LineItem::Number(number_value) => number_value.to_string(),
            LineItem::String(string_value) => string_value.to_owned(),
            LineItem::Label(_) => {
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
            LineItem::Command(_) => panic!("Unexpected argument of type command."),
            _ => "".to_string(),
        }
    }

    pub fn add_label(&mut self, name: String, file: String, line_number: usize) {
        self.labels.insert(name, Label {file, line_number});
    }

    pub fn call_function(&mut self, function_line: &Line, call_line: &Line, file: &String, fun: usize, ret: usize) -> Result<Label, &str> {
        // The number 2 is because both the call and function commands begin with two items: command name and function name
        let num_arguments = function_line.items.len() - 2;
        if num_arguments > call_line.items.len() - 2 {
            return Err("Not enough arguments are supplied.")
        }
        let new_env = Environment::that_returns_to(Some(Label { file: file.clone(), line_number: ret + 1}));
        self.environment.push(new_env);

        let mut i = 2;
        while i < num_arguments + 2 {
            let value = match self.line_item_as_value(&call_line.items[i]) {
                Ok(v) => v,
                Err(e) => panic!("Error at line {}: {}", ret, e),
            };
            self.assign_variable(match &function_line.items[i] {
                LineItem::Label(label) => label,
                _ => panic!("Error at line {}: Function arguments must be variables.", function_line.line_number),
            }, value);
            i += 1;
        }
        
        Ok(Label { file: file.clone(), line_number: fun })
    }

    pub fn return_from_function(&mut self, return_line: &Line, line_number: usize) -> Result<Label, &str> {
        if self.environment.len() <= 1 {
            return Err("Cannot return from root.");
        }

        let return_value = if return_line.items.len() > 1 {
            match self.line_item_as_value(&return_line.items[1]) {
                Ok(v) => v,
                Err(e) => panic!("Error at line {}: {}", line_number, e),
            }
        } else {
            Value::uninitialized()
        };

        let ret = self.environment.pop().unwrap().return_address.unwrap();

        if return_line.items.len() > 1 {
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

    pub fn slice_table(&self, table_value: &Value, offset: &Vec<usize>) -> Result<Value, &'static str> {
        let table_dimensions = self.table_manager.get_dimensions(&table_value)?;

        let new_offset: Vec<usize> = table_value.slice_offset.as_ref().unwrap().iter()
            // Add the new offset
            .zip(offset).map(|(a, b)| a + b)
            // Check if the new offset is below capacity
            .zip(table_dimensions)
                .map(|(len, cap)| if &len < cap { Ok(len) } else {Err("Slice start too large.")})
            .collect::<Result<_, _>>()?;
        
        Ok(Value {
            value_type: ValueType::Table,
            string_value: None,
            number_value: 0f64,
            table_index: table_value.table_index,
            slice_offset: Some(new_offset),
        })
    }

    pub fn collect_garbage(&mut self) {
        self.table_manager.mark(&self.environment);
        self.table_manager.sweep();
    }
}