// Utility functions for user operations
import type { UserData, UserRole } from '../core/test-service';

export function formatUser(user: UserData): string {
  return `${user.name} (${user.email})${user.age ? ` - Age: ${user.age}` : ''}`;
}

export function sortUsersByName(users: UserData[]): UserData[] {
  return users.sort((a, b) => a.name.localeCompare(b.name));
}

export function sortUsersByAge(users: UserData[]): UserData[] {
  return users.sort((a, b) => {
    if (a.age === undefined && b.age === undefined) return 0;
    if (a.age === undefined) return 1;
    if (b.age === undefined) return -1;
    return a.age - b.age;
  });
}

export function getUsersByRole(users: UserData[], role: UserRole): UserData[] {
  return users.filter((user) => user.role === role);
}

export function calculateAverageAge(users: UserData[]): number {
  const usersWithAge = users.filter((user) => user.age !== undefined);
  if (usersWithAge.length === 0) return 0;

  const totalAge = usersWithAge.reduce((sum, user) => sum + (user.age || 0), 0);
  return totalAge / usersWithAge.length;
}

export function validateUserData(user: Partial<UserData>): string[] {
  const errors: string[] = [];

  if (!user.name || user.name.trim() === '') {
    errors.push('Name is required');
  }

  if (!user.email || !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(user.email)) {
    errors.push('Valid email is required');
  }

  if (user.age !== undefined && (user.age < 0 || user.age > 150)) {
    errors.push('Age must be between 0 and 150');
  }

  return errors;
}
