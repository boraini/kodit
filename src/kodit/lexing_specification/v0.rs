use std::num::TryFromIntError;

use yaml_rust::{yaml::Yaml, YamlLoader};

use crate::kodit::line::{Line, LineItem, Command};
use super::{LexingSpecification, Matcher};

pub struct LexingSpecificationV0 {
    pub name: String,
    pub version: i32,
    pub matchings: Vec<(Command, Vec<Matcher>)>,
}

pub struct Matching {
    arguments : Vec<(i32, LineItem)>,
}

impl Matching {
    pub fn of_rest(leftover : &[LineItem], max_argument_index : i32) -> Matching {
        Matching {
            arguments : ((max_argument_index + 1)..).zip(leftover).map(|(a, b)| (a, b.to_owned())).collect(),
        }
    }
}

impl LexingSpecification for LexingSpecificationV0 {
    fn lex(&self, line : &Line) -> Option<Line> {
        for (command, matchers) in &self.matchings {
            match parse(&line.items, matchers, 0) {
                Some(mut matching) => {
                    matching.arguments.sort_by(|m, n| m.0.cmp(&n.0));
                    let mut line = Line::new(line.line_number);
                    line.items.push(LineItem::Command(command.to_owned()));

                    for (_, item) in matching.arguments {
                        line.items.push(item);
                    }
                    return Some(line);
                },
                None => continue,
            }
        }

        return None;
    }
}

impl LexingSpecificationV0 {
    fn convert_yaml_matchers(file_name : &str, spec : &Yaml) -> Result<Vec<(Command, Vec<Matcher>)>, String> {
        let commands = spec.as_hash().ok_or_else(|| format!("Command descriptions must come as an object in v0 specifications like {}", file_name))?;

        let mut result = vec!();

        for (command_name, matcher) in commands {
            let command = match Command::from_string(command_name.as_str().ok_or_else(|| format!("Command names must be string in {}", file_name))?) {
                Command::NOOP => return Err(format!("Command name {} either is NOOP or unknown in {}", command_name.as_str().unwrap(), file_name)),
                command => command,
            };

            let command_name = command_name.as_str().unwrap();

            let matcher_spec_array = matcher
              .as_hash().ok_or_else(|| format!("Command matcher specification for {} should be objects in {}", command_name, file_name))?
              .get(&Yaml::String("format".to_string())).ok_or_else(|| format!("Command matcher specification for {} should contain a format argument in {}", command_name, file_name))?
              .as_vec().ok_or_else(|| format!("Command matcher format in {} should be an array in {}", command_name, file_name))?;

            let matchers = matcher_spec_array.iter().map(|val| match val {
                Yaml::Integer(i) => {
                    let val = i.to_owned().try_into().map_err(|e : TryFromIntError| e.to_string())?;
                    Ok(Matcher::Argument(val))
                }
                Yaml::String(str) => if str == "..." { Ok(Matcher::Rest) } else { Ok(Matcher::Symbol(str.to_owned())) },
                _ => Err(format!("Unknown type in format array of {} in {}", command_name, file_name)),
            }).collect::<Result<Vec<Matcher>, String>>()?;

            result.push((command, matchers))
        }
        return Ok(result);
    }
    pub fn from_file(file_name : &str) -> Result<LexingSpecificationV0, String> {
        let buf = match std::fs::read_to_string(file_name) {
            Ok(str) => str,
            Err(str) => {
                return Err(format!("Error while loading lexing specification {}: {:?}.", file_name, str))
            },
        };

        let yaml = match YamlLoader::load_from_str(&buf) {
            Ok(vec) => vec,
            Err(str) => {
                return Err(format!("Error while loading lexing specification {}: {:?}.", file_name, str))
            },
        };

        if yaml.is_empty() {
            panic!("Lexing specification {} is empty!", file_name);
        }

        if yaml.len() > 1 {
            println!("Warning: Lexing specification {} has multiple objects at its root.", file_name);
        }

        let mut matchers = None;
        let mut name = None;

        for root_entry in &yaml {
            for (key, value) in root_entry.as_hash().unwrap() {
                match key {
                    Yaml::String(str) => match str.as_str() {
                        "name" => match value {
                            Yaml::String(str) => { let _ = name.insert(str); }
                            _ => return Err("Name must be string if provided".to_string()),
                        },
                        "commands" => { let _ = matchers.insert(LexingSpecificationV0::convert_yaml_matchers(file_name, value)?); }
                        "version" => continue,
                        _ => println!("Warning: key {} in {} is unknown. Skipping.", str, file_name),
                    },
                    val => println!("Warning: key {:?} in {} is unknown. Skipping.", val, file_name),
                }
            }
        }

        Ok(LexingSpecificationV0 {
            name: match name {
                Some(name) => name.to_owned(),
                None => file_name.to_string(),
            },
            version: 0,
            matchings: matchers.ok_or_else(|| format!("No command matchers were found in {}.", file_name))?,
        })
    }
}

fn parse<'a>(leftover : &'a [LineItem], spec : &'a [Matcher], max_argument_index : i32) -> Option<Matching> {
    if spec.is_empty() {
        return Some(Matching::of_rest(leftover, max_argument_index));
    } else if leftover.is_empty() {
        if matches!(spec[0], Matcher::Rest) {
            return Some(Matching::of_rest(&[], max_argument_index))
        } else {
            return None
        }
    } else {
        let new_item = match (&spec[0], &leftover[0]) {
            (Matcher::Symbol(sym), item) => {
                match item {
                    LineItem::Label(my_sym) if my_sym == sym => None,
                    _ => return None,
                }
            },
            (Matcher::Argument(idx), item) => Some((idx, item)),
            (Matcher::Rest, _) => return Some(Matching::of_rest(leftover, max_argument_index)),
        };

        let new_max_argument_index = match new_item {
            Some((i, _)) if max_argument_index < *i => i,
            _ => &max_argument_index,
        };

        let mut parsed_rest = parse(&leftover[1..], &spec[1..], *new_max_argument_index);

        match &mut parsed_rest {
            Some(matching) => {
                match new_item {
                    Some(item) => {
                        matching.arguments.push((item.0.to_owned(), item.1.to_owned()));
                        parsed_rest
                    }
                    _ => parsed_rest
                }
            }
            None => None,
        }
    }
}