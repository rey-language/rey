use super::value::{StructDef, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

pub struct Environment {
    values: HashMap<String, Value>,
    parent: Option<Rc<Environment>>,
    pub struct_defs: HashMap<String, StructDef>,
    pub enum_defs: HashMap<String, Vec<String>>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            parent: self.parent.clone(),
            struct_defs: self.struct_defs.clone(),
            enum_defs: self.enum_defs.clone(),
        }
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: HashMap::new(),
            parent: None,
            struct_defs: HashMap::new(),
            enum_defs: HashMap::new(),
        }
    }
    pub fn with_parent(parent: Environment) -> Self {
        Self {
            values: HashMap::new(),
            struct_defs: HashMap::new(),  // look up via parent chain, avoid cloning AST
            enum_defs: HashMap::new(),
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

    // mutable access to a local-scope variable (parent scope is immutable via Rc)
    pub fn get_mut_local(&mut self, name: &str) -> Option<&mut Value> {
        self.values.get_mut(name)
    }

    // take ownership of a local-scope value (removes from env); caller must put it back
    pub fn take_local(&mut self, name: &str) -> Option<Value> {
        self.values.remove(name)
    }

    // restore a previously taken local value
    pub fn restore_local(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
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

    pub fn register_enum(&mut self, name: String, variants: Vec<String>) {
        self.enum_defs.insert(name.clone(), variants.clone());
        // Also register each variant as a value, and expose a namespace dict under the enum name
        let mut namespace = HashMap::new();
        for variant in variants {
            let val = Value::EnumVariant {
                enum_name: name.clone(),
                variant: variant.clone(),
            };
            namespace.insert(variant.clone(), val.clone());
            self.values.insert(variant, val);
        }
        self.values.insert(
            name,
            Value::Dict(Rc::new(RefCell::new(namespace))),
        );
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructDef> {
        if let Some(def) = self.struct_defs.get(name) {
            Some(def)
        } else if let Some(parent) = &self.parent {
            parent.get_struct(name)
        } else {
            None
        }
    }
}
