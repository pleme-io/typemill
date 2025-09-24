export class OuterClass {
  public middleClass = class MiddleClass {
    private middleValue = 2;

    public innerClass = class InnerClass {
      private innerValue = 3;

      public deepestClass = class DeepestClass {
        private deepestValue = 4;

        public executeNested = () => {
          const level1 = () => {
            const level2 = () => {
              const level3 = () => {
                const level4 = () => {
                  const level5 = () => {
                    return 'deeply nested';
                  };
                  return level5();
                };
                return level4();
              };
              return level3();
            };
            return level2();
          };
          return level1();
        };
      };
    };

    public createInner() {
      return new this.innerClass();
    }
  };

  public createMiddle() {
    return new this.middleClass();
  }

  public static createFactory() {
    return function factoryLevel1() {
      return function factoryLevel2() {
        return function factoryLevel3() {
          return function factoryLevel4() {
            return new OuterClass();
          };
        };
      };
    };
  }
}

// Deeply nested interfaces
export interface Level1 {
  level2: {
    level3: {
      level4: {
        level5: {
          level6: {
            value: string;
          };
        };
      };
    };
  };
}

// Deeply nested namespaces
export namespace Outer {
  export namespace Middle {
    export namespace Inner {
      export namespace Deeper {
        export namespace Deepest {
          export const VALUE = 'nested';

          export function deepFunction() {
            return VALUE;
          }
        }
      }
    }
  }
}

// Deeply nested type definitions
export type DeepType<T> = {
  value: T;
  next?: DeepType<DeepType<DeepType<DeepType<T>>>>;
};

// Complex generic constraints
export class GenericNesting<
  T extends { a: U },
  U extends { b: V },
  V extends { c: W },
  W extends { d: string },
> {
  constructor(public value: T) {}
}
