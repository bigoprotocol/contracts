use near_sdk::{env, near, PanicOnDefault, AccountId};
use near_sdk::store::LookupMap;
use near_sdk::json_types::U128;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    total_supply: u128,
    balances: LookupMap<AccountId, u128>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self { owner, total_supply: 0, balances: LookupMap::new(b"b") }
    }

    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }

    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(*self.balances.get(&account_id).unwrap_or(&0))
    }

    #[payable]
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        let entry = self.balances.get(&account_id).cloned().unwrap_or(0);
        self.balances.insert(account_id, entry + amount.0);
        self.total_supply += amount.0;
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
    fn mint_works() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let mut c = Contract { owner: accounts(0), total_supply: 0, balances: LookupMap::new(b"b") };
        c.mint(accounts(1), U128(1000));
        assert_eq!(c.ft_balance_of(accounts(1)).0, 1000);
        assert_eq!(c.ft_total_supply().0, 1000);
    }
}
