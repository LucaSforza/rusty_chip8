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

        let result = chip8_asm::analyze_with(
            &source,
            &chip8_asm::AssemblyOptions {
                base_dir,
                files: HashMap::new(),
            },
        );

        let mut ws = self.workspace.write().await;
        let doc = ws.documents.get_mut(uri).unwrap();

        match result {
            Ok(analysis) => {
                doc.statements = Some(analysis.statements.clone());
                doc.tokens = Some(analysis.tokens.clone());
                doc.symbol_table = Some(analysis.symbol_table.clone());
                doc.source_map = Some(analysis.source_map.clone());
                doc.addresses = Some(analysis.addresses.clone());
                doc.errors = None;
                doc.analysis = Some(analysis);
            }
            Err(errs) => {
                doc.statements = None;
                doc.tokens = None;
                doc.symbol_table = None;
                doc.source_map = None;
                doc.addresses = None;
                doc.errors = Some(errs);
            }
        }

        let errors = doc.errors.clone();
        let doc_uri = uri.clone();
        let client = self.client.clone();

        tokio::spawn(async move {
            let diags = diagnostics::translate(&errors, &doc_uri);
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
