import { StringUtils } from './utils';

export class DataService {
  formatData(data: string): string {
    return StringUtils.capitalize(StringUtils.reverse(data));
  }

  processItems(items: string[]): string[] {
    return items.map((item) => this.formatData(item));
  }
}