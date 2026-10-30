use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquadError {
    NotFound,
    AlreadyExists,
    DeadlinePassed,
    NotMember,
    AlreadyBought,
}

impl std::fmt::Display for SquadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
