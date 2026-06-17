use std::collections::HashMap;
use std::sync::Arc;

use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer};
use tokio::sync::RwLock;

use crate::completion;
use crate::definition;
use crate::diagnostics;
use crate::document::Document;
use crate::highlight;
use crate::hover;
use crate::references;
use crate::rename;
use crate::symbols;
use crate::workspace::Workspace;

pub struct LspServer {
    client: Client,
    workspace: Arc<RwLock<Workspace>>,
}

impl LspServer {
    pub fn new(client: Client) -> Self {
        LspServer {
            client,
            workspace: Arc::new(RwLock::new(Workspace::new())),
        }
    }

    async fn analyze_file(&self, uri: &Url) {
        let doc = {
            let ws = self.workspace.read().await;
            ws.documents.get(uri).map(|d| {
                let path = d.path.clone();
                let source = d.source.clone();
                let base_dir = d.base_dir.clone();
                (path, source, base_dir)
            })
        };

        let Some((_path, source, base_dir)) = doc else { return };

        // Do partial analysis: tokenize first, then parse, then compute layout
        // This way, even with errors we keep the partial state
        let tokens = chip8_asm::lexer::tokenize(&source);

        let mut errors = Vec::new();
        let has_lex_errors: Vec<(String, usize, usize)> = tokens.iter()
            .filter_map(|(t, l, c)| if let chip8_asm::lexer::Token::Error(s) = t {
                Some((s.clone(), *l, *c))
            } else { None })
            .collect();

        for (msg, line, col) in has_lex_errors {
            errors.push(chip8_asm::AssemblyError::from_string(format!("lex error at {}:{}: {}", line + 1, col + 1, msg)));
        }

        let (statements, parse_errors) = match chip8_asm::parser::parse(&tokens) {
            Ok(stmts) => (stmts, Vec::new()),
            Err(parse_errs) => {
                let errs: Vec<chip8_asm::AssemblyError> = parse_errs.iter().map(|e| {
                    let (l, c) = crate::diagnostics::extract_parse_pos(e);
                    chip8_asm::AssemblyError {
                        message: e.to_string(),
                        file: Some("<root>".into()),
                        line: l,
                        col: c,
                    }
                }).collect();
                (Vec::new(), errs)
            }
        };

        errors.extend(parse_errors);

        let (symbol_table, addresses) = if !statements.is_empty() {
            chip8_asm::compute_layout(&statements).unwrap_or_default()
        } else {
            (Default::default(), Vec::new())
        };

        let mut ws = self.workspace.write().await;
        let doc = ws.documents.get_mut(uri).unwrap();

        doc.tokens = Some(tokens);
        doc.statements = Some(statements);
        doc.symbol_table = Some(symbol_table);
        doc.addresses = Some(addresses);

        let empty_errors: Vec<chip8_asm::AssemblyError> = Vec::new();
        doc.errors = if errors.is_empty() { None } else { Some(errors) };
        doc.analysis = doc.tokens.as_ref().map(|_| chip8_asm::AnalysisResult {
            source: source.clone(),
            expanded_source: source.clone(),
            source_map: chip8_asm::sourcemap::SourceMap::new(),
            tokens: doc.tokens.as_ref().unwrap().clone(),
            statements: doc.statements.as_ref().unwrap_or(&Vec::new()).clone(),
            addresses: doc.addresses.as_ref().unwrap_or(&Vec::new()).clone(),
            symbol_table: doc.symbol_table.as_ref().unwrap().clone(),
            macro_defs: Vec::new(),
        });
        doc.source_map = doc.analysis.as_ref().map(|a| a.source_map.clone());

        let publish_errors = doc.errors.clone().unwrap_or(empty_errors);
        let doc_uri = uri.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let diags = diagnostics::translate(&Some(publish_errors), &doc_uri);
            client.publish_diagnostics(doc_uri, diags, None).await;
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for LspServer {
    async fn initialize(&self, _params: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "chip8-lsp".into(),
                version: Some("0.1.0".into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: None,
                    trigger_characters: Some(vec![".".into()]),
                    ..Default::default()
                }),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                workspace_symbol_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                        legend: SemanticTokensLegend {
                            token_types: highlight::TOKEN_TYPES.to_vec(),
                            token_modifiers: highlight::TOKEN_MODIFIERS.to_vec(),
                        },
                        full: Some(SemanticTokensFullOptions::Bool(true)),
                        ..Default::default()
                    }),
                ),
                rename_provider: Some(OneOf::Left(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _params: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "chip8-lsp initialized")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = params.text_document.text;

        let path = uri.to_file_path().unwrap_or_default();
        let base_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

        {
            let mut ws = self.workspace.write().await;
            ws.documents.insert(
                uri.clone(),
                Document {
                    path: path.clone(),
                    source: source.clone(),
                    base_dir,
                    statements: None,
                    tokens: None,
                    symbol_table: None,
                    source_map: None,
                    addresses: None,
                    errors: None,
                    analysis: None,
                },
            );
            ws.index_file(&path, &source);
        }

        self.analyze_file(&uri).await;
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        let uri = params.text_document.uri;
        let source = params.content_changes.into_iter().next()
            .map(|c| c.text)
            .unwrap_or_default();

        let path = uri.to_file_path().unwrap_or_default();
        let base_dir = path.parent().map(|p| p.to_path_buf()).unwrap_or_default();

        {
            let mut ws = self.workspace.write().await;
            ws.documents.insert(
                uri.clone(),
                Document {
                    path: path.clone(),
                    source: source.clone(),
                    base_dir,
                    statements: None,
                    tokens: None,
                    symbol_table: None,
                    source_map: None,
                    addresses: None,
                    errors: None,
                    analysis: None,
                },
            );
            ws.index_file(&path, &source);
        }

        self.analyze_file(&uri).await;
    }

    async fn did_save(&self, params: DidSaveTextDocumentParams) {
        let uri = params.text_document.uri;
        self.analyze_file(&uri).await;
        self.client
            .log_message(MessageType::INFO, format!("saved: {}", uri.as_str()))
            .await;
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;
        let mut ws = self.workspace.write().await;
        ws.documents.remove(&uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let ws = self.workspace.read().await;
        Ok(hover::get_hover(&ws, &uri, pos))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let uri = params.text_document_position_params.text_document.uri;
        let pos = params.text_document_position_params.position;
        let ws = self.workspace.read().await;
        Ok(definition::goto_definition(&ws, &uri, pos))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let ws = self.workspace.read().await;
        Ok(references::find_references(&ws, &uri, pos))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let ws = self.workspace.read().await;
        Ok(completion::get_completions(&ws, &uri, pos))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let ws = self.workspace.read().await;
        Ok(highlight::get_semantic_tokens(&ws, &uri))
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let ws = self.workspace.read().await;
        Ok(symbols::document_symbols(&ws, &uri))
    }

    async fn symbol(&self, params: WorkspaceSymbolParams) -> Result<Option<Vec<SymbolInformation>>> {
        let ws = self.workspace.read().await;
        Ok(symbols::workspace_symbols(&ws, &params.query))
    }

    async fn prepare_rename(
        &self,
        params: TextDocumentPositionParams,
    ) -> Result<Option<PrepareRenameResponse>> {
        let uri = params.text_document.uri;
        let pos = params.position;
        let ws = self.workspace.read().await;
        Ok(rename::prepare_rename(&ws, &uri, pos))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let pos = params.text_document_position.position;
        let new_name = params.new_name;
        let ws = self.workspace.read().await;
        Ok(rename::perform_rename(&ws, &uri, pos, &new_name))
    }
}
