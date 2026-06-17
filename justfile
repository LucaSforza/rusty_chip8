home := env_var_or_default("HOME", "~")

# Build the LSP server
build lsp:
    cargo build -p chip8-lsp

# Build the LSP server (release)
build lsp-release:
    cargo build -p chip8-lsp --release

# Build all workspace crates
build all:
    cargo build --workspace

# Test the LSP server
test lsp:
    cargo test -p chip8-lsp

# Test all workspace crates
test all:
    cargo test --workspace

# Install LSP for all editors
install lsp: install-lsp-nvim install-lsp-vscode

# Install LSP binary and Neovim config
install lsp-nvim:
    cargo build -p chip8-lsp --release
    mkdir -p {{home}}/.local/bin
    cp target/release/chip8-lsp {{home}}/.local/bin/chip8-lsp
    @echo ""
    @echo "✓ Installed chip8-lsp to ~/.local/bin/chip8-lsp"
    @echo ""
    @echo "Add to your Neovim LSP config (~/.config/nvim/lua/plugins/lsp.lua):"
    @echo ""
    @echo "  return {"
    @echo "    {"
    @echo "      'neovim/nvim-lspconfig',"
    @echo "      opts = {"
    @echo "        servers = {"
    @echo "          chip8_asm = {"
    @echo "            cmd = { 'chip8-lsp' },"
    @echo "            filetypes = { 'asm', 'chip8' },"
    @echo "          },"
    @echo "        },"
    @echo "      },"
    @echo "    },"
    @echo "  }"

# Prepare VSCode extension
install lsp-vscode:
    cargo build -p chip8-lsp --release
    mkdir -p vscode-chip8/bin
    cp target/release/chip8-lsp vscode-chip8/bin/chip8-lsp
    cd vscode-chip8 && npm install 2>/dev/null; npx --yes @vscode/vsce package 2>/dev/null || echo "Install vsce: npm install -g @vscode/vsce"
    @echo ""
    @echo "✓ VSIX created in vscode-chip8/"
    @echo "Install: code --install-extension vscode-chip8/chip8-asm-*.vsix"

# Run the LSP (stdio mode)
run lsp:
    cargo run -p chip8-lsp

# Clean LSP build artifacts
clean lsp:
    cargo clean -p chip8-lsp
