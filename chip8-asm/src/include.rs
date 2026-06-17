use std::collections::HashMap;
use std::path::{Path, PathBuf};

use crate::sourcemap::SourceMap;

pub trait FileProvider {
    fn read_file(&self, path: &Path) -> Result<String, String>;
}

pub struct FsFileProvider;

impl FileProvider for FsFileProvider {
    fn read_file(&self, path: &Path) -> Result<String, String> {
        std::fs::read_to_string(path)
            .map_err(|e| format!("cannot read {}: {}", path.display(), e))
    }
}

pub struct OverlayFileProvider<'a> {
    pub overlay: &'a HashMap<PathBuf, String>,
    pub base: &'a dyn FileProvider,
}

impl<'a> FileProvider for OverlayFileProvider<'a> {
    fn read_file(&self, path: &Path) -> Result<String, String> {
        if let Some(content) = self.overlay.get(path) {
            return Ok(content.clone());
        }
        self.base.read_file(path)
    }
}

#[derive(Debug)]
pub enum IncludeError {
    FileNotFound(String),
    IncludeCycle(Vec<String>),
    ReadError(String),
}

impl std::fmt::Display for IncludeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IncludeError::FileNotFound(path) => write!(f, "file not found: {}", path),
            IncludeError::IncludeCycle(chain) => {
                write!(f, "include cycle: {}", chain.join(" → "))
            }
            IncludeError::ReadError(msg) => write!(f, "{}", msg),
        }
    }
}

pub struct IncludeResolver {
    source_map: SourceMap,
}

impl IncludeResolver {
    pub fn new() -> Self {
        IncludeResolver {
            source_map: SourceMap::new(),
        }
    }

    pub fn resolve(
        &mut self,
        source: &str,
        base_dir: &Path,
        provider: &dyn FileProvider,
    ) -> Result<(String, SourceMap), IncludeError> {
        let mut stack: Vec<PathBuf> = Vec::new();
        let mut result = String::new();

        self.resolve_lines(source, "<root>", base_dir, provider, &mut stack, &mut result)?;

        Ok((result, self.source_map.clone()))
    }

    fn resolve_lines(
        &mut self,
        source: &str,
        file_name: &str,
        base_dir: &Path,
        provider: &dyn FileProvider,
        stack: &mut Vec<PathBuf>,
        result: &mut String,
    ) -> Result<(), IncludeError> {
        for (line_idx, line) in source.lines().enumerate() {
            let trimmed = line.trim();
            if let Some(include_path) = Self::parse_include(trimmed) {
                let resolved_path = base_dir.join(&include_path);
                let canonical = resolved_path
                    .canonicalize()
                    .unwrap_or_else(|_| resolved_path.clone());

                if stack.iter().any(|p| *p == canonical) {
                    let cycle: Vec<String> = stack
                        .iter()
                        .chain(std::iter::once(&canonical))
                        .map(|p| p.display().to_string())
                        .collect();
                    return Err(IncludeError::IncludeCycle(cycle));
                }

                let included_source = provider.read_file(&resolved_path).map_err(|e| {
                    IncludeError::ReadError(format!("{}: {}", resolved_path.display(), e))
                })?;
                let included_dir = resolved_path.parent().unwrap_or(Path::new("."));
                let included_name = resolved_path.display().to_string();

                stack.push(canonical);
                self.resolve_lines(
                    &included_source,
                    &included_name,
                    included_dir,
                    provider,
                    stack,
                    result,
                )?;
                stack.pop();
            } else {
                result.push_str(line);
                result.push('\n');
                self.source_map.add_line(file_name, line_idx);
            }
        }

        Ok(())
    }

    fn parse_include(line: &str) -> Option<String> {
        let line = line.trim();
        let rest = line
            .strip_prefix("include ")
            .or_else(|| line.strip_prefix("INCLUDE "))?;
        let rest = rest.trim();
        if rest.starts_with('"') && rest.ends_with('"') {
            let inner = &rest[1..rest.len() - 1];
            if !inner.is_empty() {
                return Some(inner.to_string());
            }
        }
        None
    }
}
