use std::collections::{HashMap, HashSet};

use super::{value::{Value, ValueType}, environment::Environment};

pub struct Table {
    dimensions: Vec<usize>,
    data: Box<[Value]>,
    is_marked: bool,
    is_handled: bool,
}

pub struct TableManager {
    table_id: usize,
    tables: HashMap<usize, Table>,
}

impl TableManager {
    pub fn new() -> TableManager {
        TableManager {
            table_id: 0,
            tables: HashMap::new(),
        }
    }

    pub fn create_table(&mut self, dimensions: &Vec<usize>) -> Value {
        const UNINITIALIZED: Option<Value> = None;
        let length: usize = dimensions.iter().product();

        let table = Table {
            dimensions: dimensions.clone(),
            data: vec![Value::uninitialized(); length].into_boxed_slice(),
            is_marked: false,
            is_handled: false,
        };

        if self.table_id == usize::MAX {
            //panic!("Runtime Error: Maximum number of tables have been created!");
            self.table_id = 0;
        }

        while self.tables.contains_key(&self.table_id) {
            self.table_id += 1;
        }

        let id = self.table_id;
        self.table_id += 1;

        self.tables.insert(id, table);

        Value {
            table_index: id,
            value_type: ValueType::Table,
            string_value: None,
            number_value: 0f64,
        }
    }

    pub fn mark(&mut self, environments: &Vec<Environment>) {
        // the queue represents the grey set if you are reading Wikipedia.
        // it doesn't run like a queue but it doesn't matter.
        let mut queue: HashSet<usize> = HashSet::new();
        
        // clear the marking on all tables
        for (_, table) in &mut self.tables {
            table.is_marked = false;
        }

        // add root pointers to the queue
        for env in environments {
            for (_, value) in &env.variables {
                if !value.is_uninitialized() && matches!(value.value_type, ValueType::Table) {
                    queue.insert(value.table_index);
                }
            }
        }

        while !queue.is_empty() {
            let next_index = queue.iter().next().unwrap().to_owned();
            queue.remove(&next_index);

            let next_table = self.tables.get_mut(&next_index).unwrap();

            next_table.is_marked = true;

            let next_table = self.tables.get(&next_index).unwrap();

            for value in next_table.data.iter() {
                if !value.is_uninitialized() && matches!(value.value_type, ValueType::Table) {
                    let next_table = self.tables.get(&value.table_index).unwrap();
                    if !next_table.is_handled {
                        queue.insert(value.table_index);
                    }
                }
            }
        }
    }

    pub fn sweep(&mut self) {
        self.tables.retain(|_, v| {
            v.is_marked
        });
    }
}