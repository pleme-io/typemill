/// Simple Rust file for testing refactoring operations
pub struct Calculator {
    pub value: i32,
}

impl Calculator {
    pub fn new(value: i32) -> Self {
        Calculator { value }
    }

    pub fn add(&mut self, amount: i32) -> i32 {
        self.value += amount;
        self.value
    }

    pub fn multiply(&mut self, factor: i32) -> i32 {
        self.value *= factor;
        self.value
    }
}

pub fn process_data(input: i32) -> i32 {
    let mut calc = Calculator::new(input);
    calc.add(10);
    calc.multiply(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculator() {
        let mut calc = Calculator::new(5);
        assert_eq!(calc.add(3), 8);
        assert_eq!(calc.multiply(2), 16);
    }
}
