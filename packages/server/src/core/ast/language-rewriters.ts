import { extname } from 'node:path';

/**
 * Result of rewriting imports in a file
 */
export interface RewriteResult {
  success: boolean;
  content?: string;
  error?: string;
  editsApplied: number;
}

/**
 * Mapping from old import path to new import path
 */
export interface ImportMapping {
  oldPath: string;
  newPath: string;
}

/**
 * Rewrite Python imports based on path mappings
 */
function rewritePythonImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Convert path to Python module format (dots instead of slashes)
    const oldModule = oldPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.py$/, '');
    const newModule = newPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.py$/, '');

    // Pattern 1: import module
    const importRegex = new RegExp(`^(\\s*import\\s+)${escapeRegex(oldModule)}(\\s|$)`, 'gm');
    const importReplacement = `$1${newModule}$2`;
    const beforeImport = modifiedContent;
    modifiedContent = modifiedContent.replace(importRegex, importReplacement);
    if (modifiedContent !== beforeImport) editsApplied++;

    // Pattern 2: from module import ...
    const fromRegex = new RegExp(`^(\\s*from\\s+)${escapeRegex(oldModule)}(\\s+import)`, 'gm');
    const fromReplacement = `$1${newModule}$2`;
    const beforeFrom = modifiedContent;
    modifiedContent = modifiedContent.replace(fromRegex, fromReplacement);
    if (modifiedContent !== beforeFrom) editsApplied++;

    // Handle relative imports (. and ..)
    if (oldPath.startsWith('.')) {
      const relativeOldModule = oldPath.replace(/\//g, '.').replace(/\.py$/, '');
      const relativeNewModule = newPath.replace(/\//g, '.').replace(/\.py$/, '');

      const relFromRegex = new RegExp(`^(\\s*from\\s+)${escapeRegex(relativeOldModule)}(\\s+import)`, 'gm');
      const relFromReplacement = `$1${relativeNewModule}$2`;
      const beforeRelFrom = modifiedContent;
      modifiedContent = modifiedContent.replace(relFromRegex, relFromReplacement);
      if (modifiedContent !== beforeRelFrom) editsApplied++;
    }
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite Go imports based on path mappings
 */
function rewriteGoImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Go uses full paths in quotes
    const oldImport = oldPath.replace(/^\.\//, '');
    const newImport = newPath.replace(/^\.\//, '');

    // Single import: import "path"
    const singleRegex = new RegExp(`^(\\s*import\\s+")${escapeRegex(oldImport)}(")`, 'gm');
    const singleReplacement = `$1${newImport}$2`;
    const beforeSingle = modifiedContent;
    modifiedContent = modifiedContent.replace(singleRegex, singleReplacement);
    if (modifiedContent !== beforeSingle) editsApplied++;

    // Multiple imports block: import ( "path" )
    const blockRegex = new RegExp(`(^\\s*")${escapeRegex(oldImport)}("\\s*$)`, 'gm');
    const blockReplacement = `$1${newImport}$2`;
    const beforeBlock = modifiedContent;
    modifiedContent = modifiedContent.replace(blockRegex, blockReplacement);
    if (modifiedContent !== beforeBlock) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite Rust imports based on path mappings
 */
function rewriteRustImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Convert path to Rust module format (:: instead of /)
    const oldModule = oldPath.replace(/^\.\//, '').replace(/\//g, '::').replace(/\.rs$/, '');
    const newModule = newPath.replace(/^\.\//, '').replace(/\//g, '::').replace(/\.rs$/, '');

    // use statements
    const useRegex = new RegExp(`^(\\s*use\\s+)${escapeRegex(oldModule)}(::.*)?;`, 'gm');
    const useReplacement = `$1${newModule}$2;`;
    const beforeUse = modifiedContent;
    modifiedContent = modifiedContent.replace(useRegex, useReplacement);
    if (modifiedContent !== beforeUse) editsApplied++;

    // mod statements
    const modRegex = new RegExp(`^(\\s*mod\\s+)${escapeRegex(oldModule)}(;|\\s*\\{)`, 'gm');
    const modReplacement = `$1${newModule}$2`;
    const beforeMod = modifiedContent;
    modifiedContent = modifiedContent.replace(modRegex, modReplacement);
    if (modifiedContent !== beforeMod) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite Java imports based on path mappings
 */
function rewriteJavaImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Convert path to Java package format (dots instead of slashes)
    const oldPackage = oldPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.java$/, '');
    const newPackage = newPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.java$/, '');

    // import statements
    const importRegex = new RegExp(`^(\\s*import\\s+(?:static\\s+)?)${escapeRegex(oldPackage)}((?:\\.[A-Z][\\w]*)?(?:\\.\\*)?;)`, 'gm');
    const importReplacement = `$1${newPackage}$2`;
    const beforeImport = modifiedContent;
    modifiedContent = modifiedContent.replace(importRegex, importReplacement);
    if (modifiedContent !== beforeImport) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite C# using directives based on path mappings
 */
function rewriteCSharpImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Convert path to C# namespace format (dots instead of slashes)
    const oldNamespace = oldPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.cs$/, '');
    const newNamespace = newPath.replace(/^\.\//, '').replace(/\//g, '.').replace(/\.cs$/, '');

    // using statements
    const usingRegex = new RegExp(`^(\\s*using\\s+)${escapeRegex(oldNamespace)}(;|\\s*=)`, 'gm');
    const usingReplacement = `$1${newNamespace}$2`;
    const beforeUsing = modifiedContent;
    modifiedContent = modifiedContent.replace(usingRegex, usingReplacement);
    if (modifiedContent !== beforeUsing) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite Ruby requires based on path mappings
 */
function rewriteRubyImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // Ruby uses string paths
    const oldRequire = oldPath.replace(/\.rb$/, '');
    const newRequire = newPath.replace(/\.rb$/, '');

    // require and require_relative
    const requireRegex = new RegExp(`^(\\s*require(?:_relative)?\\s+['"]\)${escapeRegex(oldRequire)}(['"])`, 'gm');
    const requireReplacement = `$1${newRequire}$2`;
    const beforeRequire = modifiedContent;
    modifiedContent = modifiedContent.replace(requireRegex, requireReplacement);
    if (modifiedContent !== beforeRequire) editsApplied++;

    // load statements
    const loadRegex = new RegExp(`^(\\s*load\\s+['"]\)${escapeRegex(oldPath)}(['"])`, 'gm');
    const loadReplacement = `$1${newPath}$2`;
    const beforeLoad = modifiedContent;
    modifiedContent = modifiedContent.replace(loadRegex, loadReplacement);
    if (modifiedContent !== beforeLoad) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Rewrite PHP imports based on path mappings
 */
function rewritePhpImports(content: string, mappings: ImportMapping[]): RewriteResult {
  let modifiedContent = content;
  let editsApplied = 0;

  for (const { oldPath, newPath } of mappings) {
    // PHP namespace format (backslashes)
    const oldNamespace = oldPath.replace(/^\.\//, '').replace(/\//g, '\\\\').replace(/\.php$/, '');
    const newNamespace = newPath.replace(/^\.\//, '').replace(/\//g, '\\\\').replace(/\.php$/, '');

    // use statements
    const useRegex = new RegExp(`^(\\s*use\\s+)${escapeRegex(oldNamespace)}(;|\\s+as)`, 'gm');
    const useReplacement = `$1${newNamespace}$2`;
    const beforeUse = modifiedContent;
    modifiedContent = modifiedContent.replace(useRegex, useReplacement);
    if (modifiedContent !== beforeUse) editsApplied++;

    // require/include with paths
    const requireRegex = new RegExp(`((?:require|include)(?:_once)?\\s*\\(?['"]\)${escapeRegex(oldPath)}(['"])`, 'gm');
    const requireReplacement = `$1${newPath}$2`;
    const beforeRequire = modifiedContent;
    modifiedContent = modifiedContent.replace(requireRegex, requireReplacement);
    if (modifiedContent !== beforeRequire) editsApplied++;
  }

  return {
    success: true,
    content: modifiedContent,
    editsApplied
  };
}

/**
 * Escape special regex characters
 */
function escapeRegex(str: string): string {
  return str.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

/**
 * Main dispatcher for rewriting imports based on file type
 *
 * @param filePath The path to the file being edited
 * @param content The current content of the file
 * @param mappings Array of import path mappings to apply
 * @returns Result with the transformed content or error
 */
export function rewriteImports(
  filePath: string,
  content: string,
  mappings: ImportMapping[]
): RewriteResult {
  const ext = extname(filePath).toLowerCase();

  switch (ext) {
    case '.py':
    case '.pyw':
      return rewritePythonImports(content, mappings);

    case '.go':
      return rewriteGoImports(content, mappings);

    case '.rs':
      return rewriteRustImports(content, mappings);

    case '.java':
      return rewriteJavaImports(content, mappings);

    case '.cs':
      return rewriteCSharpImports(content, mappings);

    case '.rb':
      return rewriteRubyImports(content, mappings);

    case '.php':
      return rewritePhpImports(content, mappings);

    default:
      return {
        success: false,
        error: `No import rewriter available for ${ext} files`,
        editsApplied: 0
      };
  }
}