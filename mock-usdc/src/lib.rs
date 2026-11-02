use near_sdk::store::LookupMap;
use near_sdk::{env, near, PanicOnDefault, AccountId, require, Promise, NearToken, Gas};
use near_sdk::json_types::U128;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub total_supply: u128,
    pub balances: LookupMap<AccountId, u128>,
    pub allowances: LookupMap<(AccountId, AccountId), u128>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self { owner, total_supply: 0, balances: LookupMap::new(b"b"), allowances: LookupMap::new(b"a") }
    }
    pub fn ft_total_supply(&self) -> U128 { U128(self.total_supply) }
    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 { U128(*self.balances.get(&account_id).unwrap_or(&0)) }
    pub fn ft_decimals(&self) -> u8 { 6 }
    pub fn ft_metadata(&self) -> FtMetadata {
        FtMetadata { spec: "ft-1.0.0".into(), name: "Mock USDC".into(), symbol: "USDC".into(), icon: Some("data:image/svg+xml;base64,PHN2Zw==".into()), reference: None, reference_hash: None, decimals: 6 }
    }
    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let _ = memo;
        require!(env::attached_deposit().as_yoctonear() == 1, "require 1 yocto");
        let sender = env::predecessor_account_id();
        let sender_bal = *self.balances.get(&sender).unwrap_or(&0);
        require!(sender_bal >= amount.0, "insufficient balance");
        self.balances.insert(sender.clone(), sender_bal.checked_sub(amount.0).expect("underflow"));
        let recv_bal = *self.balances.get(&receiver_id).unwrap_or(&0);
        self.balances.insert(receiver_id, recv_bal.checked_add(amount.0).expect("overflow"));
    }
    #[payable]
    pub fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String) -> Promise {
        self.ft_transfer(receiver_id.clone(), amount, memo);
        Promise::new(receiver_id.clone()).function_call(
            "ft_on_transfer".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "sender_id": env::predecessor_account_id(),
                "amount": amount,
                "msg": msg,
            })).unwrap(),
            NearToken::from_yoctonear(0),
            Gas::from_tgas(20),
        ).then(
            Self::ext(env::current_account_id()).with_static_gas(Gas::from_tgas(10)).ft_resolve_transfer(env::predecessor_account_id(), receiver_id, amount)
        )
    }
    #[private]
    pub fn ft_resolve_transfer(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128, #[callback_result] result: Result<U128, near_sdk::PromiseError>) -> U128 {
        let used = match result {
            Ok(used) => used.0,
            Err(_) => 0,
        };
        if used < amount.0 {
            let refund = amount.0.checked_sub(used).expect("refund underflow");
            let recv_bal = *self.balances.get(&receiver_id).unwrap_or(&0);
            self.balances.insert(receiver_id.clone(), recv_bal.checked_sub(refund).unwrap_or(0));
            let sender_bal = *self.balances.get(&sender_id).unwrap_or(&0);
            self.balances.insert(sender_id, sender_bal.checked_add(refund).expect("refund overflow"));
            U128(used)
        } else {
            amount
        }
    }
    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>) -> StorageBalance {
        let _ = registration_only;
        let account = account_id.unwrap_or_else(|| env::predecessor_account_id());
        // Mock: storage deposit is free, just ensure account is tracked
        if self.balances.get(&account).is_none() {
            self.balances.insert(account, 0);
        }
        StorageBalance { total: U128(1250000000000000000000), available: U128(0) }
    }
    pub fn storage_balance_of(&self, _account_id: AccountId) -> Option<StorageBalance> {
        Some(StorageBalance { total: U128(1250000000000000000000), available: U128(0) })
    }
    #[payable]
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        let bal = *self.balances.get(&account_id).unwrap_or(&0);
        self.balances.insert(account_id, bal.checked_add(amount.0).expect("mint overflow"));
        self.total_supply = self.total_supply.checked_add(amount.0).expect("supply overflow");
    }
    pub fn get_owner(&self) -> AccountId { self.owner.clone() }
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct FtMetadata { pub spec: String, pub name: String, pub symbol: String, pub icon: Option<String>, pub reference: Option<String>, pub reference_hash: Option<String>, pub decimals: u8 }

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct StorageBalance { pub total: U128, pub available: U128 }

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    #[test] fn mint_and_transfer() {
        let ctx = VMContextBuilder::new().predecessor_account_id(accounts(0)).build(); testing_env!(ctx);
        let mut c = Contract::new(accounts(0));
        c.mint(accounts(0), U128(1000));
        assert_eq!(c.ft_balance_of(accounts(0)).0, 1000);
        let ctx2 = VMContextBuilder::new().predecessor_account_id(accounts(0)).attached_deposit(NearToken::from_yoctonear(1)).build(); testing_env!(ctx2);
        c.ft_transfer(accounts(1), U128(100), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 100);
    }
}
