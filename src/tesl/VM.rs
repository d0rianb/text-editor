use std::collections::HashMap;
use crate::tesl::parser::{Type, Value};

struct Scope {
    memory: HashMap<usize, (Type, Value)>
}

pub struct VM {
    scopes: Vec<Scope>
}

impl VM {
    pub fn new() -> Self {
        Self {
            scopes: vec![]
        }
    }
}