use super::value::{StructDef, Value};
use std::collections::HashMap;
use std::rc::Rc;

pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Rc<Environment>>,
    pub struct_defs: HashMap<String, StructDef>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            parent: self.parent.clone(),
            struct_defs: self.struct_defs.clone(),
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
            struct_defs: HashMap::new(),
        }
    }
    pub fn with_parent(parent: Environment) -> Self {
        Self {
            values: HashMap::new(),
            struct_defs: parent.struct_defs.clone(),
            parent: Some(Rc::new(parent)),
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        if let Some(v) = self.values.get(name) {
            Some(v)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub fn assign(&mut self, name: &str, value: Value) -> Result<(), String> {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            Ok(())
        } else {
            Err(format!("Undefined variable '{}'", name))
        }
    }

    pub fn register_struct(&mut self, def: StructDef) {
        self.struct_defs.insert(def.name.clone(), def);
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructDef> {
        self.struct_defs.get(name)
    }
}
