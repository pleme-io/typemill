// test_data/circular.ts

// This creates a circular dependency with complex.ts
import { highComplexityFunction } from './complex';

export class B {
    constructor() {
        console.log(highComplexityFunction(1,2,3));
    }
}
