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
var import_ignore = __toESM(require_ignore(), 1);
import { readFile, readdir, stat } from "node:fs/promises";
import { constants, access } from "node:fs/promises";
import { extname, join } from "node:path";
var DEFAULT_IGNORE_PATTERNS = [
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
export {
  scanProjectFiles,
  scanDirectoryForExtensions,
  loadGitignore
};
