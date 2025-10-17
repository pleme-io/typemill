import { UserForm } from './components/user-form';
import { UserList } from './components/user-list';
// Main index file for playground
import { DEFAULT_USER, TestProcessor, UserData, UserRole } from '../test-workspace-symbols/service';
import { formatUser } from '../test-workspace-symbols/service';

export class App {
  private processor: TestProcessor;
  private userForm: UserForm;
  private userList: UserList;

  constructor() {
    this.processor = new TestProcessor();
    this.userForm = new UserForm();
    this.userList = new UserList();
  }

  async initialize(): Promise<void> {
    console.log('Initializing app...');

    // Process default user
    const processedUser = this.processor.processUser(DEFAULT_USER);
    console.log('Processed user:', formatUser(processedUser));

    // Initialize components
    await this.userForm.initialize();
    await this.userList.initialize();
  }

  getUserProcessor(): TestProcessor {
    return this.processor;
  }
}

export { TestProcessor, UserData, UserRole };
export default App;