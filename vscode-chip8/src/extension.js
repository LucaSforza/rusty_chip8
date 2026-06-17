const path = require('path');

function activate(context) {
  const { LanguageClient } = require('vscode-languageclient/node');
  const serverCommand = context.asAbsolutePath(path.join('bin', 'chip8-lsp'));
  const client = new LanguageClient('chip8', 'CHIP-8 Assembler', { command: serverCommand }, {
    documentSelector: [{ language: 'chip8' }],
  });
  context.subscriptions.push(client.start());
}

function deactivate() {}

module.exports = { activate, deactivate };
