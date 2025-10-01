// Go AST analysis tool for CodeBuddy
//
// This tool parses Go source code and extracts import information
// using Go's native go/parser and go/ast packages.
//
// Usage: echo "package main\nimport \"fmt\"" | go run ast_tool.go analyze-imports

package main

import (
	"encoding/json"
	"fmt"
	"go/ast"
	"go/parser"
	"go/token"
	"io"
	"os"
	"strings"
)

// ImportInfo represents a single import statement
type ImportInfo struct {
	ModulePath      string        `json:"module_path"`
	ImportType      string        `json:"import_type"`
	NamedImports    []NamedImport `json:"named_imports"`
	DefaultImport   *string       `json:"default_import"`
	NamespaceImport *string       `json:"namespace_import"`
	TypeOnly        bool          `json:"type_only"`
	Location        Location      `json:"location"`
}

// NamedImport represents a named import
type NamedImport struct {
	Name     string  `json:"name"`
	Alias    *string `json:"alias"`
	TypeOnly bool    `json:"type_only"`
}

// Location represents source code location
type Location struct {
	StartLine   int `json:"start_line"`
	StartColumn int `json:"start_column"`
	EndLine     int `json:"end_line"`
	EndColumn   int `json:"end_column"`
}

// AnalyzeImports parses Go source and extracts import information
func analyzeImports(source string) ([]ImportInfo, error) {
	fset := token.NewFileSet()

	// Parse the Go source
	file, err := parser.ParseFile(fset, "", source, parser.ImportsOnly)
	if err != nil {
		return nil, fmt.Errorf("failed to parse Go source: %w", err)
	}

	var imports []ImportInfo

	for _, importSpec := range file.Imports {
		// Get the import path (remove quotes)
		modulePath := strings.Trim(importSpec.Path.Value, `"`)

		// Get position information
		pos := fset.Position(importSpec.Pos())
		endPos := fset.Position(importSpec.End())

		location := Location{
			StartLine:   pos.Line - 1, // Convert to 0-based
			StartColumn: pos.Column - 1,
			EndLine:     endPos.Line - 1,
			EndColumn:   endPos.Column - 1,
		}

		var alias *string
		var nameImport *string

		// Check if there's an alias (e.g., import foo "fmt")
		if importSpec.Name != nil {
			aliasName := importSpec.Name.Name
			if aliasName == "." {
				// Dot import - treat as namespace import
				nameImport = &modulePath
			} else if aliasName != "_" {
				// Named alias
				alias = &aliasName
			}
			// Blank identifier "_" is ignored
		}

		// Determine what's being imported from the path
		pathParts := strings.Split(modulePath, "/")
		lastPart := pathParts[len(pathParts)-1]

		importInfo := ImportInfo{
			ModulePath:      modulePath,
			ImportType:      "es_module", // Go uses a similar module system
			NamedImports:    []NamedImport{},
			DefaultImport:   nil,
			NamespaceImport: nameImport,
			TypeOnly:        false,
			Location:        location,
		}

		// If there's an alias, add it as a named import
		if alias != nil {
			importInfo.NamedImports = append(importInfo.NamedImports, NamedImport{
				Name:     lastPart,
				Alias:    alias,
				TypeOnly: false,
			})
		} else if nameImport == nil {
			// Regular import - package name is the last part of the path
			importInfo.NamedImports = append(importInfo.NamedImports, NamedImport{
				Name:     lastPart,
				Alias:    nil,
				TypeOnly: false,
			})
		}

		imports = append(imports, importInfo)
	}

	return imports, nil
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <command>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Commands:\n")
		fmt.Fprintf(os.Stderr, "  analyze-imports  Parse Go source from stdin and output import information as JSON\n")
		os.Exit(1)
	}

	command := os.Args[1]

	switch command {
	case "analyze-imports":
		// Read source from stdin
		sourceBytes, err := io.ReadAll(os.Stdin)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading stdin: %v\n", err)
			os.Exit(1)
		}

		source := string(sourceBytes)

		// Analyze imports
		imports, err := analyzeImports(source)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error analyzing imports: %v\n", err)
			os.Exit(1)
		}

		// Output as JSON
		output, err := json.MarshalIndent(imports, "", "  ")
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error marshaling JSON: %v\n", err)
			os.Exit(1)
		}

		fmt.Println(string(output))

	default:
		fmt.Fprintf(os.Stderr, "Unknown command: %s\n", command)
		os.Exit(1)
	}
}
