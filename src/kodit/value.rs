use std::fmt::{Display, Error};

#[derive(Clone)]
pub enum ValueType {
    String,
    Number,
    Table,
}

pub struct Value {
    pub value_type: ValueType,
    pub string_value: Option<String>,
    pub number_value: f64,
    pub table_index: usize,
}

impl Value {
    pub fn uninitialized() -> Value {
        Value {
            value_type: ValueType::Table,
            string_value: None,
            number_value: 0f64,
            table_index: usize::MAX,
        }
    }

    pub fn number_value(n: f64) -> Value {
        Value {
            value_type: ValueType::Number,
            string_value: None,
            number_value: n,
            table_index: 0,
        }
    }

    pub fn string_value(string: &String) -> Value {
        // TODO: Remove the copying
        Value {
            value_type: ValueType::String,
            string_value: Some(string.to_owned()),
            number_value: 0f64,
            table_index: 0,
        }
    }

    pub fn boolean_value(b: bool) -> Value {
        Value::number_value(if b {1.0} else {0.0})
    }

    pub fn as_boolean(&self) -> bool {
        !matches!(self.value_type, ValueType::Number) || self.number_value != 0.0
    }

    pub fn is_uninitialized(&self) -> bool {
        self.table_index == usize::MAX && matches!(self.value_type, ValueType::Table)
    }
}

impl Clone for Value {
    fn clone(&self) -> Self {
        Self { 
            value_type: self.value_type.clone(),
            string_value: self.string_value.clone(),
            number_value: self.number_value.clone(),
            table_index: self.table_index.clone(),
        }
    }
}

impl Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.value_type {
            ValueType::Number => write!(f, "<Number Value {}>", self.number_value),
            ValueType::String => write!(f, "<String Value {}>", match self.string_value.as_ref() {
                Some(s) => s,
                None => return Err(Error),
            }),
            ValueType::Table => write!(f, "<Table Value>"),
        }
    }
}