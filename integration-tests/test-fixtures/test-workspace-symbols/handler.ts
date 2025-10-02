export class UserHandler {
  handle(user: UserData): void {
    console.log(user.name);
  }
}

export interface UserData {
  name: string;
  id: number;
}

export function processUser(data: UserData): UserData {
  return data;
}
