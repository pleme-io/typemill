export interface User {
  id: string;
  name: string;
  email: string;
  role: 'admin' | 'user' | 'guest';
}

export interface UserSearchCriteria {
  name?: string;
  email?: string;
  role?: string;
}

export interface UserStats {
  totalUsers: number;
  activeUsers: number;
  usersByRole: Record<string, number>;
}

export type UserRole = User['role'];

export interface CreateUserRequest {
  name: string;
  email: string;
  role: UserRole;
}