/**
 * Type definitions for codeflow-buddy-client
 */

// Tool-related types
export interface ToolInfo {
  name: string;
  description?: string;
  inputSchema?: {
    properties?: Record<string, SchemaProperty>;
    required?: string[];
  };
}

export interface SchemaProperty {
  type: string;
  description?: string;
  enum?: string[];
  default?: unknown;
  items?: SchemaProperty;
  properties?: Record<string, SchemaProperty>;
}

export interface ToolListResponse {
  tools?: ToolInfo[];
}

// Event handler types
export type EventHandler = (...args: unknown[]) => void;
export type EventMap = Map<string, Set<EventHandler>>;

// CLI command options
export interface CallToolOptions {
  interactive?: boolean;
  format?: 'json' | 'pretty';
}

// Prompt types for inquirer
export interface InquirerPrompt {
  type: string;
  name: string;
  message: string;
  default?: unknown;
  choices?: Array<{ name: string; value: unknown }>;
  when?: (answers: Record<string, unknown>) => boolean | Promise<boolean>;
}

export interface PromptAnswer {
  include?: boolean;
  [key: string]: unknown;
}

// HTTP proxy types
export interface HttpProxyServer {
  listen(port: number): void;
  close(): void;
}

// WebSocket event data types
export interface WebSocketMessageData {
  toString(): string;
}

// Tool parameters
export type ToolParameters = Record<string, unknown>;

// Response types
export interface MCPResponse {
  id?: string | number;
  result?: unknown;
  error?: {
    code: number;
    message: string;
    data?: unknown;
  };
}

// Client configuration
export interface ClientConfig {
  url?: string;
  configDir?: string;
  wsUrl?: string;
  interactive?: boolean;
  timeout?: number;
}