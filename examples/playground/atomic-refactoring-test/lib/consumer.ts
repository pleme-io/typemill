import { StringUtils } from '../src/utils';
import { DataService } from '../src/data-service';

export class Consumer {
  private dataService = new DataService();

  consume(input: string): string {
    const reversed = StringUtils.reverse(input);
    return this.dataService.formatData(reversed);
  }
}