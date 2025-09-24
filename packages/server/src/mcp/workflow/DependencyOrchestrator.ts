import type { ServiceContext } from '../../services/service-context.js';
import { getTool } from '../tool-registry.js';
import type { ToolChain, WorkflowContext } from './types.js';

export class DependencyOrchestrator {
  constructor(private serviceContext: ServiceContext) {}

  async execute(chain: ToolChain, inputs: Record<string, any>): Promise<any> {
    const context: WorkflowContext = {
      results: new Map(),
      inputs,
    };

    for (const step of chain.steps) {
      const resolvedArgs = this.resolveArgs(step.args, context);
      const toolInfo = getTool(step.tool);
      if (!toolInfo) {
        throw new Error(`Unknown tool in workflow: ${step.tool}`);
      }

      // Execute the tool with proper service requirements
      const result = await this.executeStep(toolInfo, resolvedArgs);
      const stepId = step.id || step.tool;
      context.results.set(stepId, result);
    }

    return context.results.get(chain.steps[chain.steps.length - 1]?.id || '');
  }

  private async executeStep(toolInfo: any, args: any): Promise<any> {
    const serviceArg = this.getServiceArgument(toolInfo.requiresService);

    // Call the handler with appropriate service pattern
    if (toolInfo.requiresService === 'none') {
      return await toolInfo.handler(args);
    }
    if (toolInfo.requiresService === 'lsp') {
      // We don't have direct access to lspClient, use serviceContext
      return await toolInfo.handler(args, this.serviceContext);
    }
    if (toolInfo.requiresService === 'serviceContext') {
      return await toolInfo.handler(args, this.serviceContext);
    }
    return await toolInfo.handler(serviceArg, args);
  }

  private getServiceArgument(serviceType: string): unknown {
    // Note: We don't have direct access to individual services in this context
    // In a real implementation, these would be injected via the ServiceContext
    // For now, return the serviceContext which contains access to all services
    switch (serviceType) {
      case 'symbol':
      case 'file':
      case 'diagnostic':
      case 'intelligence':
      case 'hierarchy':
      case 'lsp':
      case 'serviceContext':
        return this.serviceContext;
      default:
        return undefined;
    }
  }

  private resolveArgs(args: Record<string, any>, context: WorkflowContext): any {
    const resolved: Record<string, any> = {};
    for (const key in args) {
      const value = args[key];
      if (typeof value === 'string' && value.startsWith('$.')) {
        // Simple resolver: $.inputs.filePath or $.step1.result.references
        const path = value.substring(2).split('.');
        let currentValue: any = context;
        for (const part of path) {
          currentValue = currentValue?.[part];
        }
        resolved[key] = currentValue;
      } else {
        resolved[key] = value;
      }
    }
    return resolved;
  }
}
