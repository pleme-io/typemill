import { DataService } from '../test-workspace-symbols/service';
import { StringUtils } from './src/utils';

const service = new DataService();
const processed = service.processItems(['hello', 'world']);
const manual = StringUtils.capitalize('test');

console.log('Processed:', processed);
console.log('Manual:', manual);