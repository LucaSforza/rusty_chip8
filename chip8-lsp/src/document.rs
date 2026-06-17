use std::path::PathBuf;

use chip8_asm::lexer::Token;
use chip8_asm::parser::Statement;
use chip8_asm::sourcemap::SourceMap;
use chip8_asm::symbol::SymbolTable;
use chip8_asm::AssemblyError;
use chip8_asm::AnalysisResult;

#[derive(Clone)]
pub struct Document {
    pub path: PathBuf,
    pub source: String,
    pub base_dir: PathBuf,
    pub statements: Option<Vec<Statement>>,
    pub tokens: Option<Vec<(Token, usize, usize)>>,
    pub symbol_table: Option<SymbolTable>,
    pub source_map: Option<SourceMap>,
    pub addresses: Option<Vec<u16>>,
    pub errors: Option<Vec<AssemblyError>>,
    pub analysis: Option<AnalysisResult>,
}
