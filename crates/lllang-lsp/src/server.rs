//! tower-lsp backend for LLLang.

use crate::analysis::{offset_to_position, AnalyzedDocument};
use crate::semantic::{legend_token_types, semantic_tokens};
use dashmap::DashMap;
use std::sync::Arc;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
pub struct Backend {
    client: Client,
    docs: Arc<DashMap<Url, AnalyzedDocument>>,
}

impl Backend {
    fn analyze_and_publish(&self, uri: Url, text: String) {
        let docs = self.docs.clone();
        let client = self.client.clone();
        tokio::spawn(async move {
            let analyzed = AnalyzedDocument::new(uri.clone(), text);
            let diags = analyzed.lsp_diagnostics();
            docs.insert(uri.clone(), analyzed);
            client.publish_diagnostics(uri, diags, None).await;
        });
    }
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: Some(ServerInfo {
                name: "lllang-lsp".into(),
                version: Some(env!("CARGO_PKG_VERSION").into()),
            }),
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
                document_symbol_provider: Some(OneOf::Left(true)),
                folding_range_provider: Some(FoldingRangeProviderCapability::Simple(true)),
                selection_range_provider: Some(SelectionRangeProviderCapability::Simple(true)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),
                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".into(), "@".into()]),
                    ..Default::default()
                }),
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensOptions(
                        SemanticTokensOptions {
                            legend: SemanticTokensLegend {
                                token_types: legend_token_types(),
                                token_modifiers: vec![],
                            },
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                            range: Some(false),
                            ..Default::default()
                        },
                    ),
                ),
                document_formatting_provider: Some(OneOf::Left(true)),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                inlay_hint_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                ..Default::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "LLLang LSP ready (phase 4 MVP)")
            .await;
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let Ok(formatted) = lllang_syntax::format_source(&doc.source) else {
            return Ok(None);
        };
        if formatted == doc.source {
            return Ok(Some(vec![]));
        }
        let end = offset_to_position(&doc.source, doc.source.len());
        Ok(Some(vec![TextEdit {
            range: Range {
                start: Position {
                    line: 0,
                    character: 0,
                },
                end,
            },
            new_text: formatted,
        }]))
    }

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let Some(doc) = self.docs.get(&params.text_document_position_params.text_document.uri)
        else {
            return Ok(None);
        };
        Ok(doc
            .goto_definition(params.text_document_position_params.position)
            .map(GotoDefinitionResponse::Scalar))
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let Some(doc) = self.docs.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(
            doc.find_references(params.text_document_position.position),
        ))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let Some(doc) = self.docs.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        Ok(doc.rename(
            params.text_document_position.position,
            params.new_name,
        ))
    }

    async fn inlay_hint(&self, params: InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(doc.inlay_hints()))
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.analyze_and_publish(params.text_document.uri, params.text_document.text);
    }

    async fn did_change(&self, params: DidChangeTextDocumentParams) {
        if let Some(change) = params.content_changes.into_iter().last() {
            self.analyze_and_publish(params.text_document.uri, change.text);
        }
    }

    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        self.docs.remove(&params.text_document.uri);
        self.client
            .publish_diagnostics(params.text_document.uri, vec![], None)
            .await;
    }

    async fn document_symbol(
        &self,
        params: DocumentSymbolParams,
    ) -> Result<Option<DocumentSymbolResponse>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(DocumentSymbolResponse::Nested(doc.document_symbols())))
    }

    async fn folding_range(&self, params: FoldingRangeParams) -> Result<Option<Vec<FoldingRange>>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(doc.folding_ranges()))
    }

    async fn selection_range(
        &self,
        params: SelectionRangeParams,
    ) -> Result<Option<Vec<SelectionRange>>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        Ok(Some(doc.selection_ranges(&params.positions)))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let Some(doc) = self.docs.get(&params.text_document_position_params.text_document.uri)
        else {
            return Ok(None);
        };
        let Some(text) = doc.hover_text(params.text_document_position_params.position) else {
            return Ok(None);
        };
        Ok(Some(Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::Markdown,
                value: text,
            }),
            range: None,
        }))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let Some(doc) = self.docs.get(&params.text_document_position.text_document.uri) else {
            return Ok(None);
        };
        let items = doc
            .completions(params.text_document_position.position)
            .into_iter()
            .map(|(label, detail, kind)| CompletionItem {
                label,
                detail: Some(detail),
                kind: Some(kind),
                ..Default::default()
            })
            .collect();
        Ok(Some(CompletionResponse::Array(items)))
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let actions = doc
            .code_actions(params.range)
            .into_iter()
            .map(|(title, edit)| {
                CodeActionOrCommand::CodeAction(CodeAction {
                    title,
                    kind: Some(CodeActionKind::QUICKFIX),
                    edit: Some(WorkspaceEdit {
                        changes: Some({
                            let mut m = std::collections::HashMap::new();
                            m.insert(params.text_document.uri.clone(), vec![edit]);
                            m
                        }),
                        ..Default::default()
                    }),
                    ..Default::default()
                })
            })
            .collect();
        Ok(Some(actions))
    }

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let Some(doc) = self.docs.get(&params.text_document.uri) else {
            return Ok(None);
        };
        let data = semantic_tokens(&doc.source);
        Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data,
        })))
    }
}

/// Run the language server over stdio.
pub async fn run_stdio() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();
    let (service, socket) = LspService::new(|client| Backend {
        client,
        docs: Arc::new(DashMap::new()),
    });
    Server::new(stdin, stdout, socket).serve(service).await;
}
