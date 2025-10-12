// Force linker to include language plugins for inventory collection
// These extern crate declarations ensure the plugins are linked into any
// binary that depends on cb-services, allowing inventory to discover them
extern crate cb_lang_rust;
extern crate cb_lang_typescript;
extern crate cb_lang_markdown;

pub mod services;
