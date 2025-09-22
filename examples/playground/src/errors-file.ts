// File with intentional TypeScript errors for diagnostic testing

export interface ErrorData {
  id: number;
  message: string;
}

// Error: Using undefined variable
export function processError(): ErrorData {
  return {
    id: undefinedVariable, // Error: undefinedVariable is not defined
    message: 'Test error',
  };
}

// Error: Type mismatch
export function getErrorId(): string {
  return 123; // Error: Type 'number' is not assignable to type 'string'
}

// Error: Missing return statement
export function processData(data: unknown[]): ErrorData[] {
  if (data.length > 0) {
    return data.map((item: any) => ({ id: item.id, message: item.msg }));
  }
  // Error: Function lacks ending return statement
}

// Error: Unused parameter
export function handleError(error: Error, context: string): void {
  console.log(error.message); // 'context' parameter is unused
}
