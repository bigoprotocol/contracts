use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MachineError {
    AlreadyInitialized,
    InvalidPrizeTable(String),
    InvalidArtUrl(String),
    EmptyPrizeTable,
    EvCapExceeded { ev: u64, cap: u64 },
    OddsSumInvalid { got: u64 },
    NotOwner,
    Paused,
    InsufficientPool { free: String, needed: String },
    InvalidLimits(String),
    DrawNotFound,
    PackNotFound,
    NotPackOwner,
    PackAlreadyOpened,
    DrawNotCommitted,
    DrawNotVrfRequested,
    DrawAlreadySettled,
    VrfNotReady,
    NoClaimable,
    VrfPubkeyEmpty,
    StorageTooLow,
}

impl std::fmt::Display for MachineError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
