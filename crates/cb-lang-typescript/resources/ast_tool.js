#!/usr/bin/env node

// TypeScript/JavaScript AST analysis tool for CodeBuddy
//
// This tool parses TypeScript/JavaScript source code and extracts import information
// and symbols using @babel/parser for maximum compatibility.
//
// Usage:
//   echo "import React from 'react';" | node ast_tool.js analyze-imports
//   echo "function foo() {}" | node ast_tool.js extract-symbols

const fs = require('fs');

// Try to use @babel/parser if available, fallback to simple regex parsing
let parser;
let traverse;

try {
    parser = require('@babel/parser');
    traverse = require('@babel/traverse').default;
} catch (e) {
    // Babel not available - will use regex fallback
    parser = null;
    traverse = null;
}

/**
 * Analyze imports from source code
 */
function analyzeImports(source) {
    if (parser && traverse) {
        return analyzeImportsAST(source);
    } else {
        return analyzeImportsRegex(source);
    }
}

/**
 * AST-based import analysis using Babel
 */
function analyzeImportsAST(source) {
    try {
        const ast = parser.parse(source, {
            sourceType: 'module',
            plugins: [
                'typescript',
                'jsx',
                'decorators-legacy',
                'classProperties',
                'dynamicImport'
            ]
        });

        const imports = [];

        traverse(ast, {
            // ES6 import statements
            ImportDeclaration(path) {
                const node = path.node;
                const modulePath = node.source.value;
                const loc = node.loc;

                const importInfo = {
                    module_path: modulePath,
                    import_type: 'es_module',
                    named_imports: [],
                    default_import: null,
                    namespace_import: null,
                    type_only: node.importKind === 'type',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    }
                };

                // Process specifiers
                node.specifiers.forEach(spec => {
                    if (spec.type === 'ImportDefaultSpecifier') {
                        importInfo.default_import = spec.local.name;
                    } else if (spec.type === 'ImportNamespaceSpecifier') {
                        importInfo.namespace_import = spec.local.name;
                    } else if (spec.type === 'ImportSpecifier') {
                        importInfo.named_imports.push({
                            name: spec.imported.name,
                            alias: spec.local.name !== spec.imported.name ? spec.local.name : null,
                            type_only: spec.importKind === 'type'
                        });
                    }
                });

                imports.push(importInfo);
            },

            // CommonJS require() calls
            CallExpression(path) {
                const node = path.node;
                if (node.callee.name === 'require' &&
                    node.arguments.length > 0 &&
                    node.arguments[0].type === 'StringLiteral') {

                    const modulePath = node.arguments[0].value;
                    const loc = node.loc;

                    imports.push({
                        module_path: modulePath,
                        import_type: 'commonjs',
                        named_imports: [],
                        default_import: null,
                        namespace_import: null,
                        type_only: false,
                        location: {
                            start_line: loc.start.line,
                            start_column: loc.start.column,
                            end_line: loc.end.line,
                            end_column: loc.end.column
                        }
                    });
                }
            },

            // Dynamic import()
            Import(path) {
                const parent = path.parent;
                if (parent.type === 'CallExpression' &&
                    parent.arguments.length > 0 &&
                    parent.arguments[0].type === 'StringLiteral') {

                    const modulePath = parent.arguments[0].value;
                    const loc = parent.loc;

                    imports.push({
                        module_path: modulePath,
                        import_type: 'dynamic',
                        named_imports: [],
                        default_import: null,
                        namespace_import: null,
                        type_only: false,
                        location: {
                            start_line: loc.start.line,
                            start_column: loc.start.column,
                            end_line: loc.end.line,
                            end_column: loc.end.column
                        }
                    });
                }
            }
        });

        return imports;
    } catch (error) {
        // Fall back to regex if AST parsing fails
        return analyzeImportsRegex(source);
    }
}

/**
 * Regex-based import analysis (fallback)
 */
function analyzeImportsRegex(source) {
    const imports = [];
    const lines = source.split('\n');

    // Regex patterns
    const es6ImportPattern = /^import\s+(?:(?:{[^}]*}|[\w$]+|\*\s+as\s+[\w$]+)(?:\s*,\s*(?:{[^}]*}|[\w$]+|\*\s+as\s+[\w$]+))*\s+from\s+)?['"]([^'"]+)['"]/;
    const requirePattern = /require\s*\(\s*['"]([^'"]+)['"]\s*\)/;
    const dynamicImportPattern = /import\s*\(\s*['"]([^'"]+)['"]\s*\)/;

    lines.forEach((line, index) => {
        const lineNum = index + 1;

        // ES6 imports
        let match = line.match(es6ImportPattern);
        if (match) {
            imports.push({
                module_path: match[1],
                import_type: 'es_module',
                named_imports: [],
                default_import: null,
                namespace_import: null,
                type_only: line.includes('import type'),
                location: {
                    start_line: lineNum,
                    start_column: 0,
                    end_line: lineNum,
                    end_column: line.length
                }
            });
            return;
        }

        // CommonJS require
        match = line.match(requirePattern);
        if (match) {
            imports.push({
                module_path: match[1],
                import_type: 'commonjs',
                named_imports: [],
                default_import: null,
                namespace_import: null,
                type_only: false,
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('require'),
                    end_line: lineNum,
                    end_column: line.indexOf('require') + match[0].length
                }
            });
            return;
        }

        // Dynamic import
        match = line.match(dynamicImportPattern);
        if (match) {
            imports.push({
                module_path: match[1],
                import_type: 'dynamic',
                named_imports: [],
                default_import: null,
                namespace_import: null,
                type_only: false,
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('import('),
                    end_line: lineNum,
                    end_column: line.indexOf('import(') + match[0].length
                }
            });
        }
    });

    return imports;
}

/**
 * Extract symbols from source code
 */
function extractSymbols(source) {
    if (parser && traverse) {
        return extractSymbolsAST(source);
    } else {
        return extractSymbolsRegex(source);
    }
}

/**
 * AST-based symbol extraction using Babel
 */
function extractSymbolsAST(source) {
    try {
        const ast = parser.parse(source, {
            sourceType: 'module',
            plugins: [
                'typescript',
                'jsx',
                'decorators-legacy',
                'classProperties'
            ]
        });

        const symbols = [];

        traverse(ast, {
            // Function declarations
            FunctionDeclaration(path) {
                const node = path.node;
                const loc = node.loc;

                symbols.push({
                    name: node.id ? node.id.name : '<anonymous>',
                    kind: node.async ? 'async_function' : 'function',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    },
                    documentation: extractLeadingComments(path)
                });
            },

            // Arrow functions assigned to variables
            VariableDeclarator(path) {
                const node = path.node;
                if (node.init &&
                    (node.init.type === 'ArrowFunctionExpression' ||
                     node.init.type === 'FunctionExpression')) {
                    const loc = node.loc;

                    symbols.push({
                        name: node.id.name,
                        kind: node.init.async ? 'async_function' : 'function',
                        location: {
                            start_line: loc.start.line,
                            start_column: loc.start.column,
                            end_line: loc.end.line,
                            end_column: loc.end.column
                        },
                        documentation: extractLeadingComments(path.parentPath)
                    });
                }
            },

            // Class declarations
            ClassDeclaration(path) {
                const node = path.node;
                const loc = node.loc;

                symbols.push({
                    name: node.id ? node.id.name : '<anonymous>',
                    kind: 'class',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    },
                    documentation: extractLeadingComments(path)
                });
            },

            // TypeScript interfaces
            TSInterfaceDeclaration(path) {
                const node = path.node;
                const loc = node.loc;

                symbols.push({
                    name: node.id.name,
                    kind: 'interface',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    },
                    documentation: extractLeadingComments(path)
                });
            },

            // TypeScript type aliases
            TSTypeAliasDeclaration(path) {
                const node = path.node;
                const loc = node.loc;

                symbols.push({
                    name: node.id.name,
                    kind: 'type_alias',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    },
                    documentation: extractLeadingComments(path)
                });
            },

            // TypeScript enums
            TSEnumDeclaration(path) {
                const node = path.node;
                const loc = node.loc;

                symbols.push({
                    name: node.id.name,
                    kind: 'enum',
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    },
                    documentation: extractLeadingComments(path)
                });
            }
        });

        return symbols;
    } catch (error) {
        // Fall back to regex if AST parsing fails
        return extractSymbolsRegex(source);
    }
}

/**
 * Extract leading comments as documentation
 */
function extractLeadingComments(path) {
    if (path.node.leadingComments && path.node.leadingComments.length > 0) {
        return path.node.leadingComments
            .map(comment => comment.value.trim())
            .join('\n');
    }
    return null;
}

/**
 * Regex-based symbol extraction (fallback)
 */
function extractSymbolsRegex(source) {
    const symbols = [];
    const lines = source.split('\n');

    const functionPattern = /^\s*(?:export\s+)?(?:async\s+)?function\s+([\w$]+)/;
    const arrowFunctionPattern = /^\s*(?:export\s+)?(?:const|let|var)\s+([\w$]+)\s*=\s*(?:async\s+)?\([^)]*\)\s*=>/;
    const classPattern = /^\s*(?:export\s+)?class\s+([\w$]+)/;
    const interfacePattern = /^\s*(?:export\s+)?interface\s+([\w$]+)/;
    const typePattern = /^\s*(?:export\s+)?type\s+([\w$]+)/;
    const enumPattern = /^\s*(?:export\s+)?enum\s+([\w$]+)/;

    lines.forEach((line, index) => {
        const lineNum = index + 1;

        let match = line.match(functionPattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: line.includes('async') ? 'async_function' : 'function',
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('function'),
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
            return;
        }

        match = line.match(arrowFunctionPattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: line.includes('async') ? 'async_function' : 'function',
                location: {
                    start_line: lineNum,
                    start_column: 0,
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
            return;
        }

        match = line.match(classPattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: 'class',
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('class'),
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
            return;
        }

        match = line.match(interfacePattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: 'interface',
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('interface'),
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
            return;
        }

        match = line.match(typePattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: 'type_alias',
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('type'),
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
            return;
        }

        match = line.match(enumPattern);
        if (match) {
            symbols.push({
                name: match[1],
                kind: 'enum',
                location: {
                    start_line: lineNum,
                    start_column: line.indexOf('enum'),
                    end_line: lineNum,
                    end_column: line.length
                },
                documentation: null
            });
        }
    });

    return symbols;
}

/**
 * AST-based deep symbol extraction using Babel for dead code analysis
 */
function extractSymbolsDeep(source) {
    if (!parser || !traverse) {
        // Babel is required for deep analysis
        console.error("Babel parser not available, cannot perform deep symbol extraction.");
        return [];
    }

    try {
        const ast = parser.parse(source, {
            sourceType: 'module',
            plugins: [
                'typescript',
                'jsx',
                'decorators-legacy',
                'classProperties'
            ]
        });

        const symbols = [];

        function isExported(path) {
            let parent = path.parentPath;
            if (parent.isExportNamedDeclaration() || parent.isExportDefaultDeclaration()) {
                return true;
            }
            // Also check for `export { symbol }`
            if (parent.isExportSpecifier() && parent.parentPath.isExportNamedDeclaration()) {
                return true;
            }

            return false;
        }

        const visitor = {
            'FunctionDeclaration|ClassDeclaration|TSInterfaceDeclaration|TSEnumDeclaration|TSTypeAliasDeclaration'(path) {
                const node = path.node;
                if (!node.id) return; // Skip anonymous declarations
                const loc = node.loc;

                let kind;
                switch (node.type) {
                    case 'FunctionDeclaration': kind = 'Function'; break;
                    case 'ClassDeclaration': kind = 'Struct'; break; // Mapped to align with Rust's SymbolKind
                    case 'TSInterfaceDeclaration': kind = 'Trait'; break; // Mapped to align with Rust's SymbolKind
                    case 'TSEnumDeclaration': kind = 'Enum'; break;
                    case 'TSTypeAliasDeclaration': kind = 'TypeAlias'; break;
                }

                symbols.push({
                    name: node.id.name,
                    kind: kind,
                    is_public: isExported(path),
                    location: {
                        start_line: loc.start.line,
                        start_column: loc.start.column,
                        end_line: loc.end.line,
                        end_column: loc.end.column
                    }
                });
            },

            VariableDeclarator(path) {
                const node = path.node;
                if (node.id.type === 'Identifier') {
                    const declaration = path.findParent((p) => p.isVariableDeclaration());
                    if (!declaration) return;

                    const loc = node.loc;

                    let kind;
                    if (declaration.node.kind === 'const') {
                        kind = 'Constant';
                    } else {
                        // For now, only 'const' is treated as a symbol for dead code analysis.
                        // 'let' and 'var' are considered too dynamic.
                        return;
                    }

                    symbols.push({
                        name: node.id.name,
                        kind: kind,
                        is_public: isExported(declaration),
                        location: {
                            start_line: loc.start.line,
                            start_column: loc.start.column,
                            end_line: loc.end.line,
                            end_column: loc.end.column
                        }
                    });
                }
            }
        };

        traverse(ast, visitor);

        return symbols;
    } catch (error) {
        // On parsing failure, return empty list. The Rust side will log this.
        return [];
    }
}


// CLI interface
const command = process.argv[2];

if (!command) {
    console.error('Usage: node ast_tool.js <command>');
    console.error('Commands:');
    console.error('  analyze-imports       Parse source from stdin and output import information as JSON');
    console.error('  extract-symbols       Parse source from stdin and output symbol information as JSON');
    console.error('  extract-symbols-deep  Parse source from stdin and output detailed symbols for dead code analysis');
    process.exit(1);
}

// Read source from stdin
const source = fs.readFileSync(0, 'utf-8');

if (command === 'analyze-imports') {
    const imports = analyzeImports(source);
    console.log(JSON.stringify(imports, null, 2));
} else if (command === 'extract-symbols') {
    const symbols = extractSymbols(source);
    console.log(JSON.stringify(symbols, null, 2));
} else if (command === 'extract-symbols-deep') {
    const symbols = extractSymbolsDeep(source);
    console.log(JSON.stringify(symbols, null, 2));
} else {
    console.error('Unknown command:', command);
    process.exit(1);
}