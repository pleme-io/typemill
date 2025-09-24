import type { User } from './types';

const mockDatabase: User[] = [
  { id: '1', name: 'John Doe', email: 'john@example.com', role: 'admin' },
  { id: '2', name: 'Jane Smith', email: 'jane@example.com', role: 'user' },
  { id: '3', name: 'Bob Wilson', email: 'bob@example.com', role: 'user' },
];

export async function findUser(id: string): Promise<User | null> {
  // Simulate database lookup
  await new Promise(resolve => setTimeout(resolve, 10));

  const user = mockDatabase.find(u => u.id === id);
  return user || null;
}

export function validateUser(user: User): boolean {
  if (!user.id || !user.name || !user.email) {
    return false;
  }

  const emailRegex = /^[^\s@]+@[^\s@]+\.[^\s@]+$/;
  return emailRegex.test(user.email);
}

export function getUsersByRole(role: string): User[] {
  return mockDatabase.filter(user => user.role === role);
}

export function formatUserName(user: User): string {
  return `${user.name} (${user.email})`;
}