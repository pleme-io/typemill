/**
 * Conditional test logging that respects LOG_LEVEL environment variable
 */

const shouldLog = () => {
  const logLevel = process.env.LOG_LEVEL;
  return !logLevel || logLevel === 'DEBUG' || logLevel === 'INFO';
};

export const testLog = (...args: unknown[]) => {
  if (shouldLog()) {
    console.log(...args);
  }
};

export const testInfo = (...args: unknown[]) => {
  if (shouldLog()) {
    console.info(...args);
  }
};

export const testWarn = (...args: unknown[]) => {
  console.warn(...args);
};

export const testError = (...args: unknown[]) => {
  console.error(...args);
};
