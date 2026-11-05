use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

pub const TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
pub const DECIMALS: u8 = 6;
pub const MIN_STORAGE_DEPOSIT: u128 = 1_250_000_000_000_000_000_000;

#[near(serializers = [json, borsh])]
#[derive(Clone, Debug)]
pub struct FungibleTokenMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,
}

#[near(serializers = [json])]
#[derive(Clone, Debug)]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub total_supply: u128,
    pub metadata: FungibleTokenMetadata,
    pub balances: LookupMap<AccountId, u128>,
    pub storage: LookupMap<AccountId, u128>,
}

#[near]
impl Contract {
    // deploy and init must be submitted as one batched transaction. the factory
    // does this with create_account + deploy + new in a single promise chain.
    #[init]
    pub fn new(owner: AccountId, total_supply: U128, metadata: FungibleTokenMetadata) -> Self {
        assert!(!env::state_exists(), "already initialized");
        require!(total_supply.0 > 0, "total supply must be > 0");
        require!(
            metadata.decimals == DECIMALS,
            format!("decimals must be {}", DECIMALS)
        );
        let mut balances = LookupMap::new(b"b");
        balances.insert(owner.clone(), total_supply.0);
        // the owner holds the supply and must be able to receive transfers
        let mut storage = LookupMap::new(b"s");
        storage.insert(owner.clone(), 0);
        Self::emit_ft_mint(&owner, total_supply.0);
        Self {
            owner,
            total_supply: total_supply.0,
            metadata,
            balances,
            storage,
        }
    }

    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }

    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(*self.balances.get(&account_id).unwrap_or(&0))
    }

    pub fn ft_metadata(&self) -> FungibleTokenMetadata {
        self.metadata.clone()
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    #[payable]
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let _ = registration_only;
        let account = account_id.unwrap_or_else(env::predecessor_account_id);
        require!(
            account == env::predecessor_account_id() || env::predecessor_account_id() == self.owner,
            "only self or owner can register"
        );
        let amount = env::attached_deposit().as_yoctonear();
        let existing = self.storage.get(&account).cloned();
        if existing.is_none() {
            require!(
                amount >= MIN_STORAGE_DEPOSIT,
                format!("minimum storage deposit is {} yocto", MIN_STORAGE_DEPOSIT)
            );
        }
        let total = existing
            .unwrap_or(0)
            .checked_add(amount)
            .expect("storage overflow");
        self.storage.insert(account.clone(), total);
        StorageBalance {
            total: U128(total),
            available: U128(0),
        }
    }

    pub fn storage_unregister(&mut self, force: Option<bool>) -> Promise {
        let _ = force;
        let account = env::predecessor_account_id();
        let balance = *self.balances.get(&account).unwrap_or(&0);
        require!(balance == 0, "cannot unregister with a token balance");
        let storage = self.storage.get(&account).cloned().unwrap_or(0);
        self.storage.remove(&account);
        Promise::new(account).transfer(NearToken::from_yoctonear(storage))
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.storage.get(&account_id).map(|total| StorageBalance {
            total: U128(*total),
            available: U128(0),
        })
    }

    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let _ = memo;
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let sender = env::predecessor_account_id();
        self.transfer_internal(&sender, &receiver_id, amount.0);
    }

    #[payable]
    pub fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        let _ = memo;
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let sender = env::predecessor_account_id();
        self.transfer_internal(&sender, &receiver_id, amount.0);
        Promise::new(receiver_id.clone())
            .function_call(
                "ft_on_transfer".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "sender_id": sender,
                    "amount": amount,
                    "msg": msg,
                }))
                .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(120),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(20))
                    .ft_resolve_transfer(sender, receiver_id, amount),
            )
    }

    #[private]
    pub fn ft_resolve_transfer(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) -> U128 {
        // ft_on_transfer returns the amount to refund, not the amount used
        let unused = match result {
            Ok(unused) => unused.0.min(amount.0),
            Err(_) => amount.0,
        };
        // never mint: a refund can only move coins the receiver still holds
        let recv_bal = *self.balances.get(&receiver_id).unwrap_or(&0);
        let refund = unused.min(recv_bal);
        if refund > 0 {
            self.balances.insert(receiver_id.clone(), recv_bal - refund);
            let sender_bal = *self.balances.get(&sender_id).unwrap_or(&0);
            self.balances.insert(
                sender_id.clone(),
                sender_bal.checked_add(refund).expect("refund overflow"),
            );
            Self::emit_ft_transfer(&receiver_id, &sender_id, refund);
        }
        U128(amount.0 - refund)
    }

    fn transfer_internal(&mut self, sender: &AccountId, receiver: &AccountId, amount: u128) {
        if amount == 0 {
            return;
        }
        require!(
            self.storage.get(receiver).is_some(),
            format!("receiver {} is not registered", receiver)
        );
        let sender_bal = *self.balances.get(sender).unwrap_or(&0);
        require!(sender_bal >= amount, "insufficient balance");
        self.balances.insert(sender.clone(), sender_bal - amount);
        let recv_bal = *self.balances.get(receiver).unwrap_or(&0);
        self.balances.insert(
            receiver.clone(),
            recv_bal.checked_add(amount).expect("balance overflow"),
        );
        Self::emit_ft_transfer(sender, receiver, amount);
    }

    fn emit_ft_transfer(old_owner: &AccountId, new_owner: &AccountId, amount: u128) {
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep141",
                "version": "1.0.0",
                "event": "ft_transfer",
                "data": [{
                    "old_owner_id": old_owner,
                    "new_owner_id": new_owner,
                    "amount": amount.to_string(),
                }],
            })
        ));
    }

    fn emit_ft_mint(owner: &AccountId, amount: u128) {
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "nep141",
                "version": "1.0.0",
                "event": "ft_mint",
                "data": [{
                    "owner_id": owner,
                    "amount": amount.to_string(),
                }],
            })
        ));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn metadata() -> FungibleTokenMetadata {
        FungibleTokenMetadata {
            spec: "ft-1.0.0".into(),
            name: "Bigo Coin".into(),
            symbol: "BIGO".into(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: DECIMALS,
        }
    }

    #[test]
    fn mints_entire_supply_to_owner() {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let c = Contract::new(accounts(0), U128(TOTAL_SUPPLY), metadata());
        assert_eq!(c.ft_total_supply().0, TOTAL_SUPPLY);
        assert_eq!(c.ft_balance_of(accounts(0)).0, TOTAL_SUPPLY);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 0);
    }

    #[test]
    fn transfers_work() {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let mut c = Contract::new(accounts(0), U128(TOTAL_SUPPLY), metadata());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(MIN_STORAGE_DEPOSIT))
            .build();
        testing_env!(ctx);
        c.storage_deposit(Some(accounts(1)), None);
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        c.ft_transfer(accounts(1), U128(1000), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 1000);
        assert_eq!(c.ft_balance_of(accounts(0)).0, TOTAL_SUPPLY - 1000);
    }

    #[test]
    #[should_panic(expected = "insufficient balance")]
    fn overspend_reverts() {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let mut c = Contract::new(accounts(0), U128(TOTAL_SUPPLY), metadata());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(MIN_STORAGE_DEPOSIT))
            .build();
        testing_env!(ctx);
        c.storage_deposit(Some(accounts(2)), None);
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        c.ft_transfer(accounts(2), U128(1), None);
    }

    #[test]
    fn resolver_never_mints() {
        let mut c = Contract::new(accounts(0), U128(TOTAL_SUPPLY), metadata());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(MIN_STORAGE_DEPOSIT))
            .build();
        testing_env!(ctx);
        c.storage_deposit(Some(accounts(1)), None);
        c.storage_deposit(Some(accounts(2)), None);
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        c.ft_transfer(accounts(1), U128(1000), None);
        // the receiver spends part of the amount before returning a large unused value
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        c.ft_transfer(accounts(2), U128(600), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 400);
        assert_eq!(c.ft_balance_of(accounts(2)).0, 600);

        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let used = c.ft_resolve_transfer(accounts(0), accounts(1), U128(1000), Ok(U128(1000)));
        assert_eq!(used.0, 600, "only the refundable part counts as unused");
        assert_eq!(c.ft_balance_of(accounts(1)).0, 0);
        assert_eq!(c.ft_balance_of(accounts(0)).0, TOTAL_SUPPLY - 600);
        assert_eq!(
            c.ft_total_supply().0,
            TOTAL_SUPPLY,
            "resolver must never mint"
        );
    }

    #[test]
    fn no_mint_path_after_init() {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let c = Contract::new(accounts(0), U128(TOTAL_SUPPLY), metadata());
        assert_eq!(c.total_supply, TOTAL_SUPPLY);
    }
}
