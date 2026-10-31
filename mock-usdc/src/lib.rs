use near_sdk::store::LookupMap;
use near_sdk::{env, near, PanicOnDefault, AccountId, require};
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
        Self {
            owner,
            total_supply: 0,
            balances: LookupMap::new(b"b"),
            allowances: LookupMap::new(b"a"),
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
            spec: "ft-1.0.0".to_string(),
            name: "Mock USDC".to_string(),
            symbol: "USDC".to_string(),
            icon: "data:image/svg+xml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIGZpbGw9Im5vbmUiIHZpZXdCb3g9IjAgMCAxNiAxNiIgaWQ9IlVzZGMtLVN0cmVhbWxpbmUtQ3J5cHRvY3VycmVuY3kiIGhlaWdodD0iNjQiIHdpZHRoPSI2NCI+CiAgICA8cGF0aCBmaWxsPSIjM2U3M2M0IiBkPSJNOCAxNmM0LjQxODMgMCA4IC0zLjU4MTcgOCAtOCAwIC00LjQxODI4IC0zLjU4MTcgLTggLTggLThDMy41ODE3MiAwIDAgMy41ODE3MiAwIDhjMCA0LjQxODMgMy41ODE3MiA4IDggOFoiIHN0cm9rZS13aWR0aD0iMC41Ij4KPC9wYXRoPgogICAgPHBhdGggZmlsbD0iI2ZmZmZmZiIgZD0iTTEwLjAxMTA1IDkuMDYyYzAgLTEuMDYyIC0wLjY0IC0xLjQyNiAtMS45MiAtMS41NzggLTAuOTE0IC0wLjEyMTUgLTEuMDk2NSAtMC4zNjQgLTEuMDk2NSAtMC43ODkgMCAtMC40MjUgMC4zMDUgLTAuNjk4IDAuOTE0IC0wLjY5OCAwLjU0ODUgMCAwLjg1MzUgMC4xODIgMS4wMDU1IDAuNjM3NSAwLjAxNTggMC4wNDQwNSAwLjA0NDc1IDAuMDgyMiAwLjA4Mjk1IDAuMTA5MjUgMC4wMzgxNSAwLjAyNzEgMC4wODM3IDAuMDQxODUgMC4xMzA1NSAwLjA0MjI1aDAuNDg3NWMwLjAyODE1IDAuMDAwNzUgMC4wNTYxNSAtMC4wMDQyIDAuMDgyMzUgLTAuMDE0NiAwLjAyNjE1IC0wLjAxMDQgMC4wNDk5NSAtMC4wMjYwNSAwLjA2OTkgLTAuMDQ1OSAwLjAxOTk1IC0wLjAxOTg1IDAuMDM1NyAtMC4wNDM2IDAuMDQ2MiAtMC4wNjk3IDAuMDEwNTUgLTAuMDI2MTUgMC4wMTU2NSAtMC4wNTQxNSAwLjAxNTA1IC0wLjA4MjN2LTAuMDNjLTAuMDU5NiAtMC4zMjk1NSAtMC4yMjYzNSAtMC42MzAyIC0wLjQ3NDM1IC0wLjg1NTI1IC0wLjI0OCAtMC4yMjUwNSAtMC41NjM0IC0wLjM2MTg1IC0wLjg5NzE1IC0wLjM4OTI1VjQuNTcxMDA1YzAgLTAuMTIxNSAtMC4wOTE1IC0wLjIxMjUgLTAuMjQzNSAtMC4yNDNoLTAuNDU3NWMtMC4xMjE1IDAgLTAuMjEzIDAuMDkxIC0wLjI0MzUgMC4yNDNWNS4yNjljLTAuOTE0NSAwLjEyMSAtMS40OTMgMC43MjggLTEuNDkzIDEuNDg3IDAgMS4wMDEgMC42MDkgMS4zOTU1IDEuODg5IDEuNTQ3NSAwLjg1MzUgMC4xNTE1IDEuMTI3NSAwLjMzNCAxLjEyNzUgMC44MTk1IDAgMC40ODUgLTAuNDI2NSAwLjgxOSAtMS4wMDU1IDAuODE5IC0wLjc5MjUgMCAtMS4wNjY1IC0wLjMzMzUgLTEuMTU4IC0wLjc4OSAtMC4wMyAtMC4xMjEgLTAuMTIyIC0wLjE4MiAtMC4yMTM1IC0wLjE4MmgtMC41MThjLTAuMDI4MTUgLTAuMDAwNyAtMC4wNTYxIDAuMDA0MzUgLTAuMDgyMiAwLjAxNDggLTAuMDI2MTUgMC4wMTA0IC0wLjA0OTg1IDAuMDI2MDUgLTAuMDY5OCAwLjA0NTkgLTAuMDE5OSAwLjAxOTg1IC0wLjAzNTU1IDAuMDQzNTUgLTAuMDQ2MDUgMC4wNjk2NSAtMC4wMTA1IDAuMDI2MSAtMC4wMTU2IDAuMDU0MDUgLTAuMDE0OTUgMC4wODIxNXYwLjAzYzAuMTIxNSAwLjc1OSAwLjYwOTUgMS4zMDUgMS42MTUgMS40NTd2MC43Mjg1YzAgMC4xMjEgMC4wOTE1IDAuMjEyNSAwLjI0MzUgMC4yNDI1aDAuNDU3NWMwLjEyMTUgMCAwLjIxMyAtMC4wOTEgMC4yNDM1IC0wLjI0MjVWMTAuNjdjMC45MTQ1IC0wLjE1MTUgMS41MjM1IC0wLjc4OSAxLjUyMzUgLTEuNjA4NXYwLjAwMDVaIiBzdHJva2Utd2lkdGg9IjAuNSI+CjwvcGF0aD4KICAgIDxwYXRoIGZpbGw9IiNmZmZmZmYiIGQ9Ik02LjQ0NiAxMi4yNDg1Yy0yLjM3Njk4IC0wLjg1IC0zLjU5NTk4IC0zLjQ5IC0yLjcxMTk4IC01LjgyNjUgMC40NTcgLTEuMjc1IDEuNDYyNDggLTIuMjQ1NSAyLjcxMTk4IC0yLjcwMSAwLjEyMiAtMC4wNjA1IDAuMTgyNSAtMC4xNTE1IDAuMTgyNSAtMC4zMDM1di0wLjQyNWMwIC0wLjEyMSAtMC4wNjA1IC0wLjIxMiAtMC4xODI1IC0wLjI0MjUgLTAuMDMwNSAwIC0wLjA5MTUgMCAtMC4xMjIgMC4wMyAtMC42ODU3NSAwLjIxNDE2IC0xLjMyMjQgMC41NjE4NjUgLTEuODczMjcgMS4wMjMwODUgLTAuNTUwODU1IDAuNDYxMjI1IC0xLjAwNTAzIDEuMDI2ODU1IC0xLjMzNjM4NSAxLjY2NDMxNSAtMC4zMzEzNTUgMC42Mzc1IC0wLjUzMzM0IDEuMzM0MiAtMC41OTQzMiAyLjA1MDA1IC0wLjA2MDk4IDAuNzE1ODUgMC4wMjAyNDUgMS40MzY3IDAuMjM4OTk1IDIuMTIxMDUgMC41NDggMS43IDEuODU4NSAzLjAwNSAzLjU2NDk4IDMuNTUxIDAuMTIyIDAuMDYwNSAwLjI0NCAwIDAuMjc0IC0wLjEyMTUgMC4wMzA1IC0wLjAzIDAuMDMwNSAtMC4wNjEgMC4wMzA1IC0wLjEyMTV2LTAuNDI1YzAgLTAuMDkxIC0wLjA5MSAtMC4yMTIgLTAuMTgyNSAtMC4yNzNabTMuMjMgLTkuNDY4Yy0wLjEyMiAtMC4wNjEgLTAuMjQ0IDAgLTAuMjc0IDAuMTIxIC0wLjAzMDUgMC4wMzA1IC0wLjAzMDUgMC4wNjEgLTAuMDMwNSAwLjEyMTV2MC40MjVjMCAwLjEyMTUgMC4wOTEgMC4yNDI1IDAuMTgyNSAwLjMwMzUgMi4zNzcgMC44NSAzLjU5NiAzLjQ5IDIuNzEyIDUuODI2NSAtMC40NTcgMS4yNzUgLTEuNDYyNSAyLjI0NTUgLTIuNzEyIDIuNzAxIC0wLjEyMiAwLjA2MDUgLTAuMTgyNSAwLjE1MTUgLTAuMTgyNSAwLjMwMzV2MC40MjVjMCAwLjEyMSAwLjA2MDUgMC4yMTIgMC4xODI1IDAuMjQyNSAwLjAzMDUgMCAwLjA5MTUgMCAwLjEyMiAtMC4wMyAwLjY4NTggLTAuMjE0MTUgMS4zMjI0NSAtMC41NjE4NSAxLjg3MzMgLTEuMDIzMSAwLjU1MDg1IC0wLjQ2MTIgMS4wMDUwNSAtMS4wMjY4NSAxLjMzNjQgLTEuNjY0MyAwLjMzMTM1IC0wLjYzNzUgMC41MzMzNSAtMS4zMzQyIDAuNTk0MyAtMi4wNTAwNSAwLjA2MSAtMC43MTU4NSAtMC4wMjAyNSAtMS40MzY3IC0wLjIzOSAtMi4xMjEwNSAtMC41NDggLTEuNzMgLTEuODg5IC0zLjAzNSAtMy41NjUgLTMuNTgxWiIgc3Ryb2tlLXdpZHRoPSIwLjUiPgo8L3BhdGg+Cjwvc3ZnPgo=",
            reference: None,
            reference_hash: None,
            decimals: 6,
        }
    }

    #[payable]
    pub fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>) {
        let _ = memo;
        require!(env::attached_deposit().as_yoctonear() == 1, "require 1 yocto");
        let sender = env::predecessor_account_id();
        let sender_bal = *self.balances.get(&sender).unwrap_or(&0);
        require!(sender_bal >= amount.0, "insufficient balance");
        self.balances.insert(sender.clone(), sender_bal - amount.0);
        let recv_bal = *self.balances.get(&receiver_id).unwrap_or(&0);
        self.balances.insert(receiver_id, recv_bal + amount.0);
    }

    #[payable]
    pub fn ft_transfer_call(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>, msg: String) -> U128 {
        self.ft_transfer(receiver_id, amount.clone(), memo);
        // Simplified: does not actually call receiver's ft_on_transfer, but returns amount for demo
        let _ = msg;
        amount
    }

    #[payable]
    pub fn storage_deposit(&mut self, account_id: Option<AccountId>, registration_only: Option<bool>) -> StorageBalance {
        let _ = registration_only;
        let account = account_id.unwrap_or_else(|| env::predecessor_account_id());
        if self.balances.get(&account).is_none() {
            self.balances.insert(account.clone(), 0);
        }
        StorageBalance {
            total: U128(125 * env::storage_byte_cost().as_yoctonear()),
            available: U128(0),
        }
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        if self.balances.get(&account_id).is_some() {
            Some(StorageBalance {
                total: U128(125 * env::storage_byte_cost().as_yoctonear()),
                available: U128(0),
            })
        } else {
            None
        }
    }

    #[payable]
    pub fn mint(&mut self, account_id: AccountId, amount: U128) {
        // Faucet for demo - permissionless but limited? For demo open
        let entry = self.balances.get(&account_id).cloned().unwrap_or(0);
        self.balances.insert(account_id, entry + amount.0);
        self.total_supply += amount.0;
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
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let mut c = Contract {
            owner: accounts(0),
            total_supply: 0,
            balances: LookupMap::new(b"b"),
            allowances: LookupMap::new(b"a"),
        };
        c.mint(accounts(1), U128(1000));
        assert_eq!(c.ft_balance_of(accounts(1)).0, 1000);
        assert_eq!(c.ft_total_supply().0, 1000);
        assert_eq!(c.ft_decimals(), 6);

        // transfer
        let ctx2 = VMContextBuilder::new().predecessor_account_id(accounts(1)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx2);
        c.ft_transfer(accounts(2), U128(400), None);
        assert_eq!(c.ft_balance_of(accounts(1)).0, 600);
        assert_eq!(c.ft_balance_of(accounts(2)).0, 400);
    }

    #[test]
    fn metadata() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let c = Contract {
            owner: accounts(0),
            total_supply: 0,
            balances: LookupMap::new(b"b"),
            allowances: LookupMap::new(b"a"),
        };
        let meta = c.ft_metadata();
        assert_eq!(meta.decimals, 6);
        assert_eq!(meta.symbol, "USDC");
    }
}
