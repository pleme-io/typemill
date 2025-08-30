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

// src/file-scanner.ts
import { readFile, readdir, stat } from "node:fs/promises";
var import_ignore = __toESM(require_ignore(), 1);
import { extname, join } from "node:path";
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
export {
  LSPClient
};
