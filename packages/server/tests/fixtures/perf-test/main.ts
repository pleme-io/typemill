import { findUser, validateUser } from './utils';
import type { User } from './types';

export class UserManager {
  private users: Map<string, User> = new Map();

  async getUser(id: string): Promise<User | null> {
    const cached = this.users.get(id);
    if (cached) {
      return cached;
    }

    // This is the function we'll test find_definition on
    const user = await findUser(id);
    if (user && validateUser(user)) {
      this.users.set(id, user);
      return user;
    }

    return null;
  }

  async createUser(userData: Omit<User, 'id'>): Promise<User> {
    const user: User = {
      id: Math.random().toString(36),
      ...userData
    };

    if (validateUser(user)) {
      this.users.set(user.id, user);
      return user;
    }

    throw new Error('Invalid user data');
  }
}