// Go AST analysis tool for CodeBuddy
//
// This tool parses Go source code and extracts import information and symbols
// using Go's native go/parser and go/ast packages.
//
// Usage:
//   echo "package main\nimport \"fmt\"" | go run ast_tool.go analyze-imports
//   echo "package main\nfunc foo() {}" | go run ast_tool.go extract-symbols

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

// SymbolInfo represents a symbol (function, struct, interface, etc.)
type SymbolInfo struct {
	Name          string   `json:"name"`
	Kind          string   `json:"kind"` // function, struct, interface, constant, variable, method
	Location      Location `json:"location"`
	Documentation *string  `json:"documentation"`
	Receiver      *string  `json:"receiver"` // For methods only
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

// ExtractSymbols parses Go source and extracts symbol information
func extractSymbols(source string) ([]SymbolInfo, error) {
	fset := token.NewFileSet()

	// Parse the Go source with all declarations
	file, err := parser.ParseFile(fset, "", source, parser.ParseComments)
	if err != nil {
		return nil, fmt.Errorf("failed to parse Go source: %w", err)
	}

	var symbols []SymbolInfo

	// Extract documentation from comment map
	cmap := ast.NewCommentMap(fset, file, file.Comments)

	// Walk the AST and extract symbols
	for _, decl := range file.Decls {
		switch d := decl.(type) {
		case *ast.FuncDecl:
			// Function or method declaration
			pos := fset.Position(d.Pos())
			endPos := fset.Position(d.End())

			location := Location{
				StartLine:   pos.Line,
				StartColumn: pos.Column - 1,
				EndLine:     endPos.Line,
				EndColumn:   endPos.Column - 1,
			}

			// Extract documentation
			var doc *string
			if comments := cmap.Filter(d).Comments(); len(comments) > 0 {
				docText := comments[0].Text()
				doc = &docText
			}

			// Check if it's a method (has receiver)
			var receiver *string
			kind := "function"
			if d.Recv != nil && len(d.Recv.List) > 0 {
				kind = "method"
				// Extract receiver type name
				switch t := d.Recv.List[0].Type.(type) {
				case *ast.StarExpr:
					if ident, ok := t.X.(*ast.Ident); ok {
						receiverName := ident.Name
						receiver = &receiverName
					}
				case *ast.Ident:
					receiverName := t.Name
					receiver = &receiverName
				}
			}

			symbols = append(symbols, SymbolInfo{
				Name:          d.Name.Name,
				Kind:          kind,
				Location:      location,
				Documentation: doc,
				Receiver:      receiver,
			})

		case *ast.GenDecl:
			// General declarations (import, const, type, var)
			for _, spec := range d.Specs {
				switch s := spec.(type) {
				case *ast.TypeSpec:
					// Type declaration (struct, interface, type alias)
					pos := fset.Position(s.Pos())
					endPos := fset.Position(s.End())

					location := Location{
						StartLine:   pos.Line,
						StartColumn: pos.Column - 1,
						EndLine:     endPos.Line,
						EndColumn:   endPos.Column - 1,
					}

					// Extract documentation
					var doc *string
					if d.Doc != nil {
						docText := d.Doc.Text()
						doc = &docText
					}

					// Determine the specific type
					kind := "other"
					switch s.Type.(type) {
					case *ast.StructType:
						kind = "struct"
					case *ast.InterfaceType:
						kind = "interface"
					default:
						kind = "other" // Type alias or named type
					}

					symbols = append(symbols, SymbolInfo{
						Name:          s.Name.Name,
						Kind:          kind,
						Location:      location,
						Documentation: doc,
						Receiver:      nil,
					})

				case *ast.ValueSpec:
					// Constant or variable declaration
					pos := fset.Position(s.Pos())
					endPos := fset.Position(s.End())

					location := Location{
						StartLine:   pos.Line,
						StartColumn: pos.Column - 1,
						EndLine:     endPos.Line,
						EndColumn:   endPos.Column - 1,
					}

					// Extract documentation
					var doc *string
					if d.Doc != nil {
						docText := d.Doc.Text()
						doc = &docText
					}

					// Determine if it's a constant or variable
					kind := "variable"
					if d.Tok == token.CONST {
						kind = "constant"
					}

					// Multiple names can be declared in one spec (e.g., var a, b int)
					for _, name := range s.Names {
						symbols = append(symbols, SymbolInfo{
							Name:          name.Name,
							Kind:          kind,
							Location:      location,
							Documentation: doc,
							Receiver:      nil,
						})
					}
				}
			}
		}
	}

	return symbols, nil
}

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <command>\n", os.Args[0])
		fmt.Fprintf(os.Stderr, "Commands:\n")
		fmt.Fprintf(os.Stderr, "  analyze-imports  Parse Go source from stdin and output import information as JSON\n")
		fmt.Fprintf(os.Stderr, "  extract-symbols  Parse Go source from stdin and output symbol information as JSON\n")
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

	case "extract-symbols":
		// Read source from stdin
		sourceBytes, err := io.ReadAll(os.Stdin)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error reading stdin: %v\n", err)
			os.Exit(1)
		}

		source := string(sourceBytes)

		// Extract symbols
		symbols, err := extractSymbols(source)
		if err != nil {
			fmt.Fprintf(os.Stderr, "Error extracting symbols: %v\n", err)
			os.Exit(1)
		}

		// Output as JSON
		output, err := json.MarshalIndent(symbols, "", "  ")
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
