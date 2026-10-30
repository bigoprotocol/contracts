use near_sdk::{env, near, PanicOnDefault, AccountId};

mod error;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    machines: Vec<AccountId>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self { owner, machines: Vec::new() }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn get_machines(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let start = from_index.unwrap_or(0) as usize;
        let lim = limit.unwrap_or(50) as usize;
        self.machines.iter().skip(start).take(lim).cloned().collect()
    }

    pub fn get_machine_count(&self) -> u64 {
        self.machines.len() as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn setup() -> Contract {
        let context = VMContextBuilder::new().build();
        testing_env!(context);
        Contract { owner: accounts(0), machines: vec![] }
    }

    #[test]
    fn owner_is_set() {
        let c = setup();
        assert_eq!(c.get_owner(), accounts(0));
    }
}
