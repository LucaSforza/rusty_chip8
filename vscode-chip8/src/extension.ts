import * as vscode from 'vscode';

export function activate(context: vscode.ExtensionContext) {
  const serverCommand = context.asAbsolutePath('bin/chip8-lsp');
  const serverOptions: vscode.ServerOptions = { command: serverCommand };
  const clientOptions: vscode.LanguageClientOptions = {
    documentSelector: [{ language: 'chip8' }],
  };
  const client = new vscode.LanguageClient('chip8', 'CHIP-8 Assembler', serverOptions, clientOptions);
  context.subscriptions.push(client.start());
}
