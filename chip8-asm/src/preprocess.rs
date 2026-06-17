use std::path::Path;

use crate::sourcemap::SourceMap;
use crate::include::{FileProvider, IncludeError, IncludeResolver};

pub struct PreprocessResult {
    pub source: String,
    pub source_map: SourceMap,
}

#[derive(Debug)]
pub enum PreprocessError {
    Include(IncludeError),
    Macro(String),
}

impl std::fmt::Display for PreprocessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PreprocessError::Include(e) => write!(f, "{}", e),
            PreprocessError::Macro(msg) => write!(f, "macro error: {}", msg),
        }
    }
}

impl From<IncludeError> for PreprocessError {
    fn from(e: IncludeError) -> Self {
        PreprocessError::Include(e)
    }
}

pub fn preprocess(
    source: &str,
    base_dir: &Path,
    provider: &dyn FileProvider,
) -> Result<PreprocessResult, Vec<PreprocessError>> {
    let mut errors = Vec::new();

    // Step 1: Include resolution
    let mut include_resolver = IncludeResolver::new();
    let (expanded, source_map) = match include_resolver.resolve(source, base_dir, provider) {
        Ok(r) => r,
        Err(e) => {
            errors.push(e.into());
            return Err(errors);
        }
    };

    // Step 2: Collect macro definitions
    let (stripped, macro_defs) = match crate::macroexpand::collect_definitions(&expanded) {
        Ok(r) => r,
        Err(e) => {
            errors.push(PreprocessError::Macro(format!("{}", e)));
            return Err(errors);
        }
    };

    // Step 3: Expand macro invocations
    let final_source = match crate::macroexpand::expand(&stripped, &macro_defs) {
        Ok(r) => r,
        Err(e) => {
            errors.push(PreprocessError::Macro(format!("{}", e)));
            return Err(errors);
        }
    };

    Ok(PreprocessResult {
        source: final_source,
        source_map,
    })
}
