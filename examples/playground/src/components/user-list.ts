// User list component
import { TestProcessor, type UserData } from '../core/test-service';
import { formatUser, sortUsersByName } from '../../../../crates/cb-server/src/services/tests.rs';

export class UserList {
  private users: UserData[] = [];
  private processor: TestProcessor;

  constructor() {
    this.processor = new TestProcessor();
  }

  async initialize(): Promise<void> {
    console.log('Initializing user list...');
    await this.loadUsers();
  }

  private async loadUsers(): Promise<void> {
    // Simulate loading users
    this.users = this.processor.getAllUsers();
  }

  addUser(user: UserData): void {
    this.users.push(user);
  }

  removeUser(id: number): boolean {
    const index = this.users.findIndex((user) => user.id === id);
    if (index !== -1) {
      this.users.splice(index, 1);
      return true;
    }
    return false;
  }

  getUsers(): UserData[] {
    return sortUsersByName([...this.users]);
  }

  getUserById(id: number): UserData | undefined {
    return this.users.find((user) => user.id === id);
  }

  displayUsers(): void {
    console.log('User List:');
    for (const user of this.getUsers()) {
      console.log(`- ${formatUser(user)}`);
    }
  }

  filterUsersByAge(minAge: number, maxAge: number): UserData[] {
    return this.users.filter(
      (user) => user.age !== undefined && user.age >= minAge && user.age <= maxAge
    );
  }
}