/**
 * Central Tool Registry
 * Single source of truth for all MCP tool mappings
 * This decouples the batch executor from handler implementations
 */

export type ServiceType =
  | 'symbol'
  | 'file'
  | 'diagnostic'
  | 'intelligence'
  | 'hierarchy'
  | 'lsp'
  | 'serviceContext'
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

// Export singleton instance
export const toolRegistry = new ToolRegistry();

// Export convenience functions
export const registerTool = toolRegistry.register.bind(toolRegistry);
export const registerTools = toolRegistry.registerBatch.bind(toolRegistry);
export const getTool = toolRegistry.get.bind(toolRegistry);
export const hasTool = toolRegistry.has.bind(toolRegistry);
export const getToolNames = toolRegistry.getToolNames.bind(toolRegistry);
export const getAllTools = toolRegistry.getAll.bind(toolRegistry);
