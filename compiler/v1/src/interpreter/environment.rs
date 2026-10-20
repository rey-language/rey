use super::value::{SharedStr, StructDef, Value};
use std::cell::RefCell;
use std::collections::HashMap;
use std::hash::{BuildHasherDefault, Hasher};
use std::rc::Rc;

#[derive(Default)]
struct FxHasher {
    hash: u64,
}

impl FxHasher {
    fn add(&mut self, value: u64) {
        self.hash = self.hash.rotate_left(5) ^ value;
        self.hash = self.hash.wrapping_mul(0x517cc1b727220a95);
    }
}

impl Hasher for FxHasher {
    fn write(&mut self, bytes: &[u8]) {
        let mut index = 0;
        while index + 8 <= bytes.len() {
            let mut buf = [0u8; 8];
            buf.copy_from_slice(&bytes[index..index + 8]);
            self.add(u64::from_ne_bytes(buf));
            index += 8;
        }
        if index + 4 <= bytes.len() {
            let mut buf = [0u8; 4];
            buf.copy_from_slice(&bytes[index..index + 4]);
            self.add(u32::from_ne_bytes(buf) as u64);
            index += 4;
        }
        while index < bytes.len() {
            self.add(bytes[index] as u64);
            index += 1;
        }
    }

    fn finish(&self) -> u64 {
        self.hash
    }
}

type FxBuild = BuildHasherDefault<FxHasher>;
type ValueMap = HashMap<String, Value, FxBuild>;
type StructMap = HashMap<String, StructDef>;
type EnumMap = HashMap<String, Vec<String>>;
type ContextMap = HashMap<String, Rc<Environment>>;

thread_local! {
    static VALUE_RC_POOL: RefCell<Vec<Rc<RefCell<ValueMap>>>> = const { RefCell::new(Vec::new()) };
    static EMPTY_STRUCTS: Rc<StructMap> = Rc::new(HashMap::new());
    static EMPTY_ENUMS: Rc<EnumMap> = Rc::new(HashMap::new());
    static EMPTY_CONTEXTS: Rc<ContextMap> = Rc::new(HashMap::new());
    static EMPTY_VALUES: Rc<RefCell<ValueMap>> = Rc::new(RefCell::new(HashMap::with_hasher(FxBuild::default())));
}

fn newValueMap(capacity: usize) -> ValueMap {
    HashMap::with_capacity_and_hasher(capacity, FxBuild::default())
}

fn emptyStructs() -> Rc<StructMap> {
    EMPTY_STRUCTS.with(Rc::clone)
}

fn emptyEnums() -> Rc<EnumMap> {
    EMPTY_ENUMS.with(Rc::clone)
}

fn emptyContexts() -> Rc<ContextMap> {
    EMPTY_CONTEXTS.with(Rc::clone)
}

fn emptyValues() -> Rc<RefCell<ValueMap>> {
    EMPTY_VALUES.with(Rc::clone)
}

fn takeValues() -> Rc<RefCell<ValueMap>> {
    VALUE_RC_POOL.with(|pool| {
        if let Some(values) = pool.borrow_mut().pop() {
            values
        } else {
            Rc::new(RefCell::new(newValueMap(8)))
        }
    })
}

fn recycleValues(values: Rc<RefCell<ValueMap>>) {
    if Rc::strong_count(&values) != 1 {
        return;
    }
    let capacity = match values.try_borrow_mut() {
        Ok(mut map) => {
            map.clear();
            map.capacity()
        }
        Err(_) => return,
    };
    if capacity > 8192 {
        return;
    }
    let _ = VALUE_RC_POOL.try_with(|pool| {
        let mut pool = pool.borrow_mut();
        if pool.len() < 256 {
            pool.push(values);
        }
    });
}

pub struct Environment {
    values: Rc<RefCell<ValueMap>>,
    parent: Option<Rc<Environment>>,
    frozen: bool,
    pub struct_defs: Rc<StructMap>,
    pub enum_defs: Rc<EnumMap>,
    struct_contexts: Rc<ContextMap>,
}

impl Clone for Environment {
    fn clone(&self) -> Self {
        Self {
            values: self.values.clone(),
            parent: self.parent.clone(),
            frozen: self.frozen,
            struct_defs: self.struct_defs.clone(),
            enum_defs: self.enum_defs.clone(),
            struct_contexts: self.struct_contexts.clone(),
        }
    }
}

impl Drop for Environment {
    fn drop(&mut self) {
        let values = std::mem::replace(&mut self.values, emptyValues());
        recycleValues(values);
    }
}

impl Environment {
    pub fn new() -> Self {
        Self {
            values: takeValues(),
            parent: None,
            frozen: false,
            struct_defs: Rc::new(HashMap::new()),
            enum_defs: Rc::new(HashMap::new()),
            struct_contexts: Rc::new(HashMap::new()),
        }
    }

    pub fn with_parent(parent: Environment) -> Self {
        Self::with_parent_rc(Rc::new(parent))
    }

    pub fn with_parent_rc(parent: Rc<Environment>) -> Self {
        Self {
            values: takeValues(),
            struct_defs: emptyStructs(),
            enum_defs: emptyEnums(),
            struct_contexts: emptyContexts(),
            parent: Some(parent),
            frozen: false,
        }
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.borrow_mut().insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.values.borrow().get(name).cloned() {
            Some(v)
        } else if let Some(parent) = &self.parent {
            parent.get(name)
        } else {
            None
        }
    }

    pub fn assign(&self, name: &str, value: Value) -> Result<(), String> {
        if self.values.borrow().contains_key(name) {
            if self.frozen {
                return Err(format!("Cannot assign to captured variable '{}'", name));
            }
            self.values.borrow_mut().insert(name.to_string(), value);
            Ok(())
        } else if let Some(parent) = &self.parent {
            parent.assign(name, value)
        } else {
            Err(format!("Undefined variable '{}'", name))
        }
    }

    pub fn snapshot_for_capture(&self) -> Self {
        let values = takeValues();
        values.borrow_mut().clone_from(&*self.values.borrow());
        Self {
            values,
            parent: self
                .parent
                .as_ref()
                .map(|parent| Rc::new(parent.snapshot_for_capture())),
            frozen: true,
            struct_defs: self.struct_defs.clone(),
            enum_defs: self.enum_defs.clone(),
            struct_contexts: self.struct_contexts.clone(),
        }
    }

    pub fn register_struct(&mut self, def: StructDef) {
        Rc::make_mut(&mut self.struct_defs).insert(def.name.clone(), def);
    }

    pub fn register_enum(&mut self, name: String, variants: Vec<String>) {
        Rc::make_mut(&mut self.enum_defs).insert(name.clone(), variants.clone());
        let enumName = SharedStr::from(name.as_str());
        let mut namespace = HashMap::new();
        let mut values = self.values.borrow_mut();
        for variant in variants {
            let val = Value::EnumVariant {
                enum_name: enumName.clone(),
                variant: SharedStr::from(variant.as_str()),
            };
            namespace.insert(variant.as_bytes().to_vec(), val.clone());
            values.insert(variant, val);
        }
        values.insert(
            name,
            Value::Namespace(Rc::new(RefCell::new(namespace))),
        );
    }

    pub fn register_struct_context(&mut self, name: String, context: Rc<Environment>) {
        Rc::make_mut(&mut self.struct_contexts).insert(name, context);
    }

    pub fn get_struct_context(&self, name: &str) -> Option<Rc<Environment>> {
        if let Some(context) = self.struct_contexts.get(name) {
            Some(context.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_struct_context(name)
        } else {
            None
        }
    }

    pub fn soleStructName(&self) -> Option<String> {
        let mut names = Vec::new();
        self.collectStructNames(&mut names);
        if names.len() == 1 {
            names.pop()
        } else {
            None
        }
    }

    fn collectStructNames(&self, names: &mut Vec<String>) {
        for name in self.struct_defs.keys() {
            if !names.iter().any(|existing| existing == name) {
                names.push(name.clone());
            }
        }
        if let Some(parent) = &self.parent {
            parent.collectStructNames(names);
        }
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

    pub fn get_enum_for_variant(&self, variant: &str) -> Option<String> {
        if let Some((name, _)) = self
            .enum_defs
            .iter()
            .find(|(_, variants)| variants.iter().any(|candidate| candidate == variant))
        {
            Some(name.clone())
        } else if let Some(parent) = &self.parent {
            parent.get_enum_for_variant(variant)
        } else {
            None
        }
    }
}
