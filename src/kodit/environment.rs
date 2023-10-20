use std::collections::HashMap;

use super::value::Value;

pub struct Environment {
    pub return_address: Option<Label>,
    pub variables: HashMap<String, Value>,
}

pub struct Label {
    pub file: String,
    pub line_number: usize,
}

impl Environment {
    pub fn that_returns_to(ret : Option<Label>) -> Environment {
        Environment {
            return_address: ret,
            variables: HashMap::new()
        }
    }
}