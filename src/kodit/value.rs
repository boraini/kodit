use super::table::Table;

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