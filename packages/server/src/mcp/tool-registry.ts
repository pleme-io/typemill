/**
 * Central Tool Registry
 * Single source of truth for all MCP tool mappings and workflow definitions
 * This decouples the batch executor from handler implementations
 */

import type { WorkflowToolDefinition } from './handler-types.js';

export type ServiceType =
  | 'symbol'
  | 'file'
  | 'diagnostic'
  | 'intelligence'
  | 'hierarchy'
  | 'lsp'
  | 'serviceContext'
  | 'container'
  | 'batch'
  | 'workflow'
  | 'none';

export interface ToolRegistryEntry {
  handler: (...args: any[]) => any | Promise<any>;
  requiresService: ServiceType;
  module: string; // Track which module this tool belongs to
}

/**
 * Central registry for all MCP tools
 * This is populated by handler modules registering their tools
 */
class ToolRegistry {
  private tools = new Map<string, ToolRegistryEntry>();

  /**
   * Register a tool with the registry
   */
  register(name: string, entry: ToolRegistryEntry): void {
    if (this.tools.has(name)) {
      console.warn(`Tool "${name}" is already registered. Overwriting.`);
    }
    this.tools.set(name, entry);
  }

  /**
   * Register multiple tools at once
   */
  registerBatch(entries: Record<string, Omit<ToolRegistryEntry, 'module'>>, module: string): void {
    for (const [name, entry] of Object.entries(entries)) {
      this.register(name, { ...entry, module });
    }
  }

  /**
   * Get a tool entry by name
   */
  get(name: string): ToolRegistryEntry | undefined {
    return this.tools.get(name);
  }

  /**
   * Check if a tool exists
   */
  has(name: string): boolean {
    return this.tools.has(name);
  }

  /**
   * Get all registered tool names
   */
  getToolNames(): string[] {
    return Array.from(this.tools.keys());
  }

  /**
   * Get all tools as a record (for backward compatibility)
   */
  getAll(): Record<string, ToolRegistryEntry> {
    const result: Record<string, ToolRegistryEntry> = {};
    for (const [name, entry] of this.tools.entries()) {
      result[name] = entry;
    }
    return result;
  }

  /**
   * Clear all registrations (useful for testing)
   */
  clear(): void {
    this.tools.clear();
  }
}

/**
 * Workflow Registry for managing workflow definitions
 */
class WorkflowRegistry {
  private workflows = new Map<string, WorkflowToolDefinition>();

  /**
   * Register a workflow definition
   */
  register(workflow: WorkflowToolDefinition): void {
    if (this.workflows.has(workflow.name)) {
      console.warn(`Workflow "${workflow.name}" is already registered. Overwriting.`);
    }
    this.workflows.set(workflow.name, workflow);
  }

  /**
   * Register multiple workflows at once
   */
  registerBatch(workflows: WorkflowToolDefinition[]): void {
    for (const workflow of workflows) {
      this.register(workflow);
    }
  }

  /**
   * Get a workflow definition by name
   */
  get(name: string): WorkflowToolDefinition | undefined {
    return this.workflows.get(name);
  }

  /**
   * Check if a workflow exists
   */
  has(name: string): boolean {
    return this.workflows.has(name);
  }

  /**
   * Get all registered workflow names
   */
  getWorkflowNames(): string[] {
    return Array.from(this.workflows.keys());
  }

  /**
   * Get all workflows as an array
   */
  getAll(): WorkflowToolDefinition[] {
    return Array.from(this.workflows.values());
  }

  /**
   * Clear all registrations (useful for testing)
   */
  clear(): void {
    this.workflows.clear();
  }
}

// Export singleton instances
export const toolRegistry = new ToolRegistry();
export const workflowRegistry = new WorkflowRegistry();

// Export convenience functions for tools
export const registerTool = toolRegistry.register.bind(toolRegistry);
export const registerTools = toolRegistry.registerBatch.bind(toolRegistry);
export const getTool = toolRegistry.get.bind(toolRegistry);
export const hasTool = toolRegistry.has.bind(toolRegistry);
export const getToolNames = toolRegistry.getToolNames.bind(toolRegistry);
export const getAllTools = toolRegistry.getAll.bind(toolRegistry);

// Export convenience functions for workflows
export const registerWorkflow = workflowRegistry.register.bind(workflowRegistry);
export const registerWorkflows = workflowRegistry.registerBatch.bind(workflowRegistry);
export const getWorkflow = workflowRegistry.get.bind(workflowRegistry);
export const hasWorkflow = workflowRegistry.has.bind(workflowRegistry);
export const getWorkflowNames = workflowRegistry.getWorkflowNames.bind(workflowRegistry);
export const getAllWorkflows = workflowRegistry.getAll.bind(workflowRegistry);

/**
 * Type guard to check if a tool name refers to a workflow
 */
export function isWorkflowTool(toolName: string): boolean {
  return workflowRegistry.has(toolName);
}

/**
 * Type guard to check if an object is a workflow definition
 */
export function isWorkflowDefinition(obj: unknown): obj is WorkflowToolDefinition {
  return (
    typeof obj === 'object' &&
    obj !== null &&
    'type' in obj &&
    obj.type === 'workflow' &&
    'name' in obj &&
    'steps' in obj &&
    Array.isArray((obj as any).steps)
  );
}
