export interface ToolChainStep {
  tool: string;
  args: Record<string, any>;
  id?: string;
}

export interface ToolChain {
  id: string;
  description?: string;
  steps: ToolChainStep[];
}

export interface WorkflowContext {
  results: Map<string, any>; // Step ID -> result
  inputs: Record<string, any>;
}