mod server;
mod document;
mod diagnostics;
mod workspace;
mod symbols;
mod completion;
mod hover;
mod definition;
mod references;
mod highlight;
mod rename;

use server::LspServer;
use tower_lsp::LspService;

#[tokio::main]
async fn main() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::new(|client| LspServer::new(client));
    tower_lsp::Server::new(stdin, stdout, socket)
        .serve(service)
        .await;
}
