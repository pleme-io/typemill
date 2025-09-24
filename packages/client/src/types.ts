// Type Definitions for Codeflow Buddy Client

export interface MCPRequest {
  method: string;
  params?: any;
  id?: string | number;
}

export interface MCPResponse {
  result?: any;
  error?: any;
  id?: string | number;
}

export interface ClientConfig {
  serverUrl: string;
  token?: string;
  projectPath?: string;
}
