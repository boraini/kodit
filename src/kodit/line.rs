use snailquote;

#[derive(Debug)]
pub struct Line {
    pub line_number: usize,
    pub items: Vec<LineItem>,
}

impl Line {
    pub fn new(line_number: usize) -> Self {
        Self {
            line_number,
            items: vec!(),
        }
    }
}

#[derive(Clone, Debug)]
pub enum LineItem {
    Command(Command),
    Label(String),
    String(String),
    Number(f64),
    Array(Vec<LineItem>),
    Table(Vec<usize>, Vec<LineItem>),
}

#[derive(Clone, Debug)]
pub enum Command {
    LABEL,
    FUNCTION,
    CALL,
    RETURN,
    GOTO,
    IF,
    FOR,
    CONTINUE,
    SUM,
    SAY,
    ASK,
    SET,
    TABLE,
    GET,
    PUT,
    SLICE,
    NOOP,
}

impl Command {
    pub fn from_string(name: &str) -> Command {
        match name.to_lowercase().as_str() {
            "label" => Command::LABEL,
            "function" => Command::FUNCTION,
            "call" => Command::CALL,
            "return" => Command::RETURN,
            "goto" => Command::GOTO,
            "if" => Command::IF,
            "for" => Command::FOR,
            "continue" => Command::CONTINUE,
            "sum" => Command::SUM,
            "say" => Command::SAY,
            "ask" => Command::ASK,
            "set" => Command::SET,
            "table" => Command::TABLE,
            "get" => Command::GET,
            "put" => Command::PUT,
            "slice" => Command::SLICE,
            _ => Command::NOOP,
        }
    }
}

struct ParserState {
    pub current_table_depth: usize,
    pub current_line: Line,
}

static SYMBOLS: [&'static str; 10] = ["+", "==", "-", "*", "/", "%", "<", ">", "<=", ">="];

pub fn decompose_lines(lines: &[String]) -> Result<Vec<Line>, String> {
    let mut parser_state = ParserState {
        current_table_depth: 0,
        current_line: Line::new(1),
    };

    let mut produced_lines = vec!();

    for i in 0..lines.len() {
        decompose_line(&lines[i], &mut parser_state);
        if parser_state.current_table_depth == 0 {
            let raw_items = &parser_state.current_line.items;
            if raw_items.is_empty() { continue; }
            let items = arrays_to_tables(raw_items)?;
            produced_lines.push(Line {line_number: parser_state.current_line.line_number, items });
            parser_state.current_line = Line::new(i + 2);
        }
    }

    Ok(produced_lines)
}

/// Decompose line into a list of line items. This corresponds to s-expressions.
fn decompose_line(line: &String, parser_state: &mut ParserState) {
    let mut current_line = &line[..];

    loop {
        let startspace = current_line.find(|c| !char::is_whitespace(c));

        let start = match startspace {
            None => return,
            Some(idx) => idx,
        };

        let endspace = current_line[start..].find(char::is_whitespace);

        let end = match endspace {
            None => current_line.len(),
            Some(idx) => idx + start,
        };

        if start >= end {
            break;
        }

        // Comment
        if end - start >= 2 && &line[start..start + 2] == "//" {
            break;
        }

        let mut line_items = {
            let mut current_depth = parser_state.current_table_depth;
            let mut current_line_items = Some(&mut parser_state.current_line.items);

            while current_depth > 0 {
                // Algorithm based on https://rust-unofficial.github.io/too-many-lists/first-pop.html
                match std::mem::take(&mut current_line_items) {
                    Some(current_items) => {
                        let last_index = current_items.len() - 1;
                        match current_items.get_mut(last_index) {
                            Some(item) => match item {
                                LineItem::Array(next_items) => {
                                    current_line_items = Some(next_items);
                                },
                                _ => panic!("Internal error with table depth traversal on line {}. The current array does not end with a table.", parser_state.current_line.line_number),
                            },
                            None => panic!("Internal error with table depth traversal on line {}. The current array is empty.", parser_state.current_line.line_number),
                        };
                    },
                    None => panic!("Internal error with table depth traversal on line {}. There is no current array.", parser_state.current_line.line_number),
                };
                current_depth -= 1;
            }

            current_line_items
        };

        if parser_state.current_table_depth == 0 && line_items.as_ref().unwrap().is_empty() {
            let command = line[start..end].to_owned();

            line_items.unwrap().push(LineItem::Command(Command::from_string(command.as_str())));

            current_line = &current_line[end..];
        } else {
            let mut skip_start = 0;
            let mut skip_end = 0;

            while start + skip_start < end && current_line.chars().nth(start + skip_start) == Some('[') {
                match std::mem::take(&mut line_items) {
                    Some(items) => {
                        items.push(LineItem::Array(vec!()));

                        let last_index = (&items).len() - 1;
                        let new_line_items  = match items.get_mut(last_index).unwrap() {
                            LineItem::Array(xs) => xs,
                            _ => panic!("Unexpected error in parser."),
                        };

                        line_items.replace(new_line_items);
                    },
                    None => panic!("Unexpected error in parser."),
                }
                parser_state.current_table_depth += 1;
                skip_start += 1;
            }
            
            while  end > skip_end && current_line.chars().nth(end - skip_end - 1) == Some(']') {
                parser_state.current_table_depth -= 1;
                skip_end += 1;
            }

            // this item was only the square bracket
            if (start + skip_start) >= (end - skip_end) {
                current_line = &current_line[end..];
                continue;
            }

            if current_line.chars().nth(start + skip_start) == Some('"') {
                // We redefine the variable for the block so we can reuse old code.
                let start = start + skip_start;
                let mut closed = false;
                let mut position = 0;
                let mut skip = 1;

                while !closed {
                    let string_end_op = current_line[start + skip..].find(|c| c == '"');

                    let string_end = match string_end_op {
                        Some(idx) => start + 1 + idx,
                        None => panic!("String is not closed"),
                    };

                    if current_line.chars().nth(string_end - 1) != Some('\\') {
                        position = string_end;
                        closed = true;
                    } else {
                        skip = string_end;
                    }
                }

                let the_string = snailquote::unescape(&current_line[start..position + 1]);

                line_items.unwrap().push(LineItem::String(the_string.unwrap()));

                current_line = &current_line[position + 1 + skip_end..];
            // number or label
            } else {
                let the_string = &current_line[start + skip_start..end - skip_end];

                let line_item: LineItem = if SYMBOLS.contains(&the_string) {
                    LineItem::Label(the_string.to_string())
                } else {
                    match the_string.parse() {
                        Ok(n) => LineItem::Number(n),
                        Err(_) => LineItem::Label(the_string.to_string()),
                    }
                };

                line_items.unwrap().push(line_item);

                current_line = &current_line[end..];
            }
        }
    }
}

pub fn arrays_to_tables(line_items: &Vec<LineItem>) -> Result<Vec<LineItem>, String> {
    let mut result: Vec<LineItem> = vec!();
    let mut i = 0;

    while i < line_items.len() {
        result.push(match line_items.get(i).unwrap() {
            LineItem::Array(dims) => {
                let dimensions: Vec<usize> = dims.iter().map(|v| match v {
                    LineItem::Number(x) => Ok(x.round() as usize),
                    _ => return Err("Table dimensions must be constant number literals.".to_string()),
                }).collect::<Result<_, String>>()?;

                i += 1;

                let data: Vec<LineItem> = match &line_items.get(i).ok_or("Arrays come in pairs of dimension and data next to each other.")? {
                    LineItem::Array(array_items) => arrays_to_tables(array_items)?,
                    _ => return Err("Arrays come in pairs of dimension and data next to each other.".to_string()),
                };
                
                LineItem::Table(dimensions, data)
            }
            _ => line_items[i].clone()
        });
        i += 1;
    }

    Ok(result)
}
