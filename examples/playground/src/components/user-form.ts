// User form component
import { type UserData, validateEmail } from '../test-file';
import { formatUser } from '../utils/user-helpers';

export class UserForm {
  private formData: Partial<UserData> = {};

  async initialize(): Promise<void> {
    console.log('Initializing user form...');
  }

  setFormData(data: Partial<UserData>): void {
    this.formData = { ...data };
  }

  validateForm(): boolean {
    if (!this.formData.name || this.formData.name.trim() === '') {
      return false;
    }

    if (!this.formData.email || !validateEmail(this.formData.email)) {
      return false;
    }

    return true;
  }

  async submitForm(): Promise<UserData | null> {
    if (!this.validateForm()) {
      return null;
    }

    const userData: UserData = {
      id: Date.now(),
      name: this.formData.name || '',
      email: this.formData.email || '',
      age: this.formData.age,
    };

    console.log('Submitting user:', formatUser(userData));
    return userData;
  }

  resetForm(): void {
    this.formData = {};
  }
}
