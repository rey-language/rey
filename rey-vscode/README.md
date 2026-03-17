# Rey — VSCode extension

Syntax highlighting for the Rey programming language (`.rey`).

## Features
- Keywords, types, and builtins highlighting
- Strings, numbers, booleans, and `//` comments
- Function declarations and calls

## Install locally (development)
1. Open the `rey-vscode/` folder in VSCode.
2. Press `F5` to launch an Extension Development Host.
3. Open a `.rey` file in the dev host and verify highlighting.

## Install locally (VSIX)
1. Package the extension:
   - `npx @vscode/vsce package`
2. Install the produced `.vsix`:
   - `code --install-extension rey-lang-0.0.1.vsix`

## Install from Marketplace
When published, search for `Rey` or `rey-lang` by `rey-language` in the VSCode Extensions view.
