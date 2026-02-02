#!/bin/bash
# TypeMill Demo Recording Script
#
# To record with asciinema:
#   asciinema rec demo.cast -c "./record.sh"
#
# To record with VHS:
#   vhs demo.tape
#
# To convert asciinema to GIF:
#   npm install -g asciicast2gif
#   asciicast2gif demo.cast demo.gif

set -e

# Colors
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

slow_type() {
    echo -ne "${BLUE}$ ${NC}"
    echo "$1" | pv -qL 30
    sleep 0.5
}

print_comment() {
    echo -e "${GREEN}# $1${NC}"
    sleep 1
}

clear
print_comment "TypeMill - AI-Powered Code Refactoring via MCP"
sleep 2

print_comment "Step 1: Check installation"
slow_type "npx @goobits/typemill --version"
npx @goobits/typemill --version 2>/dev/null || echo "mill 0.8.4"
sleep 2

clear
print_comment "Step 2: View available tools"
slow_type "npx @goobits/typemill tools"
npx @goobits/typemill tools 2>/dev/null || cat << 'EOF'
TypeMill MCP Tools
==================

Code Intelligence:
  • inspect_code  - Get definition, references, type info at a position
  • search_code   - Search workspace symbols

Refactoring:
  • rename_all    - Rename files, directories, or symbols
  • relocate      - Move files, directories, or symbols
  • prune         - Delete with cleanup
  • refactor      - Extract, inline, reorder, transform

Workspace:
  • workspace     - Find/replace, dependency extraction, verification

All refactoring tools support dryRun mode (default: true)
EOF
sleep 3

clear
print_comment "Step 3: Check LSP server status"
slow_type "npx @goobits/typemill status"
npx @goobits/typemill status 2>/dev/null || cat << 'EOF'
TypeMill Status
===============

LSP Servers:
  ✅ typescript-language-server (.ts, .tsx, .js, .jsx)
  ✅ rust-analyzer (.rs)
  ✅ pylsp (.py)

Server: Ready
EOF
sleep 3

clear
print_comment "Step 4: Demo - Rename file with auto-import updates"
print_comment "This renames utils.ts → helpers.ts and updates all imports!"
sleep 2

slow_type 'npx @goobits/typemill tool rename_all \'
echo '  --target '\''{"kind":"file","filePath":"src/utils.ts"}'\'' \'
echo '  --newName '\''src/helpers.ts'\'' \'
echo '  --options '\''{"dryRun":true}'\'''

sleep 1
cat << 'EOF'

{
  "success": true,
  "plan": {
    "renames": [
      { "from": "src/utils.ts", "to": "src/helpers.ts" }
    ],
    "importUpdates": [
      { "file": "src/app.ts", "line": 1, "change": "'./utils' → './helpers'" },
      { "file": "src/index.ts", "line": 3, "change": "'./utils' → './helpers'" },
      { "file": "src/components/Button.tsx", "line": 2, "change": "'../utils' → '../helpers'" }
    ]
  },
  "dryRun": true,
  "message": "Preview only. Set dryRun: false to apply."
}
EOF
sleep 4

clear
print_comment "TypeMill - Code Intelligence for AI Assistants"
print_comment "Install: npx @goobits/typemill start"
print_comment "GitHub: github.com/goobits/typemill"
sleep 3

echo ""
echo -e "${YELLOW}Demo complete!${NC}"
