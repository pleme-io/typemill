import { DataService } from '../src/data-service';
import { StringUtils } from '../src/utils';

export class Consumer {
  private dataService = new DataService();

  consume(input: string): string {
    const reversed = StringUtils.reverse(input);
    return this.dataService.formatData(reversed);
  }
}
