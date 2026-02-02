# TypeMill Demo - VHS Tape
# Run with: vhs demo.tape
# Install VHS: brew install charmbracelet/tap/vhs

Output demo.gif

Set FontSize 14
Set Width 1200
Set Height 600
Set Theme "Catppuccin Mocha"
Set Padding 20

# Title
Type "# TypeMill - AI-Powered Code Refactoring"
Enter
Sleep 1s

# Show help
Type "npx @goobits/typemill --help"
Enter
Sleep 3s

# Clear and show status
Type "clear"
Enter
Sleep 500ms

Type "# Check LSP server status"
Enter
Sleep 500ms

Type "npx @goobits/typemill status"
Enter
Sleep 4s

# Clear and show tools
Type "clear"
Enter
Sleep 500ms

Type "# Available MCP Tools"
Enter
Sleep 500ms

Type "npx @goobits/typemill tools"
Enter
Sleep 4s

# Show a rename operation
Type "clear"
Enter
Sleep 500ms

Type "# Demo: Rename a file with automatic import updates"
Enter
Sleep 1s

Type "# Before: src/utils.ts is imported by 3 files"
Enter
Sleep 1s

Type "# After: src/helpers.ts - all imports automatically updated!"
Enter
Sleep 2s

Type "npx @goobits/typemill tool rename_all --target '{\"kind\":\"file\",\"filePath\":\"src/utils.ts\"}' --newName 'src/helpers.ts' --options '{\"dryRun\":true}'"
Enter
Sleep 5s

# End
Type "clear"
Enter
Sleep 500ms

Type "# TypeMill - npx @goobits/typemill start"
Enter
Type "# Pure Rust MCP Server for AI Code Assistants"
Enter
Sleep 3s
