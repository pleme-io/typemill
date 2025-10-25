use mill_plugin_api::{ PluginError , PluginResult };
use mill_foundation::protocol::DependencyUpdate;
use std::path::Path;
use swc_common::{sync::Lrc, FileName, FilePathMapping, SourceMap};
use swc_ecma_ast::{ImportSpecifier, Module, ModuleDecl, ModuleItem};
use swc_ecma_codegen::{text_writer::JsWriter, Emitter};
use swc_ecma_parser::{lexer::Lexer, Parser, StringInput, Syntax, TsSyntax};
use tracing::debug;

/// Remove a named import from an import line using AST parsing.
/// This function is moved from `mill-ast/src/analyzer.rs`.
pub fn remove_named_import_from_line(line: &str, import_name: &str) -> PluginResult<String> {
    // Set up SWC parser
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Anon);
    let source_file = cm.new_source_file(file_name, line.to_string());

    // Parse the import line as TypeScript (most permissive syntax)
    let syntax = Syntax::Typescript(TsSyntax {
        tsx: true,
        decorators: true,
        ..Default::default()
    });

    let lexer = Lexer::new(
        syntax,
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);

    // Try to parse as a module
    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(_) => {
            // If parsing fails, return the original line unchanged
            return Ok(line.to_string());
        }
    };

    // Find the import declaration and filter out the specified import
    let mut modified = false;
    let new_items: Vec<ModuleItem> = module
        .body
        .into_iter()
        .filter_map(|item| {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(mut import_decl)) = item {
                // Filter out the import specifier matching import_name
                let original_len = import_decl.specifiers.len();
                import_decl.specifiers.retain(|spec| {
                    match spec {
                        ImportSpecifier::Named(named) => {
                            // Check both the local name and imported name
                            let local_name = named.local.sym.as_ref();
                            let imported_name =
                                named.imported.as_ref().map_or(local_name, |imp| match imp {
                                    swc_ecma_ast::ModuleExportName::Ident(ident) => {
                                        ident.sym.as_ref()
                                    }
                                    swc_ecma_ast::ModuleExportName::Str(s) => s.value.as_ref(),
                                });
                            local_name != import_name && imported_name != import_name
                        }
                        ImportSpecifier::Default(default) => {
                            default.local.sym.as_ref() != import_name
                        }
                        ImportSpecifier::Namespace(ns) => ns.local.sym.as_ref() != import_name,
                    }
                });

                // If we removed something, mark as modified
                if import_decl.specifiers.len() < original_len {
                    modified = true;
                }

                // If no specifiers left, remove the entire import
                if import_decl.specifiers.is_empty() {
                    return None;
                }

                return Some(ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)));
            }
            Some(item)
        })
        .collect();

    // If nothing was modified, return original line
    if !modified {
        return Ok(line.to_string());
    }

    // If no items left (entire import was removed), return empty string
    if new_items.is_empty() {
        return Ok(String::new());
    }

    // Emit the modified import
    let mut buf = vec![];
    {
        let new_module = Module {
            body: new_items,
            ..module
        };

        let mut emitter = Emitter {
            cfg: Default::default(),
            cm: cm.clone(),
            comments: None,
            wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
        };

        if emitter.emit_module(&new_module).is_err() {
            // If emission fails, return original line
            return Ok(line.to_string());
        }
    }

    Ok(String::from_utf8(buf)
        .unwrap_or_else(|_| line.to_string())
        .trim()
        .to_string())
}

/// Update an import reference in a file using AST-based transformation.
/// This logic is moved from `cb-services/src/services/import_service.rs`.
pub fn update_import_reference_ast(
    file_path: &Path,
    content: &str,
    update: &DependencyUpdate,
) -> PluginResult<String> {
    // Check if the file contains the old reference.
    if !content.contains(&update.old_reference) {
        return Ok(content.to_string());
    }

    // Set up SWC parser
    let cm = Lrc::new(SourceMap::new(FilePathMapping::empty()));
    let file_name = Lrc::new(FileName::Real(file_path.to_path_buf()));
    let source_file = cm.new_source_file(file_name, content.to_string());

    // Determine syntax based on file extension
    let syntax = match file_path.extension().and_then(|ext| ext.to_str()) {
        Some("ts") | Some("tsx") => Syntax::Typescript(TsSyntax {
            tsx: file_path.extension().and_then(|e| e.to_str()) == Some("tsx"),
            decorators: true,
            ..Default::default()
        }),
        _ => Syntax::Es(Default::default()),
    };

    // Parse the file
    let lexer = Lexer::new(
        syntax,
        Default::default(),
        StringInput::from(&*source_file),
        None,
    );
    let mut parser = Parser::new_from(lexer);

    let module = match parser.parse_module() {
        Ok(module) => module,
        Err(e) => {
            return Err(PluginError::parse(format!(
                "Failed to parse file for import update: {:?}",
                e
            )));
        }
    };

    // Transform imports
    let mut updated = false;
    let new_items: Vec<ModuleItem> = module
        .body
        .into_iter()
        .map(|item| {
            if let ModuleItem::ModuleDecl(ModuleDecl::Import(import_decl)) = &item {
                if import_decl.src.value.as_ref() == update.old_reference {
                    updated = true;
                    let mut new_import = import_decl.clone();
                    new_import.src = Box::new(swc_ecma_ast::Str {
                        span: import_decl.src.span,
                        value: update.new_reference.clone().into(),
                        raw: None,
                    });
                    return ModuleItem::ModuleDecl(ModuleDecl::Import(new_import));
                }
            }
            item
        })
        .collect();

    if !updated {
        debug!(
            file_path = %file_path.display(),
            old_ref = %update.old_reference,
            "No matching imports found to update in AST"
        );
        return Ok(content.to_string());
    }

    // Create new module with updated imports
    let new_module = swc_ecma_ast::Module {
        body: new_items,
        ..module
    };

    // Emit the updated code
    let mut buf = vec![];
    {
        let mut emitter = Emitter {
            cfg: Default::default(),
            cm: cm.clone(),
            comments: None,
            wr: JsWriter::new(cm.clone(), "\n", &mut buf, None),
        };

        emitter
            .emit_module(&new_module)
            .map_err(|e| PluginError::internal(format!("Failed to emit updated code: {:?}", e)))?;
    }

    String::from_utf8(buf).map_err(|e| {
        PluginError::internal(format!("Failed to convert emitted code to string: {}", e))
    })
}