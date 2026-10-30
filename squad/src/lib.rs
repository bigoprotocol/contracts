use near_sdk::{env, near, PanicOnDefault, AccountId};

mod error;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self { owner }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    #[test]
    fn owner_is_set() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let c = Contract { owner: accounts(0) };
        assert_eq!(c.get_owner(), accounts(0));
    }
}
