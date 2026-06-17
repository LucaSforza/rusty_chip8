#[derive(Debug, Clone, Default)]
pub struct SourceMap {
    lines: Vec<(String, usize)>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_line(&mut self, file: &str, file_line: usize) {
        self.lines.push((file.to_string(), file_line));
    }

    pub fn resolve(&self, expanded_line: usize) -> (&str, usize) {
        self.lines
            .get(expanded_line)
            .map(|(f, l)| (f.as_str(), *l))
            .unwrap_or(("<unknown>", expanded_line))
    }
}
