# TypeMill Demo

## Recording

The demo was recorded using asciinema and converted to SVG:

```bash
# Record
TERM=xterm-256color asciinema rec demo.cast -c "./record.sh"

# Convert to animated SVG
npm install -g svg-term-cli
svg-term --in demo.cast --out demo.svg --window --no-cursor
```

## Files

- `record.sh` - Demo script showing TypeMill commands
- `demo.cast` - asciinema recording
- `demo.svg` - Animated SVG for README

## Demo Content

The script demonstrates:
1. Version check (`npx @goobits/typemill --version`)
2. Available tools list (`npx @goobits/typemill tools`)
3. LSP server status (`npx @goobits/typemill status`)
4. Rename preview with dry-run mode
