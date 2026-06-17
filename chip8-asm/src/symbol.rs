use std::collections::HashMap;

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct PendingRef {
    pub line: usize,
    pub col: usize,
    pub name: String,
    pub addr: u16,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Default)]
pub struct SymbolTable {
    labels: HashMap<String, u16>,
    constants: HashMap<String, u16>,
    pub pending_refs: Vec<PendingRef>,
}

#[allow(dead_code)]
#[derive(Debug)]
pub enum SymError {
    DuplicateLabel(String, usize),
    DuplicateConst(String, usize),
    Undefined(String, usize, usize, u16),
}

impl SymbolTable {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn define_label(&mut self, name: &str, addr: u16, _line: usize) -> Result<(), SymError> {
        if self.labels.contains_key(name) {
            return Err(SymError::DuplicateLabel(name.to_string(), 0));
        }
        self.labels.insert(name.to_string(), addr);
        Ok(())
    }

    pub fn define_const(&mut self, name: &str, val: u16, _line: usize) -> Result<(), SymError> {
        if self.constants.contains_key(name) {
            return Err(SymError::DuplicateConst(name.to_string(), 0));
        }
        self.constants.insert(name.to_string(), val);
        Ok(())
    }

    pub fn resolve(&self, name: &str) -> Option<u16> {
        self.labels
            .get(name)
            .copied()
            .or_else(|| self.constants.get(name).copied())
    }

    #[allow(dead_code)]
    pub fn add_pending(&mut self, name: String, line: usize, col: usize, addr: u16) {
        self.pending_refs.push(PendingRef {
            line,
            col,
            name,
            addr,
        });
    }

    #[allow(dead_code)]
    pub fn resolve_pending(&self) -> Result<HashMap<String, u16>, SymError> {
        let mut resolved = HashMap::new();
        for pref in &self.pending_refs {
            match self.resolve(&pref.name) {
                Some(val) => {
                    resolved.insert(pref.name.clone(), val);
                }
                None => {
                    return Err(SymError::Undefined(
                        pref.name.clone(),
                        pref.line,
                        pref.col,
                        pref.addr,
                    ));
                }
            }
        }
        Ok(resolved)
    }

    pub fn has_label(&self, name: &str) -> bool {
        self.labels.contains_key(name)
    }

    pub fn labels(&self) -> impl Iterator<Item = (&String, &u16)> {
        self.labels.iter()
    }

    pub fn constants(&self) -> impl Iterator<Item = (&String, &u16)> {
        self.constants.iter()
    }
}
