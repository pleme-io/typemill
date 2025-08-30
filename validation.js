// src/mcp/validation.ts
function validateFilePath(args) {
  if (typeof args !== "object" || args === null) {
    return false;
  }
  const obj = args;
  return "file_path" in obj && typeof obj.file_path === "string" && obj.file_path.length > 0;
}
function validatePosition(args) {
  if (typeof args !== "object" || args === null) {
    return false;
  }
  const obj = args;
  return "line" in obj && "character" in obj && typeof obj.line === "number" && typeof obj.character === "number" && Number.isInteger(obj.line) && Number.isInteger(obj.character) && obj.line >= 0 && obj.character >= 0;
}
function validateQuery(args) {
  if (typeof args !== "object" || args === null) {
    return false;
  }
  const obj = args;
  return "query" in obj && typeof obj.query === "string" && obj.query.trim().length > 0;
}
function validateSymbolName(args) {
  if (typeof args !== "object" || args === null) {
    return false;
  }
  const obj = args;
  return "symbol_name" in obj && typeof obj.symbol_name === "string" && obj.symbol_name.trim().length > 0;
}
function createValidationError(fieldName, expectedType) {
  return new Error(`Invalid ${fieldName}: expected ${expectedType}`);
}
export {
  validateSymbolName,
  validateQuery,
  validatePosition,
  validateFilePath,
  createValidationError
};
