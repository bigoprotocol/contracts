use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub total_supply: u128,
    pub balances: LookupMap<AccountId, u128>,
    pub allowances: LookupMap<(AccountId, AccountId), u128>,
    pub fail_transfers: bool,
    pub fail_account: Option<AccountId>,
}

#[near]
impl Contract {
    // deploy and init must be submitted as one batched transaction. the factory
    // does this with create_account + deploy + new in a single promise chain.
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self {
            owner,
            total_supply: 0,
            balances: LookupMap::new(b"b"),
            allowances: LookupMap::new(b"a"),
            fail_transfers: false,
            fail_account: None,
        }
    }
    pub fn ft_total_supply(&self) -> U128 {
        U128(self.total_supply)
    }
    pub fn ft_balance_of(&self, account_id: AccountId) -> U128 {
        U128(*self.balances.get(&account_id).unwrap_or(&0))
    }
    pub fn ft_decimals(&self) -> u8 {
        6
    }
    pub fn ft_metadata(&self) -> FtMetadata {
        FtMetadata {
            spec: "ft-1.0.0".into(),
            name: "Mock USDC".into(),
            symbol: "USDC".into(),
            icon: Some("data:image/svg+xml;base64,PD94bWwgdmVyc2lvbj0iMS4wIiBlbmNvZGluZz0idXRmLTgiPz4KPCEtLSBMaWNlbnNlOiBQRC4gTWFkZSBieSBzcG90aHE6IGh0dHBzOi8vZ2l0aHViLmNvbS9zcG90aHEvY3J5cHRvY3VycmVuY3ktaWNvbnMgLS0+Cjxzdmcgd2lkdGg9IjY0cHgiIGhlaWdodD0iNjRweCIgdmlld0JveD0iMCAwIDMyIDMyIiB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciPgoKPGcgZmlsbD0ibm9uZSI+Cgo8Y2lyY2xlIGZpbGw9IiMzRTczQzQiIGN4PSIxNiIgY3k9IjE2IiByPSIxNiIvPgoKPGcgZmlsbD0iI0ZGRiI+Cgo8cGF0aCBkPSJNMjAuMDIyIDE4LjEyNGMwLTIuMTI0LTEuMjgtMi44NTItMy44NC0zLjE1Ni0xLjgyOC0uMjQzLTIuMTkzLS43MjgtMi4xOTMtMS41NzggMC0uODUuNjEtMS4zOTYgMS44MjgtMS4zOTYgMS4wOTcgMCAxLjcwNy4zNjQgMi4wMTEgMS4yNzVhLjQ1OC40NTggMCAwMC40MjcuMzAzaC45NzVhLjQxNi40MTYgMCAwMC40MjctLjQyNXYtLjA2YTMuMDQgMy4wNCAwIDAwLTIuNzQzLTIuNDg5VjkuMTQyYzAtLjI0My0uMTgzLS40MjUtLjQ4Ny0uNDg2aC0uOTE1Yy0uMjQzIDAtLjQyNi4xODItLjQ4Ny40ODZ2MS4zOTZjLTEuODI5LjI0Mi0yLjk4NiAxLjQ1Ni0yLjk4NiAyLjk3NCAwIDIuMDAyIDEuMjE4IDIuNzkxIDMuNzc4IDMuMDk1IDEuNzA3LjMwMyAyLjI1NS42NjggMi4yNTUgMS42MzkgMCAuOTctLjg1MyAxLjYzOC0yLjAxMSAxLjYzOC0xLjU4NSAwLTIuMTMzLS42NjctMi4zMTYtMS41NzgtLjA2LS4yNDItLjI0NC0uMzY0LS40MjctLjM2NGgtMS4wMzZhLjQxNi40MTYgMCAwMC0uNDI2LjQyNXYuMDZjLjI0MyAxLjUxOCAxLjIxOSAyLjYxIDMuMjMgMi45MTR2MS40NTdjMCAuMjQyLjE4My40MjUuNDg3LjQ4NWguOTE1Yy4yNDMgMCAuNDI2LS4xODIuNDg3LS40ODVWMjEuMzRjMS44MjktLjMwMyAzLjA0Ny0xLjU3OCAzLjA0Ny0zLjIxN3oiLz4KCjxwYXRoIGQ9Ik0xMi44OTIgMjQuNDk3Yy00Ljc1NC0xLjctNy4xOTItNi45OC01LjQyNC0xMS42NTMuOTE0LTIuNTUgMi45MjUtNC40OTEgNS40MjQtNS40MDIuMjQ0LS4xMjEuMzY1LS4zMDMuMzY1LS42MDd2LS44NWMwLS4yNDItLjEyMS0uNDI0LS4zNjUtLjQ4NS0uMDYxIDAtLjE4MyAwLS4yNDQuMDZhMTAuODk1IDEwLjg5NSAwIDAwLTcuMTMgMTMuNzE3YzEuMDk2IDMuNCAzLjcxNyA2LjAxIDcuMTMgNy4xMDIuMjQ0LjEyMS40ODggMCAuNTQ4LS4yNDMuMDYxLS4wNi4wNjEtLjEyMi4wNjEtLjI0M3YtLjg1YzAtLjE4Mi0uMTgyLS40MjQtLjM2NS0uNTQ2em02LjQ2LTE4LjkzNmMtLjI0NC0uMTIyLS40ODggMC0uNTQ4LjI0Mi0uMDYxLjA2MS0uMDYxLjEyMi0uMDYxLjI0M3YuODVjMCAuMjQzLjE4Mi40ODUuMzY1LjYwNyA0Ljc1NCAxLjcgNy4xOTIgNi45OCA1LjQyNCAxMS42NTMtLjkxNCAyLjU1LTIuOTI1IDQuNDkxLTUuNDI0IDUuNDAyLS4yNDQuMTIxLS4zNjUuMzAzLS4zNjUuNjA3di44NWMwIC4yNDIuMTIxLjQyNC4zNjUuNDg1LjA2MSAwIC4xODMgMCAuMjQ0LS4wNmExMC44OTUgMTAuODk1IDAgMDA3LjEzLTEzLjcxN2MtMS4wOTYtMy40Ni0zLjc3OC02LjA3LTcuMTMtNy4xNjJ6Ii8+Cgo8L2c+Cgo8L2c+Cgo8L3N2Zz4=".into()),
            reference: None,
            reference_hash: None,
            decimals: 6,
        }
    }
    pub fn set_fail_transfers(&mut self, fail: bool) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.fail_transfers = fail;
    }

    pub fn set_fail_account(&mut self, account: Option<AccountId>) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.fail_account = account;
    }

    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let _ = memo;
        require!(!self.fail_transfers, "transfers disabled");
        require!(
            self.fail_account.as_ref() != Some(&receiver_id),
            "receiver transfers disabled"
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "require 1 yocto"
        );
        let sender = env::predecessor_account_id();
        let sender_bal = *self.balances.get(&sender).unwrap_or(&0);
        require!(sender_bal >= amount.0, "insufficient balance");
        self.balances.insert(
            sender.clone(),
            sender_bal.checked_sub(amount.0).expect("underflow"),
        );
        let recv_bal = *self.balances.get(&receiver_id).unwrap_or(&0);
        self.balances.insert(
            receiver_id,
            recv_bal.checked_add(amount.0).expect("overflow"),
        );
    }
    #[payable]
    pub fn ft_transfer_call(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
        msg: String,
    ) -> Promise {
        self.ft_transfer(receiver_id.clone(), amount, memo);
        Promise::new(receiver_id.clone())
            .function_call(
                "ft_on_transfer".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "sender_id": env::predecessor_account_id(),
                    "amount": amount,
                    "msg": msg,
                }))
                .unwrap(),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(120),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(20))
                    .ft_resolve_transfer(env::predecessor_account_id(), receiver_id, amount),
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
                sender_id,
                sender_bal.checked_add(refund).expect("refund overflow"),
            );
        }
        U128(amount.0 - refund)
    }
    #[payable]
    // test-only token: registration is lenient so demo buyers need one deposit
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> StorageBalance {
        let _ = registration_only;
        let account = account_id.unwrap_or_else(env::predecessor_account_id);
        // Mock: storage deposit is free, just ensure account is tracked
        if self.balances.get(&account).is_none() {
            self.balances.insert(account, 0);
        }
        StorageBalance {
            total: U128(1250000000000000000000),
            available: U128(0),
        }
    }
    pub fn storage_balance_of(&self, _account_id: AccountId) -> Option<StorageBalance> {
        Some(StorageBalance {
            total: U128(1250000000000000000000),
            available: U128(0),
        })
    }
    #[payable]
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        let bal = *self.balances.get(&account_id).unwrap_or(&0);
        self.balances.insert(
            account_id,
            bal.checked_add(amount.0).expect("mint overflow"),
        );
        self.total_supply = self
            .total_supply
            .checked_add(amount.0)
            .expect("supply overflow");
    }
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct FtMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;
    #[test]
    fn mint_and_transfer() {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let mut c = Contract::new(accounts(0));
        c.mint(accounts(0), U128(1000));
        assert_eq!(c.ft_balance_of(accounts(0)).0, 1000);
        let ctx2 = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx2);
        c.ft_transfer(accounts(1), U128(100), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 100);
    }
}
