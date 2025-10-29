# PHP Language Support

## Problem

PHP developers (Rank 10 language, WordPress, Laravel, legacy web apps) cannot use TypeMill. No LSP integration or language plugin exists for PHP projects.

## Solution

Implement full PHP support with `intelephense` LSP integration and `mill-lang-php` plugin. Use tree-sitter-php for AST parsing and support Composer package manager.

### Technical Approach

- **LSP Server**: `intelephense` (recommended) or `phpactor` (alternative)
- **AST Parser**: `tree-sitter-php`
- **Package Manager**: Composer (`composer.json`)
- **Import Syntax**: `use`, `require`, `include`, `require_once`, `include_once`

## Checklists

### LSP Integration
- [ ] Add `intelephense` to default LSP server configurations in `mill setup`
- [ ] Document installation (`npm install -g intelephense`)
- [ ] Configure alternative `phpactor` option
- [ ] Configure file extensions (`.php`, `.phtml`)
- [ ] Test initialization and basic navigation
- [ ] Test with Laravel framework projects
- [ ] Test with Symfony framework projects
- [ ] Verify namespace navigation and autocompletion
- [ ] Test PSR-4 autoloading support

### Language Plugin (`crates/mill-lang-php`)
- [ ] Create crate structure following `mill-lang-*` pattern
- [ ] Add to `languages.toml` registry:
  ```toml
  [languages.php]
  path = "crates/mill-lang-php"
  plugin_struct = "PhpPlugin"
  category = "full"
  default = false
  ```
- [ ] Run `cargo xtask sync-languages` to generate feature flags
- [ ] Implement `LanguagePlugin` trait with `define_language_plugin!` macro
- [ ] Set metadata (name: "PHP", extensions: `["php", "phtml"]`)
- [ ] Configure LSP: `LspConfig::new("intelephense", &["intelephense", "--stdio"])`

### AST Parsing
- [ ] Integrate `tree-sitter-php` dependency
- [ ] Implement `parse()` method for symbol extraction
- [ ] Parse classes, functions, methods, interfaces, traits
- [ ] Parse namespaces and nested namespaces
- [ ] Extract symbol hierarchy (class members, properties, constants)
- [ ] Handle PHP-specific syntax (magic methods, closures)

### Import Support (5 Traits)
- [ ] Implement `ImportParser` trait
  - [ ] Parse `use` statements (classes, functions, constants)
  - [ ] Parse `require` and `include` statements
  - [ ] Parse `require_once` and `include_once`
  - [ ] Handle aliased imports (`use Foo\Bar as Baz`)
- [ ] Implement `ImportRenameSupport` trait
- [ ] Implement `ImportMoveSupport` trait
- [ ] Implement `ImportMutationSupport` trait
- [ ] Implement `ImportAdvancedSupport` trait

### Manifest Parsing
- [ ] Implement `analyze_manifest()` for `composer.json`
  - [ ] Parse `require` dependencies
  - [ ] Parse `require-dev` dev dependencies
  - [ ] Parse `autoload` configuration (PSR-4, PSR-0)
  - [ ] Parse `autoload-dev` configuration
  - [ ] Parse classmap and files autoload
  - [ ] Extract package metadata (name, version, description)
- [ ] Implement manifest update capabilities
  - [ ] Add dependencies to `require`
  - [ ] Update PSR-4 namespace mappings
  - [ ] Modify autoload configuration

### Framework Integration
- [ ] Test with Laravel projects
  - [ ] Service providers navigation
  - [ ] Facade usage detection
  - [ ] Eloquent model relationships
- [ ] Test with Symfony projects
  - [ ] Service container navigation
  - [ ] Bundle detection
  - [ ] Annotation/attribute support
- [ ] Test with WordPress
  - [ ] Plugin/theme structure
  - [ ] Hook system navigation

### Testing
- [ ] Unit tests for AST parsing (classes, traits, namespaces)
- [ ] Unit tests for `use`, `require`, `include` parsing
- [ ] Integration tests with Composer projects
- [ ] Manifest parsing tests (`composer.json`)
- [ ] PSR-4 autoload resolution tests
- [ ] LSP integration tests with `intelephense`
- [ ] Test with Laravel framework
- [ ] Test with Symfony framework
- [ ] Test with plain PHP projects

### Documentation
- [ ] Update `docs/architecture/overview.md` language support matrix
- [ ] Add PHP examples to `docs/tools/` documentation
- [ ] Document `intelephense` installation and configuration
- [ ] Document Composer integration
- [ ] Note framework-specific considerations (Laravel, Symfony)
- [ ] Create PHP plugin development guide
- [ ] Document PSR-4 autoloading behavior

## Success Criteria

- [ ] `cargo check -p mill-lang-php` compiles without errors
- [ ] `cargo check --workspace --features lang-php` compiles
- [ ] All unit tests pass for AST and manifest parsing
- [ ] Integration tests pass with Composer projects
- [ ] Plugin loads via `define_language_plugin!` macro
- [ ] LSP integration works with `intelephense`
- [ ] Can navigate PHP codebases (find definition, references)
- [ ] Can parse `composer.json` manifests
- [ ] Import rewriting works for `use` statements
- [ ] PSR-4 autoloading resolution works
- [ ] Framework integration works (Laravel, Symfony)

## Benefits

- **PHP developers** can use TypeMill for web applications
- **Legacy codebases** get modern code intelligence (WordPress, Drupal)
- **Framework support** enables Laravel/Symfony navigation
- **Composer integration** tracks dependencies and autoloading
- **Large market** covers significant web development segment
- **Market coverage** reaches 90%+ (completes top 10 languages)