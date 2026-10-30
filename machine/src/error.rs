use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineError {
    AlreadyInitialized,
    InvalidPrizeTable(String),
    InvalidArtUrl,
    NotOwner,
    Paused,
    InsufficientPool,
    InvalidLimits,
    NotFound,
    NotPackOwner,
    AlreadyOpened,
    VrfNotReady,
    NoClaimable,
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
