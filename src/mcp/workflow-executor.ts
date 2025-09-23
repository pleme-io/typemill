/**
 * Workflow Executor - Manages multi-step tool execution with dependency resolution
 * Enables powerful automation by chaining MCP tools together
 */

import { createUserFriendlyErrorMessage, MCPError } from '../core/diagnostics/error-utils.js';
import { getLogger } from '../core/diagnostics/structured-logger.js';
import type { WorkflowToolDefinition } from './handler-types.js';
import { createMCPResponse } from './utils.js';

const logger = getLogger('WorkflowExecutor');

/**
 * Context object that holds results from each workflow step
 */
interface WorkflowContext {
  /** Input arguments passed to the workflow */
  input: Record<string, unknown>;
  /** Results from each completed step, keyed by step ID */
  steps: Record<string, unknown>;
  /** Metadata about the workflow execution */
  metadata: {
    workflowName: string;
    startTime: number;
    currentStep: number;
    totalSteps: number;
  };
}

/**
 * Result of a workflow execution
 */
interface WorkflowResult {
  success: boolean;
  /** Final result (typically the output of the last step) */
  result?: unknown;
  /** All step results for debugging */
  stepResults?: Record<string, unknown>;
  /** Error information if workflow failed */
  error?: string;
  /** Execution metadata */
  metadata: {
    workflowName: string;
    duration: number;
    stepsCompleted: number;
    totalSteps: number;
  };
}

/**
 * Execute a multi-step workflow with dependency resolution
 */
export async function executeWorkflow(
  workflowDef: WorkflowToolDefinition,
  initialArgs: Record<string, unknown>,
  toolExecutor: (toolName: string, args: Record<string, unknown>) => Promise<unknown>
): Promise<WorkflowResult> {
  const startTime = Date.now();

  // Initialize workflow context
  const context: WorkflowContext = {
    input: initialArgs,
    steps: {},
    metadata: {
      workflowName: workflowDef.name,
      startTime,
      currentStep: 0,
      totalSteps: workflowDef.steps.length,
    },
  };

  logger.info('Starting workflow execution', {
    workflow: workflowDef.name,
    steps: workflowDef.steps.length,
    input: JSON.stringify(initialArgs),
  });

  try {
    // Execute each step in sequence
    for (let i = 0; i < workflowDef.steps.length; i++) {
      const step = workflowDef.steps[i]!;
      const stepId = step.id || `step${i + 1}`;

      context.metadata.currentStep = i + 1;

      logger.info('Executing workflow step', {
        workflow: workflowDef.name,
        step: stepId,
        tool: step.tool,
        description: step.description || 'No description',
      });

      try {
        // Resolve placeholders in step arguments
        const resolvedArgs = resolvePlaceholders(step.args, context);

        logger.debug('Step arguments resolved', {
          workflow: workflowDef.name,
          step: stepId,
          originalArgs: JSON.stringify(step.args),
          resolvedArgs: JSON.stringify(resolvedArgs),
        });

        // Execute the tool for this step
        const stepResult = await toolExecutor(step.tool, resolvedArgs);

        // Store the result in context for future steps
        context.steps[stepId] = stepResult;

        logger.info('Workflow step completed', {
          workflow: workflowDef.name,
          step: stepId,
          tool: step.tool,
          resultType: typeof stepResult,
        });
      } catch (error) {
        const mcpError = new MCPError(
          `Workflow step '${stepId}' failed: ${error instanceof Error ? error.message : String(error)}`,
          workflowDef.name,
          'OPERATION_FAILED',
          {
            step: stepId,
            tool: step.tool,
            stepNumber: i + 1,
            totalSteps: workflowDef.steps.length,
          },
          error
        );

        const errorMessage = createUserFriendlyErrorMessage(mcpError, 'workflow execution');

        return {
          success: false,
          error: errorMessage,
          stepResults: context.steps,
          metadata: {
            workflowName: workflowDef.name,
            duration: Date.now() - startTime,
            stepsCompleted: i,
            totalSteps: workflowDef.steps.length,
          },
        };
      }
    }

    // Get the final result (output of the last step)
    const lastStepId =
      workflowDef.steps[workflowDef.steps.length - 1]?.id || `step${workflowDef.steps.length}`;
    const finalResult = context.steps[lastStepId];

    const duration = Date.now() - startTime;

    logger.info('Workflow execution completed', {
      workflow: workflowDef.name,
      duration,
      stepsCompleted: workflowDef.steps.length,
      success: true,
    });

    return {
      success: true,
      result: finalResult,
      stepResults: context.steps,
      metadata: {
        workflowName: workflowDef.name,
        duration,
        stepsCompleted: workflowDef.steps.length,
        totalSteps: workflowDef.steps.length,
      },
    };
  } catch (error) {
    const mcpError = new MCPError(
      `Workflow execution failed: ${error instanceof Error ? error.message : String(error)}`,
      workflowDef.name,
      'INTERNAL_ERROR',
      {
        currentStep: context.metadata.currentStep,
        totalSteps: context.metadata.totalSteps,
      },
      error
    );

    const errorMessage = createUserFriendlyErrorMessage(mcpError, 'workflow execution');

    return {
      success: false,
      error: errorMessage,
      stepResults: context.steps,
      metadata: {
        workflowName: workflowDef.name,
        duration: Date.now() - startTime,
        stepsCompleted: context.metadata.currentStep - 1,
        totalSteps: workflowDef.steps.length,
      },
    };
  }
}

/**
 * Resolve placeholder variables in step arguments using workflow context
 * Supports patterns like {{input.file_path}}, {{step1.result.symbols}}, etc.
 */
function resolvePlaceholders(
  args: Record<string, unknown>,
  context: WorkflowContext
): Record<string, unknown> {
  const resolvedArgs: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(args)) {
    if (typeof value === 'string' && value.includes('{{')) {
      // Handle placeholder resolution
      resolvedArgs[key] = resolvePlaceholderString(value, context);
    } else if (Array.isArray(value)) {
      // Handle arrays that might contain placeholders
      resolvedArgs[key] = value.map((item) =>
        typeof item === 'string' && item.includes('{{')
          ? resolvePlaceholderString(item, context)
          : item
      );
    } else if (value && typeof value === 'object') {
      // Recursively handle nested objects
      resolvedArgs[key] = resolvePlaceholders(value as Record<string, unknown>, context);
    } else {
      // Copy non-placeholder values as-is
      resolvedArgs[key] = value;
    }
  }

  return resolvedArgs;
}

/**
 * Resolve a single placeholder string like "{{input.file_path}}" or "{{step1.result}}"
 */
function resolvePlaceholderString(value: string, context: WorkflowContext): unknown {
  // Extract all placeholders from the string
  const placeholderRegex = /\{\{([^}]+)\}\}/g;
  let resolved = value;

  let match: RegExpExecArray | null = placeholderRegex.exec(value);
  while (match !== null) {
    const placeholder = match[1]!;
    const placeholderValue = resolvePlaceholderPath(placeholder, context);

    // If the entire string is just this placeholder, return the actual value
    if (value === `{{${placeholder}}}`) {
      return placeholderValue;
    }

    // Otherwise, replace the placeholder with its string representation
    resolved = resolved.replace(match[0], String(placeholderValue));
    match = placeholderRegex.exec(value);
  }

  return resolved;
}

/**
 * Resolve a placeholder path like "input.file_path" or "step1.result.symbols"
 */
function resolvePlaceholderPath(path: string, context: WorkflowContext): unknown {
  const parts = path.trim().split('.');

  if (parts.length === 0) {
    throw new Error(`Invalid placeholder path: ${path}`);
  }

  let current: unknown;

  // Start with the root context
  if (parts[0] === 'input') {
    current = context.input;
    parts.shift(); // Remove 'input' from the path
  } else if (parts[0]?.startsWith('step')) {
    const stepId = parts[0];
    current = context.steps[stepId];
    if (current === undefined) {
      throw new Error(
        `Step '${stepId}' not found in workflow context. Available steps: ${Object.keys(context.steps).join(', ')}`
      );
    }
    parts.shift(); // Remove step id from the path
  } else {
    throw new Error(`Invalid placeholder root: ${parts[0]}. Must start with 'input' or 'stepX'`);
  }

  // Navigate through the remaining path
  for (const part of parts) {
    if (current && typeof current === 'object' && part in (current as Record<string, unknown>)) {
      current = (current as Record<string, unknown>)[part];
    } else {
      throw new Error(`Property '${part}' not found in placeholder path: ${path}`);
    }
  }

  return current;
}

/**
 * Create an MCP response for a workflow result
 */
export function createWorkflowResponse(
  result: WorkflowResult
): ReturnType<typeof createMCPResponse> {
  if (result.success) {
    return createMCPResponse(
      `# 🔄 Workflow: ${result.metadata.workflowName}

## ✅ Completed Successfully
- **Duration**: ${result.metadata.duration}ms
- **Steps**: ${result.metadata.stepsCompleted}/${result.metadata.totalSteps}

## 📊 Final Result
${JSON.stringify(result.result, null, 2)}

## 🔍 Step Details
${Object.entries(result.stepResults || {})
  .map(
    ([stepId, stepResult]) =>
      `### ${stepId}
${JSON.stringify(stepResult, null, 2)}`
  )
  .join('\n\n')}

---
*Powered by CodeFlow Buddy Workflow Engine*`
    );
  }
  return createMCPResponse(
    `# ❌ Workflow Failed: ${result.metadata.workflowName}

## Error Details
${result.error}

## Execution Summary
- **Duration**: ${result.metadata.duration}ms
- **Steps Completed**: ${result.metadata.stepsCompleted}/${result.metadata.totalSteps}

## Partial Results
${Object.entries(result.stepResults || {})
  .map(
    ([stepId, stepResult]) =>
      `### ${stepId}
${JSON.stringify(stepResult, null, 2)}`
  )
  .join('\n\n')}

---
*Powered by CodeFlow Buddy Workflow Engine*`
  );
}
