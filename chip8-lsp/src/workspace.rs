use std::collections::HashMap;
use std::path::PathBuf;

use tower_lsp::lsp_types::Url;

use crate::document::Document;

#[derive(Default)]
pub struct Workspace {
    pub documents: HashMap<Url, Document>,
    pub include_graph: HashMap<PathBuf, Vec<PathBuf>>,
    pub included_by: HashMap<PathBuf, Vec<PathBuf>>,
}

impl Workspace {
    pub fn new() -> Self {
        Workspace::default()
    }

    pub fn index_file(&mut self, path: &PathBuf, source: &str) {
        let includes = extract_includes(source);
        let prev = self.include_graph.insert(path.clone(), includes.clone());

        if let Some(old) = prev {
            for inc in &old {
                if let Some(parents) = self.included_by.get_mut(inc) {
                    parents.retain(|p| p != path);
                }
            }
        }

        for inc in &includes {
            self.included_by.entry(inc.clone()).or_default().push(path.clone());
        }
    }

    pub fn get_document(&self, uri: &Url) -> Option<&Document> {
        self.documents.get(uri)
    }
}

fn extract_includes(source: &str) -> Vec<PathBuf> {
    let mut includes = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        let rest = trimmed
            .strip_prefix("include ")
            .or_else(|| trimmed.strip_prefix("INCLUDE "))
            .and_then(|r| {
                let r = r.trim();
                if r.starts_with('"') && r.ends_with('"') {
                    Some(&r[1..r.len()-1])
                } else {
                    None
                }
            });
        if let Some(path) = rest {
            includes.push(PathBuf::from(path));
        }
    }
    includes
}
