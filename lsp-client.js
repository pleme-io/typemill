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

// src/capability-manager.ts
class CapabilityManager {
  capabilityCache = new Map;
  cacheCapabilities(serverKey, initResult) {
    if (initResult && typeof initResult === "object" && "capabilities" in initResult) {
      this.capabilityCache.set(serverKey, initResult.capabilities);
      process.stderr.write(`[DEBUG CapabilityManager] Cached capabilities for ${serverKey}
`);
    } else {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found in init result for ${serverKey}
`);
    }
  }
  getCapabilities(serverKeyOrState) {
    if (typeof serverKeyOrState === "string") {
      return this.capabilityCache.get(serverKeyOrState) || null;
    }
    const serverKey = this.getServerKey(serverKeyOrState);
    return this.capabilityCache.get(serverKey) || serverKeyOrState.capabilities;
  }
  hasCapability(serverState, capabilityPath) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server
`);
      return false;
    }
    const pathParts = capabilityPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} not found
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${capabilityPath} has unexpected type: ${typeof current}
`);
    return false;
  }
  checkCapability(serverKey, capabilityPath, subCapability) {
    const capabilities = this.getCapabilities(serverKey);
    if (!capabilities) {
      process.stderr.write(`[DEBUG CapabilityManager] No capabilities found for server ${serverKey}
`);
      return false;
    }
    let fullPath = capabilityPath;
    if (subCapability) {
      fullPath = `${capabilityPath}.${subCapability}`;
    }
    const pathParts = fullPath.split(".");
    let current = capabilities;
    for (const part of pathParts) {
      if (current && typeof current === "object" && part in current) {
        current = current[part];
      } else {
        process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} not found for server ${serverKey}
`);
        return false;
      }
    }
    if (typeof current === "boolean") {
      return current;
    }
    if (current && typeof current === "object") {
      return true;
    }
    process.stderr.write(`[DEBUG CapabilityManager] Capability ${fullPath} has unexpected type: ${typeof current} for server ${serverKey}
`);
    return false;
  }
  getSignatureHelpTriggers(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (capabilities?.signatureHelpProvider?.triggerCharacters) {
      return capabilities.signatureHelpProvider.triggerCharacters;
    }
    return ["(", ","];
  }
  supportsAdvancedWorkspaceEdit(serverState) {
    return this.hasCapability(serverState, "workspace.workspaceEdit.documentChanges");
  }
  supportsFileOperations(serverState) {
    return this.hasCapability(serverState, "workspace.fileOperations");
  }
  getCapabilityInfo(serverState) {
    const capabilities = this.getCapabilities(serverState);
    if (!capabilities) {
      return "No capabilities available";
    }
    const supportedFeatures = [
      "hoverProvider",
      "signatureHelpProvider",
      "definitionProvider",
      "referencesProvider",
      "documentSymbolProvider",
      "workspaceSymbolProvider",
      "codeActionProvider",
      "documentLinkProvider",
      "documentFormattingProvider",
      "renameProvider",
      "foldingRangeProvider",
      "selectionRangeProvider",
      "callHierarchyProvider",
      "semanticTokensProvider",
      "typeHierarchyProvider",
      "inlayHintProvider"
    ].filter((feature) => {
      const value = capabilities[feature];
      return Boolean(value);
    });
    const workspaceFeatures = [];
    if (capabilities.workspace) {
      if (capabilities.workspace.workspaceEdit)
        workspaceFeatures.push("workspaceEdit");
      if (capabilities.workspace.fileOperations)
        workspaceFeatures.push("fileOperations");
      if (capabilities.workspace.workspaceFolders)
        workspaceFeatures.push("workspaceFolders");
    }
    return `Supported features: ${supportedFeatures.join(", ")}
Workspace features: ${workspaceFeatures.join(", ")}`;
  }
  getServerKey(serverState) {
    if (serverState.config?.command) {
      return JSON.stringify(serverState.config.command);
    }
    return "unknown-server";
  }
  validateRequiredCapabilities(serverState, requiredCapabilities) {
    const missing = [];
    for (const capability of requiredCapabilities) {
      if (!this.hasCapability(serverState, capability)) {
        missing.push(capability);
      }
    }
    return {
      supported: missing.length === 0,
      missing
    };
  }
  getServerDescription(serverState) {
    if (serverState.config?.command) {
      const command = serverState.config.command;
      if (Array.isArray(command) && command.length > 0) {
        const serverName = command[0];
        if (serverName?.includes("typescript-language-server"))
          return "TypeScript";
        if (serverName?.includes("pylsp"))
          return "Python (pylsp)";
        if (serverName?.includes("gopls"))
          return "Go (gopls)";
        if (serverName?.includes("rust-analyzer"))
          return "Rust (rust-analyzer)";
        return serverName || "Unknown Server";
      }
      return String(command);
    }
    return "Unknown Server";
  }
}
var capabilityManager = new CapabilityManager;

// src/lsp/client.ts
import { existsSync, readFileSync } from "node:fs";

// src/default-config.ts
var DEFAULT_SERVERS = [
  {
    extensions: ["ts", "tsx", "js", "jsx", "mjs", "cjs"],
    command: ["npx", "--", "typescript-language-server", "--stdio"]
  },
  {
    extensions: ["py", "pyi"],
    command: ["pylsp"]
  },
  {
    extensions: ["go"],
    command: ["gopls"]
  },
  {
    extensions: ["rs"],
    command: ["rust-analyzer"]
  },
  {
    extensions: ["json", "jsonc"],
    command: ["npx", "--", "vscode-json-languageserver", "--stdio"]
  },
  {
    extensions: ["html", "htm"],
    command: ["npx", "--", "vscode-html-languageserver", "--stdio"]
  },
  {
    extensions: ["css", "scss", "sass", "less"],
    command: ["npx", "--", "vscode-css-languageserver", "--stdio"]
  },
  {
    extensions: ["vue"],
    command: ["npx", "--", "vue-language-server", "--stdio"]
  },
  {
    extensions: ["svelte"],
    command: ["npx", "--", "svelteserver", "--stdio"]
  },
  {
    extensions: ["c", "cpp", "cc", "cxx", "h", "hpp"],
    command: ["clangd"]
  },
  {
    extensions: ["java"],
    command: ["jdtls"]
  },
  {
    extensions: ["rb", "ruby"],
    command: ["solargraph", "stdio"]
  },
  {
    extensions: ["php"],
    command: ["intelephense", "--stdio"]
  },
  {
    extensions: ["sh", "bash", "zsh"],
    command: ["npx", "--", "bash-language-server", "start"]
  },
  {
    extensions: ["dockerfile", "Dockerfile"],
    command: ["docker-langserver", "--stdio"]
  },
  {
    extensions: ["yaml", "yml"],
    command: ["npx", "--", "yaml-language-server", "--stdio"]
  },
  {
    extensions: ["md", "markdown"],
    command: ["npx", "--", "markdownlint-language-server", "--stdio"]
  }
];
function createDefaultConfig() {
  return {
    servers: DEFAULT_SERVERS
  };
}
function mergeWithDefaults(userConfig) {
  if (!userConfig?.servers) {
    return createDefaultConfig();
  }
  const userExtensions = new Set;
  for (const server of userConfig.servers) {
    for (const ext of server.extensions) {
      userExtensions.add(ext);
    }
  }
  const mergedServers = [...userConfig.servers];
  for (const defaultServer of DEFAULT_SERVERS) {
    const hasUnconfiguredExtension = defaultServer.extensions.some((ext) => !userExtensions.has(ext));
    if (hasUnconfiguredExtension) {
      const unconfiguredExtensions = defaultServer.extensions.filter((ext) => !userExtensions.has(ext));
      if (unconfiguredExtensions.length > 0) {
        mergedServers.push({
          ...defaultServer,
          extensions: unconfiguredExtensions
        });
      }
    }
  }
  return {
    servers: mergedServers
  };
}

// src/lsp/client.ts
init_file_scanner();

// src/lsp/protocol.ts
class LSPProtocol {
  nextId = 1;
  pendingRequests = new Map;
  async sendRequest(process2, method, params, timeout = 30000) {
    return new Promise((resolve, reject) => {
      const id = this.nextId++;
      const message = {
        jsonrpc: "2.0",
        id,
        method,
        params
      };
      this.pendingRequests.set(id, { resolve, reject });
      const timeoutId = setTimeout(() => {
        this.pendingRequests.delete(id);
        reject(new Error(`Request timed out after ${timeout}ms: ${method}`));
      }, timeout);
      const originalResolve = resolve;
      const originalReject = reject;
      this.pendingRequests.set(id, {
        resolve: (value) => {
          clearTimeout(timeoutId);
          originalResolve(value);
        },
        reject: (reason) => {
          clearTimeout(timeoutId);
          originalReject(reason);
        }
      });
      this.sendMessage(process2, message);
    });
  }
  sendNotification(process2, method, params) {
    const message = {
      jsonrpc: "2.0",
      method,
      params
    };
    this.sendMessage(process2, message);
  }
  handleMessage(message, serverState) {
    if (message.id && this.pendingRequests.has(message.id)) {
      const request = this.pendingRequests.get(message.id);
      if (!request)
        return;
      const { resolve, reject } = request;
      this.pendingRequests.delete(message.id);
      if (message.error) {
        if (message.error.code === -32601 || message.error.message?.toLowerCase().includes("unhandled method") || message.error.message?.toLowerCase().includes("method not found")) {
          resolve(null);
        } else {
          reject(new Error(message.error.message || "LSP Error"));
        }
      } else {
        resolve(message.result);
      }
    }
    if (message.method && serverState) {
      this.handleServerNotification(message, serverState);
    }
  }
  parseMessages(buffer) {
    const messages = [];
    let remaining = buffer;
    while (true) {
      const headerEndIndex = remaining.indexOf(`\r
\r
`);
      if (headerEndIndex === -1)
        break;
      const headers = remaining.substring(0, headerEndIndex);
      const contentLengthMatch = headers.match(/Content-Length: (\d+)/);
      if (!contentLengthMatch || !contentLengthMatch[1]) {
        remaining = remaining.substring(headerEndIndex + 4);
        continue;
      }
      const contentLength = Number.parseInt(contentLengthMatch[1], 10);
      const messageStart = headerEndIndex + 4;
      if (remaining.length < messageStart + contentLength)
        break;
      const messageContent = remaining.substring(messageStart, messageStart + contentLength);
      try {
        const message = JSON.parse(messageContent);
        messages.push(message);
      } catch (error) {
        process.stderr.write(`[ERROR] Failed to parse LSP message: ${error}
`);
      }
      remaining = remaining.substring(messageStart + contentLength);
    }
    return { messages, remainingBuffer: remaining };
  }
  sendMessage(process2, message) {
    const content = JSON.stringify(message);
    const header = `Content-Length: ${Buffer.byteLength(content)}\r
\r
`;
    process2.stdin?.write(header + content);
  }
  handleServerNotification(message, serverState) {
    if (message.method === "initialized") {
      process.stderr.write(`[DEBUG] Received initialized notification from server
`);
      serverState.initialized = true;
      if (serverState.initializationResolve) {
        serverState.initializationResolve();
        serverState.initializationResolve = undefined;
      }
    } else if (message.method === "textDocument/publishDiagnostics") {
      const params = message.params;
      if (params?.uri) {
        process.stderr.write(`[DEBUG] Received publishDiagnostics for ${params.uri} with ${params.diagnostics?.length || 0} diagnostics${params.version !== undefined ? ` (version: ${params.version})` : ""}
`);
        serverState.diagnostics.set(params.uri, params.diagnostics || []);
        serverState.lastDiagnosticUpdate.set(params.uri, Date.now());
        if (params.version !== undefined) {
          serverState.diagnosticVersions.set(params.uri, params.version);
        }
      }
    }
  }
  dispose() {
    for (const [id, request] of this.pendingRequests) {
      request.reject(new Error("LSP client disposed"));
    }
    this.pendingRequests.clear();
  }
}

// src/lsp/server-manager.ts
import { spawn } from "node:child_process";

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}

// src/lsp/server-manager.ts
class ServerManager {
  servers = new Map;
  serversStarting = new Map;
  failedServers = new Set;
  protocol;
  constructor(protocol) {
    this.protocol = protocol;
  }
  get activeServers() {
    return this.servers;
  }
  async getServer(filePath, config) {
    const serverConfig = this.getServerForFile(filePath, config);
    if (!serverConfig) {
      throw new Error(`No language server configured for file: ${filePath}`);
    }
    const serverKey = JSON.stringify(serverConfig.command);
    if (this.failedServers.has(serverKey)) {
      throw new Error(`Language server for ${serverConfig.extensions.join(", ")} files is not available. ` + `Install it with: ${this.getInstallInstructions(serverConfig.command[0])}`);
    }
    const existingServer = this.servers.get(serverKey);
    if (existingServer) {
      if (!existingServer.process.killed) {
        await existingServer.initializationPromise;
        return existingServer;
      }
      this.servers.delete(serverKey);
    }
    const startingPromise = this.serversStarting.get(serverKey);
    if (startingPromise) {
      return await startingPromise;
    }
    const startupPromise = this.startServer(serverConfig);
    this.serversStarting.set(serverKey, startupPromise);
    try {
      const serverState = await startupPromise;
      this.servers.set(serverKey, serverState);
      return serverState;
    } finally {
      this.serversStarting.delete(serverKey);
    }
  }
  clearFailedServers() {
    const count = this.failedServers.size;
    this.failedServers.clear();
    if (count > 0) {
      process.stderr.write(`Cleared ${count} failed server(s). They will be retried on next access.
`);
    }
  }
  async restartServer(extensions, config) {
    const restartedServers = [];
    if (!extensions || extensions.length === 0) {
      const serversToRestart = Array.from(this.servers.entries());
      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command?.join(" ") || "unknown");
      }
    } else {
      const serversToRestart = Array.from(this.servers.entries()).filter(([, serverState]) => {
        const serverConfig = serverState.config;
        return serverConfig && extensions.some((ext) => serverConfig.extensions.includes(ext));
      });
      for (const [serverKey, serverState] of serversToRestart) {
        this.killServer(serverState);
        this.servers.delete(serverKey);
        restartedServers.push(serverState.config?.command.join(" "));
      }
    }
    return restartedServers;
  }
  async preloadServers(config, extensions) {
    const serverConfigs = new Map;
    for (const extension of extensions) {
      const serverConfig = this.getServerForFile(`dummy.${extension}`, config);
      if (serverConfig) {
        const key = JSON.stringify(serverConfig.command);
        serverConfigs.set(key, serverConfig);
      }
    }
    const startPromises = Array.from(serverConfigs.values()).map(async (serverConfig) => {
      try {
        await this.startServer(serverConfig);
        process.stderr.write(`Preloaded server: ${serverConfig.command.join(" ")}
`);
      } catch (error) {
        process.stderr.write(`Failed to preload server ${serverConfig.command.join(" ")}: ${error}
`);
      }
    });
    await Promise.allSettled(startPromises);
  }
  getServerForFile(filePath, config) {
    const extension = filePath.split(".").pop();
    if (!extension)
      return null;
    process.stderr.write(`Looking for server for extension: ${extension}
`);
    const server = config.servers.find((server2) => server2.extensions.includes(extension));
    if (server) {
      process.stderr.write(`Found server for ${extension}: ${server.command.join(" ")}
`);
    } else {
      process.stderr.write(`No server found for extension: ${extension}
`);
    }
    return server || null;
  }
  async startServer(serverConfig) {
    const [command, ...args] = serverConfig.command;
    if (!command) {
      throw new Error("No command specified in server config");
    }
    if (command === "npx") {
      try {
        const { execSync } = await import("node:child_process");
        execSync("npm --version", { stdio: "ignore" });
      } catch {
        throw new Error("npm is required for TypeScript/JavaScript support. Please install Node.js from https://nodejs.org");
      }
    }
    const childProcess = spawn(command, args, {
      stdio: ["pipe", "pipe", "pipe"],
      cwd: serverConfig.rootDir || process.cwd()
    });
    let startupFailed = false;
    const startupErrorHandler = (error) => {
      startupFailed = true;
      const extensions = serverConfig.extensions.join(", ");
      if (error.message.includes("ENOENT")) {
        process.stderr.write(`⚠️  Language server not found for ${extensions} files
   Command: ${serverConfig.command.join(" ")}
   To enable: ${this.getInstallInstructions(command)}
`);
      } else {
        process.stderr.write(`⚠️  Failed to start language server for ${extensions} files
   Error: ${error.message}
`);
      }
      const serverKey2 = JSON.stringify(serverConfig.command);
      this.failedServers.add(serverKey2);
    };
    childProcess.once("error", startupErrorHandler);
    await new Promise((resolve) => setTimeout(resolve, 100));
    if (startupFailed) {
      throw new Error(`Language server for ${serverConfig.extensions.join(", ")} is not available`);
    }
    childProcess.removeListener("error", startupErrorHandler);
    let initializationResolve;
    const initializationPromise = new Promise((resolve) => {
      initializationResolve = resolve;
    });
    const serverState = {
      process: childProcess,
      initialized: false,
      initializationPromise,
      initializationResolve,
      capabilities: undefined,
      buffer: "",
      openFiles: new Set,
      diagnostics: new Map,
      lastDiagnosticUpdate: new Map,
      diagnosticVersions: new Map,
      restartTimer: undefined,
      config: serverConfig,
      fileVersions: new Map,
      startTime: Date.now()
    };
    this.setupProtocolHandlers(serverState);
    const initResult = await this.initializeServer(serverState, serverConfig);
    const serverKey = JSON.stringify(serverConfig.command);
    capabilityManager.cacheCapabilities(serverKey, initResult);
    if (initResult && typeof initResult === "object" && "capabilities" in initResult) {
      serverState.capabilities = initResult.capabilities;
    }
    this.protocol.sendNotification(childProcess, "initialized", {});
    await new Promise((resolve) => setTimeout(resolve, 500));
    serverState.initialized = true;
    if (serverState.initializationResolve) {
      serverState.initializationResolve();
      serverState.initializationResolve = undefined;
    }
    process.stderr.write(`Server initialized successfully: ${serverConfig.command.join(" ")}
`);
    this.setupRestartTimer(serverState, serverConfig);
    return serverState;
  }
  setupProtocolHandlers(serverState) {
    const serverKey = JSON.stringify(serverState.config?.command);
    serverState.process.stdout?.on("data", (data) => {
      serverState.buffer += data.toString();
      const { messages, remainingBuffer } = this.protocol.parseMessages(serverState.buffer);
      serverState.buffer = remainingBuffer;
      for (const message of messages) {
        this.protocol.handleMessage(message, serverState);
      }
    });
    serverState.process.stderr?.on("data", (data) => {
      process.stderr.write(data);
    });
    serverState.process.on("error", (error) => {
      process.stderr.write(`LSP server process error (${serverState.config?.command.join(" ")}): ${error.message}
`);
      this.servers.delete(serverKey);
    });
    serverState.process.on("exit", (code, signal) => {
      process.stderr.write(`LSP server exited (${serverState.config?.command.join(" ")}): code=${code}, signal=${signal}
`);
      if (serverState.restartTimer) {
        clearTimeout(serverState.restartTimer);
        serverState.restartTimer = undefined;
      }
      this.servers.delete(serverKey);
    });
  }
  async initializeServer(serverState, serverConfig) {
    const initializeParams = {
      processId: serverState.process.pid || null,
      clientInfo: { name: "cclsp", version: "0.5.12" },
      capabilities: {
        textDocument: {
          synchronization: {
            didOpen: true,
            didChange: true,
            didClose: true
          },
          definition: { linkSupport: false },
          references: {
            includeDeclaration: true,
            dynamicRegistration: false
          },
          rename: { prepareSupport: false },
          documentSymbol: {
            symbolKind: {
              valueSet: [
                1,
                2,
                3,
                4,
                5,
                6,
                7,
                8,
                9,
                10,
                11,
                12,
                13,
                14,
                15,
                16,
                17,
                18,
                19,
                20,
                21,
                22,
                23,
                24,
                25,
                26
              ]
            },
            hierarchicalDocumentSymbolSupport: true
          },
          completion: {
            completionItem: {
              snippetSupport: true
            }
          },
          hover: {},
          signatureHelp: {},
          diagnostic: {
            dynamicRegistration: false,
            relatedDocumentSupport: false
          }
        },
        workspace: {
          workspaceEdit: {
            documentChanges: true
          },
          workspaceFolders: true
        }
      },
      rootUri: pathToUri(serverConfig.rootDir || process.cwd()),
      workspaceFolders: [
        {
          uri: pathToUri(serverConfig.rootDir || process.cwd()),
          name: "workspace"
        }
      ],
      initializationOptions: this.getInitializationOptions(serverConfig)
    };
    return await this.protocol.sendRequest(serverState.process, "initialize", initializeParams, 1e4);
  }
  getInitializationOptions(serverConfig) {
    if (serverConfig.initializationOptions !== undefined) {
      return serverConfig.initializationOptions;
    }
    if (this.isPylspServer(serverConfig)) {
      return {
        settings: {
          pylsp: {
            plugins: {
              jedi_completion: { enabled: true },
              jedi_definition: { enabled: true },
              jedi_hover: { enabled: true },
              jedi_references: { enabled: true },
              jedi_signature_help: { enabled: true },
              jedi_symbols: { enabled: true },
              pylint: { enabled: false },
              pycodestyle: { enabled: false },
              pyflakes: { enabled: false },
              yapf: { enabled: false },
              rope_completion: { enabled: false }
            }
          }
        }
      };
    }
    if (this.isTypeScriptServer(serverConfig)) {
      return {
        hostInfo: "cclsp",
        preferences: {
          includeCompletionsForModuleExports: true,
          includeCompletionsWithInsertText: true
        }
      };
    }
    return;
  }
  setupRestartTimer(serverState, serverConfig) {
    if (serverConfig.restartInterval && serverConfig.restartInterval > 0) {
      const intervalMs = serverConfig.restartInterval * 60 * 1000;
      serverState.restartTimer = setTimeout(() => {
        process.stderr.write(`Auto-restarting server ${serverConfig.command.join(" ")} after ${serverConfig.restartInterval} minutes
`);
        this.killServer(serverState);
        const serverKey = JSON.stringify(serverConfig.command);
        this.servers.delete(serverKey);
      }, intervalMs);
    }
  }
  killServer(serverState) {
    if (serverState.restartTimer) {
      clearTimeout(serverState.restartTimer);
    }
    try {
      if (!serverState.process.killed) {
        serverState.process.kill("SIGTERM");
      }
    } catch (error) {
      process.stderr.write(`Warning: Failed to kill server process (PID: ${serverState.process.pid}): ${error instanceof Error ? error.message : String(error)}
`);
    }
  }
  isPylspServer(serverConfig) {
    return serverConfig.command.some((cmd) => cmd.includes("pylsp"));
  }
  getInstallInstructions(command) {
    const instructions = {
      "typescript-language-server": "npm install -g typescript-language-server typescript",
      pylsp: "pip install python-lsp-server",
      gopls: "go install golang.org/x/tools/gopls@latest",
      "rust-analyzer": "rustup component add rust-analyzer",
      clangd: "apt install clangd OR brew install llvm",
      jdtls: "Download from Eclipse JDT releases",
      solargraph: "gem install solargraph",
      intelephense: "npm install -g intelephense"
    };
    return instructions[command] || `Install ${command} for your system`;
  }
  isTypeScriptServer(serverConfig) {
    return serverConfig.command.some((cmd) => cmd.includes("typescript-language-server") || cmd.includes("tsserver"));
  }
  dispose() {
    for (const serverState of this.servers.values()) {
      this.killServer(serverState);
    }
    this.servers.clear();
    this.serversStarting.clear();
    this.protocol.dispose();
  }
}

// src/lsp/client.ts
class LSPClient {
  config;
  _protocol;
  _serverManager;
  get protocol() {
    return this._protocol;
  }
  get serverManager() {
    return this._serverManager;
  }
  constructor(configPath) {
    this._protocol = new LSPProtocol;
    this._serverManager = new ServerManager(this._protocol);
    this.config = this.loadConfig(configPath);
  }
  loadConfig(configPath) {
    if (process.env.CCLSP_CONFIG_PATH) {
      process.stderr.write(`Loading config from CCLSP_CONFIG_PATH: ${process.env.CCLSP_CONFIG_PATH}
`);
      if (!existsSync(process.env.CCLSP_CONFIG_PATH)) {
        process.stderr.write(`Warning: Config file specified in CCLSP_CONFIG_PATH does not exist: ${process.env.CCLSP_CONFIG_PATH}
`);
        process.stderr.write(`Falling back to default configuration...
`);
        return this.loadDefaultConfig();
      }
      try {
        const configData = readFileSync(process.env.CCLSP_CONFIG_PATH, "utf-8");
        const config = JSON.parse(configData);
        process.stderr.write(`Loaded ${config.servers.length} server configurations from env
`);
        return mergeWithDefaults(config);
      } catch (error) {
        process.stderr.write(`Warning: Failed to load config from CCLSP_CONFIG_PATH: ${error instanceof Error ? error.message : String(error)}
`);
        process.stderr.write(`Falling back to default configuration...
`);
        return this.loadDefaultConfig();
      }
    }
    if (configPath) {
      try {
        process.stderr.write(`Loading config from file: ${configPath}
`);
        const configData = readFileSync(configPath, "utf-8");
        const config = JSON.parse(configData);
        process.stderr.write(`Loaded ${config.servers.length} server configurations
`);
        return mergeWithDefaults(config);
      } catch (error) {
        process.stderr.write(`Warning: Failed to load config from ${configPath}: ${error instanceof Error ? error.message : String(error)}
`);
        process.stderr.write(`Falling back to default configuration...
`);
        return this.loadDefaultConfig();
      }
    }
    const defaultConfigPath = "cclsp.json";
    if (existsSync(defaultConfigPath)) {
      try {
        process.stderr.write(`Found cclsp.json in current directory, loading...
`);
        const configData = readFileSync(defaultConfigPath, "utf-8");
        const config = JSON.parse(configData);
        process.stderr.write(`Loaded ${config.servers.length} server configurations
`);
        return mergeWithDefaults(config);
      } catch (error) {
        process.stderr.write(`Warning: Failed to load cclsp.json: ${error instanceof Error ? error.message : String(error)}
`);
      }
    }
    process.stderr.write(`No configuration found, using smart defaults...
`);
    return this.loadDefaultConfig();
  }
  loadDefaultConfig() {
    const defaultConfig = createDefaultConfig();
    process.stderr.write(`Using default configuration with support for ${defaultConfig.servers.length} languages
`);
    process.stderr.write(`TypeScript/JavaScript works out of the box (bundled dependency)
`);
    process.stderr.write(`Other languages work if their servers are installed
`);
    process.stderr.write(`To customize, create a cclsp.json file or run: cclsp setup
`);
    return defaultConfig;
  }
  getLanguageName(extension) {
    const languageMap = {
      ts: "TypeScript",
      tsx: "TypeScript",
      js: "JavaScript",
      jsx: "JavaScript",
      py: "Python",
      go: "Go",
      rs: "Rust",
      java: "Java",
      rb: "Ruby",
      php: "PHP",
      c: "C",
      cpp: "C++",
      css: "CSS",
      html: "HTML",
      json: "JSON",
      yaml: "YAML",
      vue: "Vue",
      svelte: "Svelte"
    };
    return languageMap[extension] || null;
  }
  async getServer(filePath) {
    return await this._serverManager.getServer(filePath, this.config);
  }
  async sendRequest(serverState, method, params, timeout) {
    return await this._protocol.sendRequest(serverState.process, method, params, timeout);
  }
  sendNotification(serverState, method, params) {
    this._protocol.sendNotification(serverState.process, method, params);
  }
  async restartServer(extensions) {
    return await this._serverManager.restartServer(extensions, this.config);
  }
  async preloadServers() {
    try {
      const extensions = await scanDirectoryForExtensions(process.cwd());
      await this._serverManager.preloadServers(this.config, Array.from(extensions));
    } catch (error) {
      process.stderr.write(`Failed to scan directory for extensions: ${error}
`);
    }
  }
  dispose() {
    this._serverManager.dispose();
  }
}

// src/services/diagnostic-service.ts
import { readFileSync as readFileSync2 } from "node:fs";
class DiagnosticService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async getDiagnostics(filePath) {
    process.stderr.write(`[DEBUG getDiagnostics] Requesting diagnostics for ${filePath}
`);
    const serverState = await this.getServer(filePath);
    await serverState.initializationPromise;
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const cachedDiagnostics = serverState.diagnostics.get(fileUri);
    if (cachedDiagnostics !== undefined) {
      process.stderr.write(`[DEBUG getDiagnostics] Returning ${cachedDiagnostics.length} cached diagnostics from publishDiagnostics
`);
      return cachedDiagnostics;
    }
    process.stderr.write(`[DEBUG getDiagnostics] No cached diagnostics, trying textDocument/diagnostic request
`);
    try {
      const result = await this.protocol.sendRequest(serverState.process, "textDocument/diagnostic", {
        textDocument: { uri: fileUri }
      });
      process.stderr.write(`[DEBUG getDiagnostics] Result type: ${typeof result}, has kind: ${result && typeof result === "object" && "kind" in result}
`);
      process.stderr.write(`[DEBUG getDiagnostics] Full result: ${JSON.stringify(result)}
`);
      if (result && typeof result === "object" && "kind" in result) {
        const report = result;
        if (report.kind === "full" && report.items) {
          process.stderr.write(`[DEBUG getDiagnostics] Full report with ${report.items.length} diagnostics
`);
          return report.items;
        }
        if (report.kind === "unchanged") {
          process.stderr.write(`[DEBUG getDiagnostics] Unchanged report (no new diagnostics)
`);
          return [];
        }
      }
      if (Array.isArray(result)) {
        process.stderr.write(`[DEBUG getDiagnostics] Direct diagnostic array with ${result.length} diagnostics
`);
        return result;
      }
      if (result === null || result === undefined) {
        process.stderr.write(`[DEBUG getDiagnostics] Null/undefined result, falling back to other methods
`);
      } else {
        process.stderr.write(`[DEBUG getDiagnostics] Unexpected response format, falling back to other methods
`);
      }
    } catch (error) {
      process.stderr.write(`[DEBUG getDiagnostics] textDocument/diagnostic not supported or failed: ${error}. Waiting for publishDiagnostics...
`);
      await this.waitForDiagnosticsIdle(serverState, fileUri, {
        maxWaitTime: 5000,
        idleTime: 300
      });
      const diagnosticsAfterWait = serverState.diagnostics.get(fileUri);
      if (diagnosticsAfterWait !== undefined) {
        process.stderr.write(`[DEBUG getDiagnostics] Returning ${diagnosticsAfterWait.length} diagnostics after waiting for idle state
`);
        return diagnosticsAfterWait;
      }
      process.stderr.write(`[DEBUG getDiagnostics] No diagnostics yet, triggering publishDiagnostics with no-op change
`);
      try {
        const fileContent = readFileSync2(filePath, "utf-8");
        const version1 = (serverState.fileVersions.get(filePath) || 1) + 1;
        serverState.fileVersions.set(filePath, version1);
        await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
          textDocument: {
            uri: fileUri,
            version: version1
          },
          contentChanges: [
            {
              text: `${fileContent} `
            }
          ]
        });
        const version2 = version1 + 1;
        serverState.fileVersions.set(filePath, version2);
        await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
          textDocument: {
            uri: fileUri,
            version: version2
          },
          contentChanges: [
            {
              text: fileContent
            }
          ]
        });
        await this.waitForDiagnosticsIdle(serverState, fileUri, {
          maxWaitTime: 3000,
          idleTime: 300
        });
        const diagnosticsAfterTrigger = serverState.diagnostics.get(fileUri);
        if (diagnosticsAfterTrigger !== undefined) {
          process.stderr.write(`[DEBUG getDiagnostics] Returning ${diagnosticsAfterTrigger.length} diagnostics after triggering publishDiagnostics
`);
          return diagnosticsAfterTrigger;
        }
      } catch (triggerError) {
        process.stderr.write(`[DEBUG getDiagnostics] Failed to trigger publishDiagnostics: ${triggerError}
`);
      }
      return [];
    }
  }
  filterDiagnosticsByLevel(diagnostics, minSeverity) {
    return diagnostics.filter((diagnostic) => diagnostic.severity === undefined || diagnostic.severity <= minSeverity);
  }
  getRelatedDiagnostics(diagnostics, position) {
    return diagnostics.filter((diagnostic) => {
      const range = diagnostic.range;
      return position.line >= range.start.line && position.line <= range.end.line && (position.line !== range.start.line || position.character >= range.start.character) && (position.line !== range.end.line || position.character <= range.end.character);
    });
  }
  categorizeDiagnostics(diagnostics) {
    const errors = [];
    const warnings = [];
    const infos = [];
    const hints = [];
    for (const diagnostic of diagnostics) {
      switch (diagnostic.severity) {
        case 1:
          errors.push(diagnostic);
          break;
        case 2:
          warnings.push(diagnostic);
          break;
        case 3:
          infos.push(diagnostic);
          break;
        case 4:
          hints.push(diagnostic);
          break;
        default:
          errors.push(diagnostic);
      }
    }
    return { errors, warnings, infos, hints };
  }
  async waitForDiagnosticsIdle(serverState, fileUri, options = {}) {
    const {
      maxWaitTime = 1e4,
      idleTime = 1000,
      checkInterval = 100
    } = options;
    const startTime = Date.now();
    let lastUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;
    return new Promise((resolve) => {
      const checkIdle = () => {
        const now = Date.now();
        const currentUpdateTime = serverState.lastDiagnosticUpdate.get(fileUri) || 0;
        if (now - startTime >= maxWaitTime) {
          process.stderr.write(`[DEBUG waitForDiagnosticsIdle] Max wait time reached for ${fileUri}
`);
          resolve();
          return;
        }
        if (currentUpdateTime > lastUpdateTime) {
          lastUpdateTime = currentUpdateTime;
          setTimeout(checkIdle, checkInterval);
          return;
        }
        if (now - lastUpdateTime >= idleTime) {
          process.stderr.write(`[DEBUG waitForDiagnosticsIdle] Diagnostics idle for ${fileUri}
`);
          resolve();
          return;
        }
        setTimeout(checkIdle, checkInterval);
      };
      setTimeout(checkIdle, checkInterval);
    });
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync2(filePath, "utf-8");
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

// src/services/file-service.ts
import { readFileSync as readFileSync3 } from "node:fs";
class FileService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async formatDocument(filePath, options) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const formattingOptions = {
      tabSize: options?.tabSize || 2,
      insertSpaces: options?.insertSpaces !== false,
      ...options?.trimTrailingWhitespace !== undefined && {
        trimTrailingWhitespace: options.trimTrailingWhitespace
      },
      ...options?.insertFinalNewline !== undefined && {
        insertFinalNewline: options.insertFinalNewline
      },
      ...options?.trimFinalNewlines !== undefined && {
        trimFinalNewlines: options.trimFinalNewlines
      }
    };
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/formatting", {
      textDocument: { uri: fileUri },
      options: formattingOptions
    });
    return Array.isArray(result) ? result : [];
  }
  async getCodeActions(filePath, range, context) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    const diagnostics = serverState.diagnostics.get(fileUri) || [];
    const requestRange = range || {
      start: { line: 0, character: 0 },
      end: { line: Math.min(100, 999999), character: 0 }
    };
    const codeActionContext = {
      diagnostics: context?.diagnostics || diagnostics,
      only: undefined
    };
    process.stderr.write(`[DEBUG getCodeActions] Request params: ${JSON.stringify({
      textDocument: { uri: fileUri },
      range: requestRange,
      context: codeActionContext
    }, null, 2)}
`);
    try {
      const result = await this.protocol.sendRequest(serverState.process, "textDocument/codeAction", {
        textDocument: { uri: fileUri },
        range: requestRange,
        context: codeActionContext
      });
      process.stderr.write(`[DEBUG getCodeActions] Raw result: ${JSON.stringify(result)}
`);
      if (!result)
        return [];
      if (Array.isArray(result))
        return result.filter((action) => action != null);
      return [];
    } catch (error) {
      process.stderr.write(`[DEBUG getCodeActions] Error: ${error}
`);
      return [];
    }
  }
  async getFoldingRanges(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    process.stderr.write(`[DEBUG getFoldingRanges] Requesting folding ranges for: ${filePath}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/foldingRange", {
      textDocument: { uri: fileUri }
    });
    process.stderr.write(`[DEBUG getFoldingRanges] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (Array.isArray(result)) {
      return result;
    }
    return [];
  }
  async getDocumentLinks(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState.initialized) {
      throw new Error("Server not initialized");
    }
    await this.ensureFileOpen(serverState, filePath);
    const fileUri = pathToUri(filePath);
    process.stderr.write(`[DEBUG getDocumentLinks] Requesting document links for: ${filePath}
`);
    const result = await this.protocol.sendRequest(serverState.process, "textDocument/documentLink", {
      textDocument: { uri: fileUri }
    });
    process.stderr.write(`[DEBUG getDocumentLinks] Result type: ${typeof result}, isArray: ${Array.isArray(result)}, length: ${Array.isArray(result) ? result.length : "N/A"}
`);
    if (Array.isArray(result)) {
      return result;
    }
    return [];
  }
  async applyWorkspaceEdit(edit) {
    try {
      if (edit.changes) {
        for (const [uri, edits] of Object.entries(edit.changes)) {
          const filePath = uri.replace("file://", "");
          await this.applyTextEdits(filePath, edits);
        }
      }
      if (edit.documentChanges) {
        for (const change of edit.documentChanges) {
          const filePath = change.textDocument.uri.replace("file://", "");
          await this.applyTextEdits(filePath, change.edits);
        }
      }
      return { applied: true };
    } catch (error) {
      return {
        applied: false,
        failureReason: error instanceof Error ? error.message : String(error)
      };
    }
  }
  async renameFile(oldPath, newPath) {
    try {
      const serverConfigs = new Map;
      for (const serverState of serverConfigs.values()) {
        this.protocol.sendNotification(serverState.process, "workspace/willRenameFiles", {
          files: [
            {
              oldUri: `file://${oldPath}`,
              newUri: `file://${newPath}`
            }
          ]
        });
      }
    } catch (error) {
      process.stderr.write(`[ERROR renameFile] ${error}
`);
    }
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync3(filePath, "utf-8");
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
  async applyTextEdits(filePath, edits) {
    if (edits.length === 0)
      return;
    try {
      const fileContent = readFileSync3(filePath, "utf-8");
      const lines = fileContent.split(`
`);
      const sortedEdits = [...edits].sort((a, b) => {
        if (a.range.start.line !== b.range.start.line) {
          return b.range.start.line - a.range.start.line;
        }
        return b.range.start.character - a.range.start.character;
      });
      for (const edit of sortedEdits) {
        const startLine = edit.range.start.line;
        const startChar = edit.range.start.character;
        const endLine = edit.range.end.line;
        const endChar = edit.range.end.character;
        if (startLine === endLine) {
          const line = lines[startLine];
          if (line !== undefined) {
            lines[startLine] = line.substring(0, startChar) + edit.newText + line.substring(endChar);
          }
        } else {
          const newLines = edit.newText.split(`
`);
          const startLineText = lines[startLine];
          const endLineText = lines[endLine];
          if (startLineText !== undefined && endLineText !== undefined) {
            const firstLine = startLineText.substring(0, startChar) + newLines[0];
            const lastLine = newLines[newLines.length - 1] + endLineText.substring(endChar);
            const replacementLines = [firstLine, ...newLines.slice(1, -1), lastLine];
            lines.splice(startLine, endLine - startLine + 1, ...replacementLines);
          }
        }
      }
      process.stderr.write(`[DEBUG applyTextEdits] Would apply ${edits.length} edits to ${filePath}
`);
    } catch (error) {
      throw new Error(`Failed to apply text edits to ${filePath}: ${error}`);
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
  async syncFileContent(filePath) {
    try {
      const serverState = await this.getServer(filePath);
      if (!serverState.openFiles.has(filePath)) {
        process.stderr.write(`[DEBUG syncFileContent] File not open, opening it first: ${filePath}
`);
        await this.ensureFileOpen(serverState, filePath);
      }
      process.stderr.write(`[DEBUG syncFileContent] Syncing file: ${filePath}
`);
      const fileContent = readFileSync3(filePath, "utf-8");
      const uri = pathToUri(filePath);
      const version = (serverState.fileVersions.get(filePath) || 1) + 1;
      serverState.fileVersions.set(filePath, version);
      await this.protocol.sendNotification(serverState.process, "textDocument/didChange", {
        textDocument: {
          uri,
          version
        },
        contentChanges: [
          {
            text: fileContent
          }
        ]
      });
      process.stderr.write(`[DEBUG syncFileContent] File synced with version ${version}: ${filePath}
`);
    } catch (error) {
      process.stderr.write(`[DEBUG syncFileContent] Failed to sync file ${filePath}: ${error}
`);
    }
  }
}

// src/services/hierarchy-service.ts
import { readFileSync as readFileSync4 } from "node:fs";

class HierarchyService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async prepareCallHierarchy(filePath, position) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/prepareCallHierarchy", {
      textDocument: { uri: `file://${filePath}` },
      position
    });
    return Array.isArray(response) ? response : [];
  }
  async getCallHierarchyIncomingCalls(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "callHierarchy/incomingCalls", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getCallHierarchyOutgoingCalls(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "callHierarchy/outgoingCalls", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async prepareTypeHierarchy(filePath, position) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/prepareTypeHierarchy", {
      textDocument: { uri: `file://${filePath}` },
      position
    });
    return Array.isArray(response) ? response : [];
  }
  async getTypeHierarchySupertypes(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "typeHierarchy/supertypes", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getTypeHierarchySubtypes(item) {
    const filePath = item.uri.replace("file://", "");
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    const response = await this.protocol.sendRequest(serverState.process, "typeHierarchy/subtypes", {
      item
    });
    return Array.isArray(response) ? response : [];
  }
  async getSelectionRange(filePath, positions) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/selectionRange", {
        textDocument: { uri: `file://${filePath}` },
        positions
      }, 5000);
      return Array.isArray(response) ? response : [];
    } catch (error) {
      if (error instanceof Error && error.message?.includes("timeout")) {
        throw new Error("Selection range request timed out - TypeScript server may be overloaded");
      }
      throw error;
    }
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync4(filePath, "utf-8");
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

// src/services/intelligence-service.ts
import { readFileSync as readFileSync5 } from "node:fs";

class IntelligenceService {
  getServer;
  protocol;
  constructor(getServer, protocol) {
    this.getServer = getServer;
    this.protocol = protocol;
  }
  async getHover(filePath, position) {
    console.error("[DEBUG getHover] Starting hover request for", filePath);
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    console.error("[DEBUG getHover] Got server state");
    await this.ensureFileOpen(serverState, filePath);
    console.error("[DEBUG getHover] File opened");
    await new Promise((resolve) => setTimeout(resolve, 500));
    console.error("[DEBUG getHover] Waited for TS to process");
    console.error("[DEBUG getHover] Calling sendRequest with 30s timeout");
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/hover", {
        textDocument: { uri: `file://${filePath}` },
        position
      }, 30000);
      console.error("[DEBUG getHover] Got response:", response);
      return response && typeof response === "object" && "contents" in response ? response : null;
    } catch (error) {
      const errorMessage = error instanceof Error ? error.message : String(error);
      console.error("[DEBUG getHover] Error:", errorMessage);
      if (error instanceof Error && error.message?.includes("timeout")) {
        return {
          contents: {
            kind: "markdown",
            value: `**Hover information unavailable**

The TypeScript Language Server did not respond to the hover request at line ${position.line + 1}, character ${position.character + 1}. This feature may not be fully supported in the current server configuration.`
          }
        };
      }
      throw error;
    }
  }
  async getCompletions(filePath, position, triggerCharacter) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    await new Promise((resolve) => setTimeout(resolve, 500));
    const completionParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter ? {
        triggerKind: 2,
        triggerCharacter
      } : {
        triggerKind: 1
      }
    };
    try {
      const response = await this.protocol.sendRequest(serverState.process, "textDocument/completion", completionParams, 5000);
      if (!response || typeof response !== "object")
        return [];
      const result = response;
      return Array.isArray(result.items) ? result.items : result.items || [];
    } catch (error) {
      if (error instanceof Error && error.message?.includes("timeout")) {
        return [
          {
            label: "Completions unavailable",
            detail: "TypeScript Language Server timeout",
            documentation: "The TypeScript Language Server did not respond to the completion request. This feature may not be fully supported in the current server configuration.",
            insertText: "",
            kind: 1
          }
        ];
      }
      throw error;
    }
  }
  async getSignatureHelp(filePath, position, triggerCharacter) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const signatureHelpParams = {
      textDocument: { uri: `file://${filePath}` },
      position,
      context: triggerCharacter ? {
        triggerKind: 2,
        triggerCharacter,
        isRetrigger: false
      } : {
        triggerKind: 1,
        isRetrigger: false
      }
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/signatureHelp", signatureHelpParams);
    return response && typeof response === "object" && "signatures" in response ? response : null;
  }
  async getInlayHints(filePath, range) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const inlayHintParams = {
      textDocument: { uri: `file://${filePath}` },
      range
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/inlayHint", inlayHintParams);
    return Array.isArray(response) ? response : [];
  }
  async getSemanticTokens(filePath) {
    const serverState = await this.getServer(filePath);
    if (!serverState) {
      throw new Error("No LSP server available for this file type");
    }
    await this.ensureFileOpen(serverState, filePath);
    const semanticTokensParams = {
      textDocument: { uri: `file://${filePath}` }
    };
    const response = await this.protocol.sendRequest(serverState.process, "textDocument/semanticTokens/full", semanticTokensParams);
    return response && typeof response === "object" && "data" in response ? response : null;
  }
  async ensureFileOpen(serverState, filePath) {
    if (serverState.openFiles.has(filePath)) {
      return;
    }
    try {
      const fileContent = readFileSync5(filePath, "utf-8");
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

// src/services/symbol-service.ts
import { readFileSync as readFileSync6 } from "node:fs";
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
      const fileContent = readFileSync6(filePath, "utf-8");
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
      const fileContent = readFileSync6(filePath, "utf-8");
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

// src/lsp-client.ts
class LSPClient2 {
  newClient;
  protocol;
  serverManager;
  symbolService;
  fileService;
  diagnosticService;
  intelligenceService;
  hierarchyService;
  constructor(configPath) {
    this.newClient = new LSPClient(configPath);
    this.protocol = this.newClient.protocol;
    this.serverManager = this.newClient.serverManager;
    const getServerWrapper = (filePath) => this.newClient.getServer(filePath);
    this.symbolService = new SymbolService(getServerWrapper, this.protocol);
    this.fileService = new FileService(getServerWrapper, this.protocol);
    this.diagnosticService = new DiagnosticService(getServerWrapper, this.protocol);
    this.intelligenceService = new IntelligenceService(getServerWrapper, this.protocol);
    this.hierarchyService = new HierarchyService(getServerWrapper, this.protocol);
  }
  async findDefinition(filePath, position) {
    return this.symbolService.findDefinition(filePath, position);
  }
  async findReferences(filePath, position, includeDeclaration = false) {
    return this.symbolService.findReferences(filePath, position, includeDeclaration);
  }
  async renameSymbol(filePath, position, newName) {
    return this.symbolService.renameSymbol(filePath, position, newName);
  }
  async getDocumentSymbols(filePath) {
    return this.symbolService.getDocumentSymbols(filePath);
  }
  async searchWorkspaceSymbols(query) {
    return this.symbolService.searchWorkspaceSymbols(query, this.serverManager.activeServers, this.newClient.preloadServers.bind(this.newClient));
  }
  async findSymbolMatches(filePath, symbolName, symbolKind) {
    return this.symbolService.findSymbolMatches(filePath, symbolName, symbolKind);
  }
  async formatDocument(filePath, options) {
    return this.fileService.formatDocument(filePath, options);
  }
  async getCodeActions(filePath, range, context) {
    return this.fileService.getCodeActions(filePath, range || { start: { line: 0, character: 0 }, end: { line: 0, character: 0 } }, context || { diagnostics: [] });
  }
  async getFoldingRanges(filePath) {
    return this.fileService.getFoldingRanges(filePath);
  }
  async getDocumentLinks(filePath) {
    return this.fileService.getDocumentLinks(filePath);
  }
  async getDiagnostics(filePath) {
    return this.diagnosticService.getDiagnostics(filePath);
  }
  async syncFileContent(filePath) {
    return this.fileService.syncFileContent(filePath);
  }
  async getHover(filePath, position) {
    return this.intelligenceService.getHover(filePath, position);
  }
  async getCompletions(filePath, position, triggerCharacter) {
    return this.intelligenceService.getCompletions(filePath, position, triggerCharacter);
  }
  async getSignatureHelp(filePath, position, triggerCharacter) {
    return this.intelligenceService.getSignatureHelp(filePath, position, triggerCharacter);
  }
  async getInlayHints(filePath, range) {
    return this.intelligenceService.getInlayHints(filePath, range);
  }
  async getSemanticTokens(filePath) {
    return this.intelligenceService.getSemanticTokens(filePath);
  }
  async prepareCallHierarchy(filePath, position) {
    return this.hierarchyService.prepareCallHierarchy(filePath, position);
  }
  async getCallHierarchyIncomingCalls(item) {
    return this.hierarchyService.getCallHierarchyIncomingCalls(item);
  }
  async getCallHierarchyOutgoingCalls(item) {
    return this.hierarchyService.getCallHierarchyOutgoingCalls(item);
  }
  async prepareTypeHierarchy(filePath, position) {
    return this.hierarchyService.prepareTypeHierarchy(filePath, position);
  }
  async getTypeHierarchySupertypes(item) {
    return this.hierarchyService.getTypeHierarchySupertypes(item);
  }
  async getTypeHierarchySubtypes(item) {
    return this.hierarchyService.getTypeHierarchySubtypes(item);
  }
  async getSelectionRange(filePath, positions) {
    return this.hierarchyService.getSelectionRange(filePath, positions);
  }
  async getServer(filePath) {
    return this.newClient.getServer(filePath);
  }
  async getServerForService(filePath) {
    return this.newClient.getServer(filePath);
  }
  async sendRequest(process2, method, params, timeout) {
    return this.protocol.sendRequest(process2, method, params, timeout);
  }
  sendNotification(process2, method, params) {
    this.protocol.sendNotification(process2, method, params);
  }
  async restartServer(extensions) {
    return this.newClient.restartServer(extensions);
  }
  async findSymbolsByName(filePath, symbolName, symbolKind) {
    const matches = await this.findSymbolMatches(filePath, symbolName, symbolKind);
    return { matches };
  }
  async restartServers(extensions) {
    try {
      const restarted = await this.restartServer(extensions);
      const message = `Successfully restarted ${restarted.length} LSP server(s)`;
      return { success: true, restarted, failed: [], message };
    } catch (error) {
      const message = `Failed to restart servers: ${error instanceof Error ? error.message : String(error)}`;
      return { success: false, restarted: [], failed: [message], message };
    }
  }
  async preloadServers() {
    return this.newClient.preloadServers();
  }
  get flattenDocumentSymbols() {
    return this.symbolService.flattenDocumentSymbols;
  }
  get isDocumentSymbolArray() {
    return this.symbolService.isDocumentSymbolArray;
  }
  get symbolKindToString() {
    return this.symbolService.symbolKindToString;
  }
  get getValidSymbolKinds() {
    return this.symbolService.getValidSymbolKinds;
  }
  hasCapability(filePath, capabilityPath) {
    return this.getServer(filePath).then((serverState) => {
      return capabilityManager.hasCapability(serverState, capabilityPath);
    }).catch(() => false);
  }
  async getCapabilityInfo(filePath) {
    try {
      const serverState = await this.getServer(filePath);
      return capabilityManager.getCapabilityInfo(serverState);
    } catch (error) {
      return `Error getting server: ${error instanceof Error ? error.message : String(error)}`;
    }
  }
  async validateCapabilities(filePath, requiredCapabilities) {
    try {
      const serverState = await this.getServer(filePath);
      const validation = capabilityManager.validateRequiredCapabilities(serverState, requiredCapabilities);
      return {
        ...validation,
        serverDescription: capabilityManager.getServerDescription(serverState)
      };
    } catch (error) {
      return {
        supported: false,
        missing: requiredCapabilities,
        serverDescription: "Unknown Server"
      };
    }
  }
  async ensureFileOpen(serverState, filePath) {
    return this.fileService.ensureFileOpen(serverState, filePath);
  }
  dispose() {
    this.newClient.dispose();
  }
}
export {
  LSPClient2 as LSPClient
};
