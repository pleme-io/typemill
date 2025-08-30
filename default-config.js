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
var __require = /* @__PURE__ */ createRequire(import.meta.url);

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
async function isCommandAvailable(command) {
  try {
    const { spawn } = await import("node:child_process");
    const [cmd, ...args] = command;
    if (cmd === "npx") {
      return await isCommandAvailable(["npm", "--version"]);
    }
    return new Promise((resolve) => {
      const testArgs = cmd === "npm" ? ["--version"] : ["--version"];
      const proc = spawn(cmd, testArgs, {
        stdio: "ignore",
        shell: false
      });
      proc.on("error", () => resolve(false));
      proc.on("exit", (code) => resolve(code === 0));
      setTimeout(() => {
        proc.kill();
        resolve(false);
      }, 2000);
    });
  } catch {
    return false;
  }
}
async function getAvailableDefaultServers() {
  const available = [];
  for (const server of DEFAULT_SERVERS) {
    if (server.command[0] === "npx") {
      available.push(server);
      continue;
    }
    if (await isCommandAvailable(server.command)) {
      available.push(server);
    }
  }
  return available;
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
export {
  mergeWithDefaults,
  isCommandAvailable,
  getAvailableDefaultServers,
  createDefaultConfig,
  DEFAULT_SERVERS
};
