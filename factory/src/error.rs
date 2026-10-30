use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FactoryError {
    NotOwner,
    AlreadyExists,
    InvalidInput(String),
}

impl std::fmt::Display for FactoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
