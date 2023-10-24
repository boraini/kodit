use std::fmt::Display;
use snailquote;

pub enum LineItemType {
    COMMAND,
    LABEL,
    STRING,
    NUMBER,
}

pub struct LineItem {
    pub item_type: LineItemType,
    pub string_value: String,
    pub number_value: f64,
}

/// Decompose line into a list of line items. This corresponds to s-expressions.
pub fn decompose_line(line: &String) -> Vec<LineItem> {
    let mut line_items: Vec<LineItem> = Vec::new();
    // let mut chars = line.chars();
    // let mut left_len = line.len();
    let mut current_line = &line[..];

    loop {
        let startspace = current_line.find(|c| !char::is_whitespace(c));

        let start = match startspace {
            None => return line_items,
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
        
        if line_items.is_empty() {
            let command = line[start..end].to_owned();

            line_items.push(LineItem { item_type: LineItemType::COMMAND, string_value: command, number_value: 0.0 });

            current_line = &current_line[end..];
        } else {
            if current_line.chars().nth(start) == Some('"') {
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

                let the_string = snailquote::unescape(&current_line[start + 1..position]);

                line_items.push(LineItem { item_type: LineItemType::STRING, string_value: the_string.unwrap(), number_value: 0.0 });

                current_line = &current_line[position + 1..];
            // number or label
            } else  {
                let the_string = &current_line[start..end];

                if match the_string.chars().next() {
                    Some(chr) => chr.is_numeric(),
                    None => panic!("Empty argument encountered."),
                } {
                    let number: f64 = the_string.parse().unwrap();

                    line_items.push(LineItem { item_type: LineItemType::NUMBER, string_value: the_string.to_string(), number_value: number });
                } else {
                    line_items.push(LineItem { item_type: LineItemType::LABEL, string_value: the_string.to_string(), number_value: 0.0 });
                }

                current_line = &current_line[end..];
            }
        }
    }

    line_items
}

impl Display for LineItem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Line Item {} {}", self.string_value, self.number_value)
    }
}