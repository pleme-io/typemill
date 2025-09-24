// Large file for performance testing - 100 lines
// This file tests LSP server performance with larger files

export class LargeClass {
  // Methods
  public method0(param: string): string {
    const result = `${param}_processed_0`;
    return result;
  }

  public method1(param: string): string {
    const result = `${param}_processed_1`;
    return result;
  }

  public method2(param: string): string {
    const result = `${param}_processed_2`;
    return result;
  }

  public method3(param: string): string {
    const result = `${param}_processed_3`;
    return result;
  }

  public method4(param: string): string {
    const result = `${param}_processed_4`;
    return result;
  }
}

// Standalone functions
export function standaloneFunction0(arg: string): string {
  const temp = arg || 'default';
  return `result_${temp}_0`;
}

export function standaloneFunction1(arg: string): string {
  const temp = arg || 'default';
  return `result_${temp}_1`;
}

export function standaloneFunction2(arg: string): string {
  const temp = arg || 'default';
  return `result_${temp}_2`;
}

export function standaloneFunction3(arg: string): string {
  const temp = arg || 'default';
  return `result_${temp}_3`;
}

export function standaloneFunction4(arg: string): string {
  const temp = arg || 'default';
  return `result_${temp}_4`;
}

// Type definitions
export type Type0 = { id: number; name: string; data: unknown; index: 0 };
export type Type1 = { id: number; name: string; data: unknown; index: 1 };
export type Type2 = { id: number; name: string; data: unknown; index: 2 };
export type Type3 = { id: number; name: string; data: unknown; index: 3 };
export type Type4 = { id: number; name: string; data: unknown; index: 4 };

// Constants
export const CONSTANT_000 = '000';
export const CONSTANT_001 = '001';
export const CONSTANT_002 = '002';
export const CONSTANT_003 = '003';
export const CONSTANT_004 = '004';

// Additional class for complexity
export class MiddleClass {
  public middleMethod0(): void {
    console.log('middle0');
  }
  public middleMethod1(): void {
    console.log('middle1');
  }
  public middleMethod2(): void {
    console.log('middle2');
  }
  public middleMethod3(): void {
    console.log('middle3');
  }
  public middleMethod4(): void {
    console.log('middle4');
  }
}

// Final class at end of file
export class FinalClass {
  constructor() {
    console.log('End of large file');
  }

  public finalMethod(): string {
    return 'This is the last method in a large file';
  }
}

export const FILE_END = true;
