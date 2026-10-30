use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenError {
    InsufficientBalance,
    InvalidAmount,
}

impl std::fmt::Display for TokenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
