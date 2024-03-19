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

// Constructor
impl TableManager {
    pub fn new() -> TableManager {
        TableManager {
            table_id: 0,
            tables: HashMap::new(),
        }
    }
}

// Read/Write
impl TableManager {
    fn calculate_data_index(table: &Table, table_value: &Value, mut dimensions: Vec<usize>) -> Result<usize, &'static str> {
        if table.dimensions.len() != dimensions.len() {
            return Err("Dimension numbers don't match.")
        }

        dimensions.iter_mut()
            .zip(table_value.slice_offset.as_ref().ok_or("Value is not a table.")?)
            .for_each(|(a, b)| *a += b);

        let check_each_dimension = dimensions.iter()
            .zip(table.dimensions.iter())
            .all(|(a, b)| a >= &0 && a < b);
        
        if !check_each_dimension {
            print!("Index out of bounds for indices {:?} out of {:?}.", dimensions, table.dimensions);
            return Err("Index out of bounds. See above.");
        }

        let number = dimensions.iter()
            .zip(table.dimensions.iter())
            .fold(0usize, |acc, (idx, dim)| dim * acc + idx);

        Ok(number)
    }

    pub fn get_dimensions(&self, table_value: &Value) -> Result<&Vec<usize>, &'static str> {
        let table = match self.tables.get(&table_value.table_index) {
            Some(t) => t,
            None => return Err("Table not found by index.")
        };

        Ok(&table.dimensions)
    }

    pub fn get(&self, table_value: &Value, dimensions: Vec<usize>) -> Result<&Value, &'static str> {
        let table = match self.tables.get(&table_value.table_index) {
            Some(t) => t,
            None => return Err("Table not found by index.")
        };

        let index = match Self::calculate_data_index(table, table_value, dimensions) {
            Ok(idx) => idx,
            Err(e) => return Err(e),
        };

        let candidate_value = table.data.get(index).unwrap();

        if candidate_value.is_uninitialized() {
            Err("Field is not initialized.")
        } else {
            Ok(candidate_value)
        }
    }

    pub fn put(&mut self, table_value: &Value, dimensions: Vec<usize>, value: Value) -> Result<(), &'static str> {
        let table = match self.tables.get_mut(&table_value.table_index) {
            Some(t) => t,
            None => return Err("Table not found by index.")
        };

        let index = match Self::calculate_data_index(table, table_value, dimensions) {
            Ok(idx) => idx,
            Err(e) => return Err(e),
        };

        table.data[index] = value.clone();
        Ok(())
    }

    pub fn write_raw(&mut self, table_value: &Value, data: &Vec<Value>) -> Result<(), &'static str> {
        let table = match self.tables.get_mut(&table_value.table_index) {
            Some(t) => t,
            None => return Err("Table not found by index.")
        };

        let max_capacity: usize = table.dimensions.iter().product();

        if data.len() > max_capacity {
            panic!("Data to fill in is too much.");
        }

        table.data[0..data.len()].clone_from_slice(data);
        Ok(())
    }
}

// Garbage Collection
impl TableManager {
    pub fn create_table(&mut self, dimensions: &Vec<usize>) -> Value {
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
            slice_offset: Some(vec![0usize; dimensions.len()]),
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