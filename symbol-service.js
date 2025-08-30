import { createRequire } from "node:module";
var __create = Object.create;
var __getProtoOf = Object.getPrototypeOf;
var __defProp = Object.defineProperty;
var __getOwnPropNames = Object.getOwnPropertyNames;
var __hasOwnProp = Object.prototype.hasOwnProperty;
var __toESM = (mod, isNodeMode, target) => {
  target = mod != null ? __create(__getProtoOf(mod)) : {};
  const to = isNodeMode || !mod || !mod.__esModule ? __defProp(target, "default", { value: mod, enumerable: true }) : target;
  for (let key of __getOwnPropNames(mod))
    if (!__hasOwnProp.call(to, key))
      __defProp(to, key, {
        get: () => mod[key],
        enumerable: true
      });
  return to;
};
var __commonJS = (cb, mod) => () => (mod || cb((mod = { exports: {} }).exports, mod), mod.exports);
var __export = (target, all) => {
  for (var name in all)
    __defProp(target, name, {
      get: all[name],
      enumerable: true,
      configurable: true,
      set: (newValue) => all[name] = () => newValue
    });
};
var __esm = (fn, res) => () => (fn && (res = fn(fn = 0)), res);
var __require = /* @__PURE__ */ createRequire(import.meta.url);

// node_modules/ignore/index.js
var require_ignore = __commonJS((exports, module) => {
  function makeArray(subject) {
    return Array.isArray(subject) ? subject : [subject];
  }
  var UNDEFINED = undefined;
  var EMPTY = "";
  var SPACE = " ";
  var ESCAPE = "\\";
  var REGEX_TEST_BLANK_LINE = /^\s+$/;
  var REGEX_INVALID_TRAILING_BACKSLASH = /(?:[^\\]|^)\\$/;
  var REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION = /^\\!/;
  var REGEX_REPLACE_LEADING_EXCAPED_HASH = /^\\#/;
  var REGEX_SPLITALL_CRLF = /\r?\n/g;
  var REGEX_TEST_INVALID_PATH = /^\.{0,2}\/|^\.{1,2}$/;
  var REGEX_TEST_TRAILING_SLASH = /\/$/;
  var SLASH = "/";
  var TMP_KEY_IGNORE = "node-ignore";
  if (typeof Symbol !== "undefined") {
    TMP_KEY_IGNORE = Symbol.for("node-ignore");
  }
  var KEY_IGNORE = TMP_KEY_IGNORE;
  var define = (object, key, value) => {
    Object.defineProperty(object, key, { value });
    return value;
  };
  var REGEX_REGEXP_RANGE = /([0-z])-([0-z])/g;
  var RETURN_FALSE = () => false;
  var sanitizeRange = (range) => range.replace(REGEX_REGEXP_RANGE, (match, from, to) => from.charCodeAt(0) <= to.charCodeAt(0) ? match : EMPTY);
  var cleanRangeBackSlash = (slashes) => {
    const { length } = slashes;
    return slashes.slice(0, length - length % 2);
  };
  var REPLACERS = [
    [
      /^\uFEFF/,
      () => EMPTY
    ],
    [
      /((?:\\\\)*?)(\\?\s+)$/,
      (_, m1, m2) => m1 + (m2.indexOf("\\") === 0 ? SPACE : EMPTY)
    ],
    [
      /(\\+?)\s/g,
      (_, m1) => {
        const { length } = m1;
        return m1.slice(0, length - length % 2) + SPACE;
      }
    ],
    [
      /[\\$.|*+(){^]/g,
      (match) => `\\${match}`
    ],
    [
      /(?!\\)\?/g,
      () => "[^/]"
    ],
    [
      /^\//,
      () => "^"
    ],
    [
      /\//g,
      () => "\\/"
    ],
    [
      /^\^*\\\*\\\*\\\//,
      () => "^(?:.*\\/)?"
    ],
    [
      /^(?=[^^])/,
      function startingReplacer() {
        return !/\/(?!$)/.test(this) ? "(?:^|\\/)" : "^";
      }
    ],
    [
      /\\\/\\\*\\\*(?=\\\/|$)/g,
      (_, index, str) => index + 6 < str.length ? "(?:\\/[^\\/]+)*" : "\\/.+"
    ],
    [
      /(^|[^\\]+)(\\\*)+(?=.+)/g,
      (_, p1, p2) => {
        const unescaped = p2.replace(/\\\*/g, "[^\\/]*");
        return p1 + unescaped;
      }
    ],
    [
      /\\\\\\(?=[$.|*+(){^])/g,
      () => ESCAPE
    ],
    [
      /\\\\/g,
      () => ESCAPE
    ],
    [
      /(\\)?\[([^\]/]*?)(\\*)($|\])/g,
      (match, leadEscape, range, endEscape, close) => leadEscape === ESCAPE ? `\\[${range}${cleanRangeBackSlash(endEscape)}${close}` : close === "]" ? endEscape.length % 2 === 0 ? `[${sanitizeRange(range)}${endEscape}]` : "[]" : "[]"
    ],
    [
      /(?:[^*])$/,
      (match) => /\/$/.test(match) ? `${match}$` : `${match}(?=$|\\/$)`
    ]
  ];
  var REGEX_REPLACE_TRAILING_WILDCARD = /(^|\\\/)?\\\*$/;
  var MODE_IGNORE = "regex";
  var MODE_CHECK_IGNORE = "checkRegex";
  var UNDERSCORE = "_";
  var TRAILING_WILD_CARD_REPLACERS = {
    [MODE_IGNORE](_, p1) {
      const prefix = p1 ? `${p1}[^/]+` : "[^/]*";
      return `${prefix}(?=$|\\/$)`;
    },
    [MODE_CHECK_IGNORE](_, p1) {
      const prefix = p1 ? `${p1}[^/]*` : "[^/]*";
      return `${prefix}(?=$|\\/$)`;
    }
  };
  var makeRegexPrefix = (pattern) => REPLACERS.reduce((prev, [matcher, replacer]) => prev.replace(matcher, replacer.bind(pattern)), pattern);
  var isString = (subject) => typeof subject === "string";
  var checkPattern = (pattern) => pattern && isString(pattern) && !REGEX_TEST_BLANK_LINE.test(pattern) && !REGEX_INVALID_TRAILING_BACKSLASH.test(pattern) && pattern.indexOf("#") !== 0;
  var splitPattern = (pattern) => pattern.split(REGEX_SPLITALL_CRLF).filter(Boolean);

  class IgnoreRule {
    constructor(pattern, mark, body, ignoreCase, negative, prefix) {
      this.pattern = pattern;
      this.mark = mark;
      this.negative = negative;
      define(this, "body", body);
      define(this, "ignoreCase", ignoreCase);
      define(this, "regexPrefix", prefix);
    }
    get regex() {
      const key = UNDERSCORE + MODE_IGNORE;
      if (this[key]) {
        return this[key];
      }
      return this._make(MODE_IGNORE, key);
    }
    get checkRegex() {
      const key = UNDERSCORE + MODE_CHECK_IGNORE;
      if (this[key]) {
        return this[key];
      }
      return this._make(MODE_CHECK_IGNORE, key);
    }
    _make(mode, key) {
      const str = this.regexPrefix.replace(REGEX_REPLACE_TRAILING_WILDCARD, TRAILING_WILD_CARD_REPLACERS[mode]);
      const regex = this.ignoreCase ? new RegExp(str, "i") : new RegExp(str);
      return define(this, key, regex);
    }
  }
  var createRule = ({
    pattern,
    mark
  }, ignoreCase) => {
    let negative = false;
    let body = pattern;
    if (body.indexOf("!") === 0) {
      negative = true;
      body = body.substr(1);
    }
    body = body.replace(REGEX_REPLACE_LEADING_EXCAPED_EXCLAMATION, "!").replace(REGEX_REPLACE_LEADING_EXCAPED_HASH, "#");
    const regexPrefix = makeRegexPrefix(body);
    return new IgnoreRule(pattern, mark, body, ignoreCase, negative, regexPrefix);
  };

  class RuleManager {
    constructor(ignoreCase) {
      this._ignoreCase = ignoreCase;
      this._rules = [];
    }
    _add(pattern) {
      if (pattern && pattern[KEY_IGNORE]) {
        this._rules = this._rules.concat(pattern._rules._rules);
        this._added = true;
        return;
      }
      if (isString(pattern)) {
        pattern = {
          pattern
        };
      }
      if (checkPattern(pattern.pattern)) {
        const rule = createRule(pattern, this._ignoreCase);
        this._added = true;
        this._rules.push(rule);
      }
    }
    add(pattern) {
      this._added = false;
      makeArray(isString(pattern) ? splitPattern(pattern) : pattern).forEach(this._add, this);
      return this._added;
    }
    test(path, checkUnignored, mode) {
      let ignored = false;
      let unignored = false;
      let matchedRule;
      this._rules.forEach((rule) => {
        const { negative } = rule;
        if (unignored === negative && ignored !== unignored || negative && !ignored && !unignored && !checkUnignored) {
          return;
        }
        const matched = rule[mode].test(path);
        if (!matched) {
          return;
        }
        ignored = !negative;
        unignored = negative;
        matchedRule = negative ? UNDEFINED : rule;
      });
      const ret = {
        ignored,
        unignored
      };
      if (matchedRule) {
        ret.rule = matchedRule;
      }
      return ret;
    }
  }
  var throwError = (message, Ctor) => {
    throw new Ctor(message);
  };
  var checkPath = (path, originalPath, doThrow) => {
    if (!isString(path)) {
      return doThrow(`path must be a string, but got \`${originalPath}\``, TypeError);
    }
    if (!path) {
      return doThrow(`path must not be empty`, TypeError);
    }
    if (checkPath.isNotRelative(path)) {
      const r = "`path.relative()`d";
      return doThrow(`path should be a ${r} string, but got "${originalPath}"`, RangeError);
    }
    return true;
  };
  var isNotRelative = (path) => REGEX_TEST_INVALID_PATH.test(path);
  checkPath.isNotRelative = isNotRelative;
  checkPath.convert = (p) => p;

  class Ignore {
    constructor({
      ignorecase = true,
      ignoreCase = ignorecase,
      allowRelativePaths = false
    } = {}) {
      define(this, KEY_IGNORE, true);
      this._rules = new RuleManager(ignoreCase);
      this._strictPathCheck = !allowRelativePaths;
      this._initCache();
    }
    _initCache() {
      this._ignoreCache = Object.create(null);
      this._testCache = Object.create(null);
    }
    add(pattern) {
      if (this._rules.add(pattern)) {
        this._initCache();
      }
      return this;
    }
    addPattern(pattern) {
      return this.add(pattern);
    }
    _test(originalPath, cache, checkUnignored, slices) {
      const path = originalPath && checkPath.convert(originalPath);
      checkPath(path, originalPath, this._strictPathCheck ? throwError : RETURN_FALSE);
      return this._t(path, cache, checkUnignored, slices);
    }
    checkIgnore(path) {
      if (!REGEX_TEST_TRAILING_SLASH.test(path)) {
        return this.test(path);
      }
      const slices = path.split(SLASH).filter(Boolean);
      slices.pop();
      if (slices.length) {
        const parent = this._t(slices.join(SLASH) + SLASH, this._testCache, true, slices);
        if (parent.ignored) {
          return parent;
        }
      }
      return this._rules.test(path, false, MODE_CHECK_IGNORE);
    }
    _t(path, cache, checkUnignored, slices) {
      if (path in cache) {
        return cache[path];
      }
      if (!slices) {
        slices = path.split(SLASH).filter(Boolean);
      }
      slices.pop();
      if (!slices.length) {
        return cache[path] = this._rules.test(path, checkUnignored, MODE_IGNORE);
      }
      const parent = this._t(slices.join(SLASH) + SLASH, cache, checkUnignored, slices);
      return cache[path] = parent.ignored ? parent : this._rules.test(path, checkUnignored, MODE_IGNORE);
    }
    ignores(path) {
      return this._test(path, this._ignoreCache, false).ignored;
    }
    createFilter() {
      return (path) => !this.ignores(path);
    }
    filter(paths) {
      return makeArray(paths).filter(this.createFilter());
    }
    test(path) {
      return this._test(path, this._testCache, true);
    }
  }
  var factory = (options) => new Ignore(options);
  var isPathValid = (path) => checkPath(path && checkPath.convert(path), path, RETURN_FALSE);
  var setupWindows = () => {
    const makePosix = (str) => /^\\\\\?\\/.test(str) || /["<>|\u0000-\u001F]+/u.test(str) ? str : str.replace(/\\/g, "/");
    checkPath.convert = makePosix;
    const REGEX_TEST_WINDOWS_PATH_ABSOLUTE = /^[a-z]:\//i;
    checkPath.isNotRelative = (path) => REGEX_TEST_WINDOWS_PATH_ABSOLUTE.test(path) || isNotRelative(path);
  };
  if (typeof process !== "undefined" && process.platform === "win32") {
    setupWindows();
  }
  module.exports = factory;
  factory.default = factory;
  module.exports.isPathValid = isPathValid;
  define(module.exports, Symbol.for("setupWindows"), setupWindows);
});

// src/file-scanner.ts
var exports_file_scanner = {};
__export(exports_file_scanner, {
  scanProjectFiles: () => scanProjectFiles,
  scanDirectoryForExtensions: () => scanDirectoryForExtensions,
  loadGitignore: () => loadGitignore
});
import { readFile, readdir, stat } from "node:fs/promises";
import { constants, access } from "node:fs/promises";
import { extname, join } from "node:path";
async function loadGitignore(projectPath) {
  const ig = import_ignore.default();
  ig.add(DEFAULT_IGNORE_PATTERNS);
  const gitignorePath = join(projectPath, ".gitignore");
  try {
    await access(gitignorePath, constants.F_OK);
    const gitignoreContent = await readFile(gitignorePath, "utf-8");
    ig.add(gitignoreContent);
  } catch (error) {}
  return ig;
}
async function scanDirectoryForExtensions(dirPath, maxDepth = 3, ignoreFilter, debug = false) {
  const extensions = new Set;
  async function scanDirectory(currentPath, currentDepth, relativePath = "") {
    if (currentDepth > maxDepth)
      return;
    try {
      const entries = await readdir(currentPath);
      if (debug) {
        process.stderr.write(`Scanning directory ${currentPath} (depth: ${currentDepth}), found ${entries.length} entries: ${entries.join(", ")}
`);
      }
      for (const entry of entries) {
        const fullPath = join(currentPath, entry);
        const entryRelativePath = relativePath ? join(relativePath, entry) : entry;
        const normalizedPath = entryRelativePath.replace(/\\/g, "/");
        if (ignoreFilter?.ignores(normalizedPath)) {
          if (debug) {
            process.stderr.write(`Skipping ignored entry: ${entryRelativePath}
`);
          }
          continue;
        }
        try {
          const fileStat = await stat(fullPath);
          if (fileStat.isDirectory()) {
            if (debug) {
              process.stderr.write(`Recursing into directory: ${entryRelativePath}
`);
            }
            await scanDirectory(fullPath, currentDepth + 1, entryRelativePath);
          } else if (fileStat.isFile()) {
            const ext = extname(entry).toLowerCase().slice(1);
            if (debug) {
              process.stderr.write(`Found file: ${entry}, extension: "${ext}"
`);
            }
            if (ext) {
              extensions.add(ext);
              if (debug) {
                process.stderr.write(`Added extension: ${ext}
`);
              }
            }
          }
        } catch (error) {
          const errorMsg = error instanceof Error ? error.message : String(error);
          process.stderr.write(`Error processing file ${fullPath} (stat/type check): ${errorMsg}
`);
        }
      }
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      process.stderr.write(`Error reading directory ${currentPath} (readdir operation): ${errorMsg}
`);
      return;
    }
  }
  await scanDirectory(dirPath, 0);
  return extensions;
}
function getRecommendedLanguageServers(extensions, languageServers) {
  const recommended = [];
  for (const server of languageServers) {
    const hasMatchingExtension = server.extensions.some((ext) => extensions.has(ext));
    if (hasMatchingExtension) {
      recommended.push(server.name);
    }
  }
  return recommended;
}
async function scanProjectFiles(projectPath, languageServers, maxDepth = 3, debug = false) {
  const ignoreFilter = await loadGitignore(projectPath);
  const extensions = await scanDirectoryForExtensions(projectPath, maxDepth, ignoreFilter, debug);
  const recommendedServers = getRecommendedLanguageServers(extensions, languageServers);
  return {
    extensions,
    recommendedServers
  };
}
var import_ignore, DEFAULT_IGNORE_PATTERNS;
var init_file_scanner = __esm(() => {
  import_ignore = __toESM(require_ignore(), 1);
  DEFAULT_IGNORE_PATTERNS = [
    "node_modules",
    ".git",
    ".svn",
    ".hg",
    "dist",
    "build",
    "out",
    "target",
    "bin",
    "obj",
    ".next",
    ".nuxt",
    "coverage",
    ".nyc_output",
    "temp",
    "cache",
    ".cache",
    ".vscode",
    ".idea",
    "*.log",
    ".DS_Store",
    "Thumbs.db"
  ];
});

// src/services/symbol-service.ts
import { readFileSync } from "node:fs";

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}

// src/services/symbol-service.ts
class SymbolService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async findDefinition(filePath, position) {
    process.stderr.write(`[DEBUG findDefinition] Requesting definition for ${filePath} at ${position.line}:${position.character}
`);
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    process.stderr.write(`[DEBUG findDefinition] Sending textDocument/definition request
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/definition", {
      textDocument: { uri: pathToUri(filePath) },
      position
    });
    process.stderr.write(`[DEBUG findDefinition] Result type: ${typeof result}, isArray: ${Array.isArray(result)}
`);
    if (Array.isArray(result)) {
      process.stderr.write(`[DEBUG findDefinition] Array result with ${result.length} locations
`);
      if (result.length > 0) {
        process.stderr.write(`[DEBUG findDefinition] First location: ${JSON.stringify(result[0], null, 2)}
`);
      }
      return result.map((loc) => ({
        uri: loc.uri,
        range: loc.range
      }));
    }
    if (result && typeof result === "object" && "uri" in result) {
      process.stderr.write(`[DEBUG findDefinition] Single location result: ${JSON.stringify(result, null, 2)}
`);
      const location = result;
      return [
        {
          uri: location.uri,
          range: location.range
        }
      ];
    }
    process.stderr.write(`[DEBUG findDefinition] No definition found or unexpected result format
`);
    return [];
  }
  async findReferences(filePath, position, includeDeclaration = false) {
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    process.stderr.write(`[DEBUG] findReferences for ${filePath} at ${position.line}:${position.character}, includeDeclaration: ${includeDeclaration}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/references", {
      textDocument: { uri: pathToUri(filePath) },
      position,
      context: { includeDeclaration }
    });
    process.stderr.write(`[DEBUG] findReferences result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (result && Array.isArray(result) && result.length > 0) {
      process.stderr.write(`[DEBUG] First reference: ${JSON.stringify(result[0], null, 2)}
`);
    } else if (result === null || result === undefined) {
      process.stderr.write(`[DEBUG] findReferences returned null/undefined
`);
    } else {
      process.stderr.write(`[DEBUG] findReferences returned unexpected result: ${JSON.stringify(result)}
`);
    }
    if (Array.isArray(result)) {
      return result.map((loc) => ({
        uri: loc.uri,
        range: loc.range
      }));
    }
    return [];
  }
  async renameSymbol(filePath, position, newName, dryRun = false) {
    process.stderr.write(`[DEBUG renameSymbol] Requesting rename for ${filePath} at ${position.line}:${position.character} to "${newName}", dryRun: ${dryRun}
`);
    if (dryRun) {
      process.stderr.write(`[DEBUG renameSymbol] Skipping LSP rename request for dry_run operation
`);
      return {
        changes: {
          [`file://${filePath}`]: [
            {
              range: { start: position, end: position },
              newText: "[DRY_RUN_PLACEHOLDER]"
            }
          ]
        }
      };
    }
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    process.stderr.write(`[DEBUG renameSymbol] Sending textDocument/rename request
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/rename", {
      textDocument: { uri: pathToUri(filePath) },
      position,
      newName
    });
    process.stderr.write(`[DEBUG renameSymbol] Result type: ${typeof result}, hasChanges: ${result && typeof result === "object" && "changes" in result}, hasDocumentChanges: ${result && typeof result === "object" && "documentChanges" in result}
`);
    if (result && typeof result === "object") {
      if ("changes" in result) {
        const workspaceEdit = result;
        const changeCount = Object.keys(workspaceEdit.changes || {}).length;
        process.stderr.write(`[DEBUG renameSymbol] WorkspaceEdit has changes for ${changeCount} files
`);
        return workspaceEdit;
      }
      if ("documentChanges" in result) {
        const workspaceEdit = result;
        process.stderr.write(`[DEBUG renameSymbol] WorkspaceEdit has documentChanges with ${workspaceEdit.documentChanges?.length || 0} entries
`);
        const changes = {};
        if (workspaceEdit.documentChanges) {
          for (const change of workspaceEdit.documentChanges) {
            if (change.textDocument && change.edits) {
              const uri = change.textDocument.uri;
              if (!changes[uri]) {
                changes[uri] = [];
              }
              changes[uri].push(...change.edits);
              process.stderr.write(`[DEBUG renameSymbol] Added ${change.edits.length} edits for ${uri}
`);
            }
          }
        }
        return { changes };
      }
    }
    process.stderr.write(`[DEBUG renameSymbol] No rename changes available
`);
    return {};
  }
  async searchWorkspaceSymbols(query, servers, preloadServers) {
    if (servers.size === 0) {
      process.stderr.write(`[DEBUG searchWorkspaceSymbols] No servers running, preloading servers first
`);
      await preloadServers(false);
    }
    let hasOpenFiles = false;
    for (const serverState of servers.values()) {
      if (serverState.openFiles.size > 0) {
        hasOpenFiles = true;
        break;
      }
    }
    if (!hasOpenFiles) {
      try {
        const { scanDirectoryForExtensions: scanDirectoryForExtensions2, loadGitignore: loadGitignore2 } = await Promise.resolve().then(() => (init_file_scanner(), exports_file_scanner));
        const gitignore = await loadGitignore2(process.cwd());
        const extensions = await scanDirectoryForExtensions2(process.cwd(), 2, gitignore, false);
        if (extensions.has("ts")) {
          const fs = await import("node:fs/promises");
          const path = await import("node:path");
          async function findTsFile(dir) {
            try {
              const entries = await fs.readdir(dir, { withFileTypes: true });
              for (const entry of entries) {
                if (entry.isFile() && entry.name.endsWith(".ts")) {
                  return path.join(dir, entry.name);
                }
                if (entry.isDirectory() && !entry.name.startsWith(".")) {
                  const found = await findTsFile(path.join(dir, entry.name));
                  if (found)
                    return found;
                }
              }
            } catch {}
            return null;
          }
          const tsFile = await findTsFile(process.cwd());
          if (tsFile) {
            process.stderr.write(`[DEBUG searchWorkspaceSymbols] Opening ${tsFile} to establish project context
`);
            const serverState = await this.getServer(tsFile);
            await this.ensureFileOpen(serverState, tsFile);
          }
        }
      } catch (error) {
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Failed to establish project context: ${error}
`);
      }
    }
    const results = [];
    process.stderr.write(`[DEBUG searchWorkspaceSymbols] Searching for "${query}" across ${servers.size} servers
`);
    for (const [serverKey, serverState] of servers.entries()) {
      process.stderr.write(`[DEBUG searchWorkspaceSymbols] Checking server: ${serverKey}, initialized: ${serverState.initialized}
`);
      if (!serverState.initialized)
        continue;
      try {
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Sending workspace/symbol request for "${query}"
`);
        const result = await this.protocol.sendRequest(serverState.process, "workspace/symbol", {
          query
        });
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Workspace symbol result: ${JSON.stringify(result)}
`);
        if (Array.isArray(result)) {
          results.push(...result);
          process.stderr.write(`[DEBUG searchWorkspaceSymbols] Added ${result.length} symbols from server
`);
        } else if (result !== null && result !== undefined) {
          process.stderr.write(`[DEBUG searchWorkspaceSymbols] Non-array result: ${typeof result}
`);
        }
      } catch (error) {
        process.stderr.write(`[DEBUG searchWorkspaceSymbols] Server error: ${error}
`);
      }
    }
    process.stderr.write(`[DEBUG searchWorkspaceSymbols] Total results found: ${results.length}
`);
    return results;
  }
  async getDocumentSymbols(filePath) {
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    process.stderr.write(`[DEBUG] Requesting documentSymbol for: ${filePath}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/documentSymbol", {
      textDocument: { uri: pathToUri(filePath) }
    });
    process.stderr.write(`[DEBUG] documentSymbol result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (result && Array.isArray(result) && result.length > 0) {
      process.stderr.write(`[DEBUG] First symbol: ${JSON.stringify(result[0], null, 2)}
`);
    } else if (result === null || result === undefined) {
      process.stderr.write(`[DEBUG] documentSymbol returned null/undefined
`);
    } else {
      process.stderr.write(`[DEBUG] documentSymbol returned unexpected result: ${JSON.stringify(result)}
`);
    }
    if (Array.isArray(result)) {
      return result;
    }
    return [];
  }
  async findSymbolMatches(filePath, symbolName, symbolKind) {
    try {
      const symbols = await this.getDocumentSymbols(filePath);
      const matches = [];
      if (this.isDocumentSymbolArray(symbols)) {
        const flatSymbols = this.flattenDocumentSymbols(symbols);
        for (const symbol of flatSymbols) {
          if (symbol.name === symbolName) {
            if (!symbolKind || this.symbolKindToString(symbol.kind) === symbolKind.toLowerCase()) {
              matches.push({
                name: symbol.name,
                kind: symbol.kind,
                position: symbol.selectionRange.start,
                range: symbol.range,
                detail: symbol.detail
              });
            }
          }
        }
      } else {
        for (const symbol of symbols) {
          if (symbol.name === symbolName) {
            if (!symbolKind || this.symbolKindToString(symbol.kind) === symbolKind.toLowerCase()) {
              const position = await this.findSymbolPositionInFile(filePath, symbol);
              matches.push({
                name: symbol.name,
                kind: symbol.kind,
                position,
                range: symbol.location.range,
                detail: undefined
              });
            }
          }
        }
      }
      return matches;
    } catch (error) {
      process.stderr.write(`[ERROR findSymbolMatches] ${error}
`);
      return [];
    }
  }
  flattenDocumentSymbols(symbols) {
    const flattened = [];
    for (const symbol of symbols) {
      flattened.push(symbol);
      if (symbol.children) {
        flattened.push(...this.flattenDocumentSymbols(symbol.children));
      }
    }
    return flattened;
  }
  isDocumentSymbolArray(symbols) {
    if (symbols.length === 0)
      return true;
    const firstSymbol = symbols[0];
    if (!firstSymbol)
      return true;
    return "range" in firstSymbol && "selectionRange" in firstSymbol;
  }
  symbolKindToString(kind) {
    const kindMap = {
      [1 /* File */]: "file",
      [2 /* Module */]: "module",
      [3 /* Namespace */]: "namespace",
      [4 /* Package */]: "package",
      [5 /* Class */]: "class",
      [6 /* Method */]: "method",
      [7 /* Property */]: "property",
      [8 /* Field */]: "field",
      [9 /* Constructor */]: "constructor",
      [10 /* Enum */]: "enum",
      [11 /* Interface */]: "interface",
      [12 /* Function */]: "function",
      [13 /* Variable */]: "variable",
      [14 /* Constant */]: "constant",
      [15 /* String */]: "string",
      [16 /* Number */]: "number",
      [17 /* Boolean */]: "boolean",
      [18 /* Array */]: "array",
      [19 /* Object */]: "object",
      [20 /* Key */]: "key",
      [21 /* Null */]: "null",
      [22 /* EnumMember */]: "enum_member",
      [23 /* Struct */]: "struct",
      [24 /* Event */]: "event",
      [25 /* Operator */]: "operator",
      [26 /* TypeParameter */]: "type_parameter"
    };
    return kindMap[kind] || "unknown";
  }
  getValidSymbolKinds() {
    return [
      "file",
      "module",
      "namespace",
      "package",
      "class",
      "method",
      "property",
      "field",
      "constructor",
      "enum",
      "interface",
      "function",
      "variable",
      "constant",
      "string",
      "number",
      "boolean",
      "array",
      "object",
      "key",
      "null",
      "enum_member",
      "struct",
      "event",
      "operator",
      "type_parameter"
    ];
  }
  async findSymbolPositionInFile(filePath, symbol) {
    try {
      const fileContent = readFileSync(filePath, "utf-8");
      const lines = fileContent.split(`
`);
      const range = symbol.location.range;
      const startLine = range.start.line;
      const endLine = range.end.line;
      for (let lineNum = startLine;lineNum <= endLine && lineNum < lines.length; lineNum++) {
        const line = lines[lineNum];
        if (!line)
          continue;
        let searchStart = 0;
        if (lineNum === startLine) {
          searchStart = range.start.character;
        }
        let searchEnd = line.length;
        if (lineNum === endLine) {
          searchEnd = range.end.character;
        }
        const searchText = line.substring(searchStart, searchEnd);
        const symbolIndex = searchText.indexOf(symbol.name);
        if (symbolIndex !== -1) {
          const actualCharacter = searchStart + symbolIndex;
          return {
            line: lineNum,
            character: actualCharacter
          };
        }
      }
      return range.start;
    } catch (error) {
      return symbol.location.range.start;
    }
  }
  stringToSymbolKind(kindStr) {
    const kindMap = {
      file: 1 /* File */,
      module: 2 /* Module */,
      namespace: 3 /* Namespace */,
      package: 4 /* Package */,
      class: 5 /* Class */,
      method: 6 /* Method */,
      property: 7 /* Property */,
      field: 8 /* Field */,
      constructor: 9 /* Constructor */,
      enum: 10 /* Enum */,
      interface: 11 /* Interface */,
      function: 12 /* Function */,
      variable: 13 /* Variable */,
      constant: 14 /* Constant */,
      string: 15 /* String */,
      number: 16 /* Number */,
      boolean: 17 /* Boolean */,
      array: 18 /* Array */
    };
    return kindMap[kindStr.toLowerCase()] || null;
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync(filePath, "utf-8");
      this.protocol.sendNotification(serverState.process, "textDocument/didOpen", {
        textDocument: {
          uri: `file://${filePath}`,
          languageId: this.getLanguageId(filePath),
          version: 1,
          text: fileContent
        }
      });
      serverState.openFiles.add(filePath);
    } catch (error) {
      throw new Error(`Failed to open file for LSP server: ${filePath} - ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  getLanguageId(filePath) {
    const ext = filePath.split(".").pop()?.toLowerCase();
    const languageMap = {
      ts: "typescript",
      tsx: "typescriptreact",
      js: "javascript",
      jsx: "javascriptreact",
      py: "python",
      go: "go",
      rs: "rust",
      java: "java",
      cpp: "cpp",
      c: "c",
      h: "c",
      hpp: "cpp"
    };
    return languageMap[ext || ""] || "plaintext";
  }
}
export {
  SymbolService
};
