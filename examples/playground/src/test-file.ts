// Core test file for playground environment
// Contains various TypeScript constructs for testing LSP functionality

export interface UserData {
  id: number;
  name: string;
  email: string;
  age?: number;
  birthDate?: string;
  role?: UserRole;
}

// Private function for calculating age
function _calculateAge(birthDate: Date): number {
  const today = new Date();
  const age = today.getFullYear() - birthDate.getFullYear();
  const monthDiff = today.getMonth() - birthDate.getMonth();

  if (monthDiff < 0 || (monthDiff === 0 && today.getDate() < birthDate.getDate())) {
    return age - 1;
  }

  return age;
}

// Test class for symbol operations
export class TestProcessor {
  private userData: UserData[];

  constructor() {
    this.userData = [];
  }

  processUser(user: UserData): UserData {
    if (user.age === undefined && user.birthDate) {
      user.age = _calculateAge(new Date(user.birthDate));
    }
    return user;
  }

  findUserById(id: number): UserData | undefined {
    return this.userData.find((user) => user.id === id);
  }

  getAllUsers(): UserData[] {
    return [...this.userData];
  }
}

// Constants for testing
export const DEFAULT_USER: UserData = {
  id: 1,
  name: 'Test User',
  email: 'test@example.com',
  age: 25,
};

// Generic function for testing
export function processData<T>(data: T[]): T[] {
  return data.filter((item) => item !== null && item !== undefined);
}

// Arrow function for testing
export const validateEmail = (email: string): boolean => {
  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(email);
};

// Enum for testing
export enum UserRole {
  Admin = 'admin',
  User = 'user',
  Guest = 'guest',
}

// Type alias for testing
export type ProcessorConfig = {
  maxUsers: number;
  enableValidation: boolean;
  defaultRole: UserRole;
};

// Function with multiple parameters for signature help testing
export function createUserProcessor(
  config: ProcessorConfig,
  logger?: (message: string) => void,
  onError?: (error: Error) => void
): TestProcessor {
  const processor = new TestProcessor();

  if (logger) {
    logger('User processor created');
  }

  return processor;
}
