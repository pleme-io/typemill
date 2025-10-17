// Simple TypeScript file for testing refactoring operations

export interface User {
    id: number;
    name: string;
    email: string;
}

export class UserService {
    private users: User[] = [];

    addUser(user: User): void {
        this.users.push(user);
    }

    findUser(id: number): User | undefined {
        return this.users.find(u => u.id === id);
    }

    updateUser(id: number, updates: Partial<User>): boolean {
        const user = this.findUser(id);
        if (user) {
            Object.assign(user, updates);
            return true;
        }
        return false;
    }
}

export function processUsers(users: User[]): number {
    let total = 0;
    for (const user of users) {
        total += user.id;
    }
    return total;
}
