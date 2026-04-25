use std::ops::Add;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenUsage {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_creation_input_tokens: u64,
    pub cache_read_input_tokens: u64,
}

impl TokenUsage {
    #[must_use]
    pub fn zero() -> Self {
        Self {
            input_tokens: 0,
            output_tokens: 0,
            cache_creation_input_tokens: 0,
            cache_read_input_tokens: 0,
        }
    }

    #[must_use]
    pub fn total(&self) -> u64 {
        self.input_tokens + self.output_tokens
    }
}

impl Add for TokenUsage {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            input_tokens: self.input_tokens + rhs.input_tokens,
            output_tokens: self.output_tokens + rhs.output_tokens,
            cache_creation_input_tokens: self.cache_creation_input_tokens
                + rhs.cache_creation_input_tokens,
            cache_read_input_tokens: self.cache_read_input_tokens + rhs.cache_read_input_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_returns_all_fields_zero() {
        let usage = TokenUsage::zero();

        assert_eq!(
            usage,
            TokenUsage {
                input_tokens: 0,
                output_tokens: 0,
                cache_creation_input_tokens: 0,
                cache_read_input_tokens: 0,
            }
        );
    }

    #[test]
    fn total_sums_input_and_output() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 10,
            cache_read_input_tokens: 20,
        };

        assert_eq!(usage.total(), 150);
    }

    #[test]
    fn add_sums_all_fields() {
        let a = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 10,
            cache_read_input_tokens: 20,
        };
        let b = TokenUsage {
            input_tokens: 200,
            output_tokens: 100,
            cache_creation_input_tokens: 30,
            cache_read_input_tokens: 40,
        };

        let result = a + b;

        assert_eq!(
            result,
            TokenUsage {
                input_tokens: 300,
                output_tokens: 150,
                cache_creation_input_tokens: 40,
                cache_read_input_tokens: 60,
            }
        );
    }

    #[test]
    fn add_with_zero_is_identity() {
        let usage = TokenUsage {
            input_tokens: 100,
            output_tokens: 50,
            cache_creation_input_tokens: 10,
            cache_read_input_tokens: 20,
        };

        let result = usage.clone() + TokenUsage::zero();

        assert_eq!(result, usage);
    }
}
