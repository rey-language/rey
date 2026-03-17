use std::collections::HashMap;
use std::rc::Rc;
use super::value::Value;

pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Rc<Environment>>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            parent: self.parent.clone(),
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
        }
    }
    pub fn with_parent(parent: Environment) -> Self {
        Self {
            values: HashMap::new(),
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


}