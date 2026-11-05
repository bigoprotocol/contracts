use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct PoolInfo {
    pub pool_id: u64,
    pub token_account_ids: Vec<AccountId>,
    pub amounts: Vec<U128>,
    pub total_fee: u32,
    pub shares_total_supply: U128,
    pub amp: u64,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub pools: Vec<PoolInfo>,
    pub deposits: LookupMap<(AccountId, AccountId), u128>,
    pub storage: LookupMap<AccountId, u128>,
    pub fail_add_liquidity: bool,
    pub fail_deposit: bool,
    pub fail_pools: bool,
    pub swap_rate: u128,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            owner,
            pools: Vec::new(),
            deposits: LookupMap::new(b"d"),
            storage: LookupMap::new(b"s"),
            fail_add_liquidity: false,
            fail_deposit: false,
            fail_pools: false,
            swap_rate: 1_000_000,
        }
    }

    #[payable]
    pub fn storage_deposit(
        &mut self,
        account_id: Option<AccountId>,
        registration_only: Option<bool>,
    ) -> near_sdk::serde_json::Value {
        let _ = registration_only;
        let account = account_id.unwrap_or_else(|| env::predecessor_account_id());
        let cur = self.storage.get(&account).cloned().unwrap_or(0);
        let total = cur + env::attached_deposit().as_yoctonear();
        self.storage.insert(account, total);
        near_sdk::serde_json::json!({"total": U128(total), "available": U128(0)})
    }

    pub fn get_pools(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<PoolInfo> {
        require!(!self.fail_pools, "pools unavailable");
        let start = from_index.unwrap_or(0) as usize;
        let lim = limit.unwrap_or(50) as usize;
        self.pools.iter().skip(start).take(lim).cloned().collect()
    }

    pub fn get_pool(&self, pool_id: u64) -> PoolInfo {
        self.pools[pool_id as usize].clone()
    }

    #[payable]
    pub fn add_simple_pool(&mut self, tokens: Vec<AccountId>, fee: u32) -> u64 {
        require!(tokens.len() == 2, "two tokens");
        let id = self.pools.len() as u64;
        self.pools.push(PoolInfo {
            pool_id: id,
            token_account_ids: tokens,
            amounts: vec![U128(0), U128(0)],
            total_fee: fee,
            shares_total_supply: U128(0),
            amp: 0,
        });
        id
    }

    #[payable]
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        require!(!self.fail_deposit, "deposit disabled");
        let token = env::predecessor_account_id();
        if msg.is_empty() {
            let key = (sender_id, token);
            let cur = self.deposits.get(&key).cloned().unwrap_or(0);
            self.deposits.insert(key, cur + amount.0);
            return U128(0);
        }
        let parsed: near_sdk::serde_json::Value =
            near_sdk::serde_json::from_str(&msg).expect("invalid msg");
        let actions = parsed
            .get("actions")
            .and_then(|v| v.as_array())
            .expect("actions required");
        let action = &actions[0];
        let token_out: AccountId = action
            .get("token_out")
            .and_then(|v| v.as_str())
            .expect("token_out")
            .parse()
            .unwrap();
        let out = amount.0 * self.swap_rate / 1_000_000;
        let key = (sender_id, token_out);
        let cur = self.deposits.get(&key).cloned().unwrap_or(0);
        self.deposits.insert(key, cur + out);
        U128(0)
    }

    #[payable]
    pub fn add_liquidity(&mut self, pool_id: u64, amounts: Vec<U128>) {
        require!(!self.fail_add_liquidity, "add_liquidity disabled");
        require!(amounts.len() == 2, "two amounts");
        let mut pool = self.pools[pool_id as usize].clone();
        let caller = env::predecessor_account_id();
        for (i, token) in pool.token_account_ids.iter().enumerate() {
            let key = (caller.clone(), token.clone());
            let cur = self.deposits.get(&key).cloned().unwrap_or(0);
            require!(cur >= amounts[i].0, "insufficient deposit");
            self.deposits.insert(key, cur - amounts[i].0);
            pool.amounts[i] = U128(pool.amounts[i].0 + amounts[i].0);
        }
        self.pools[pool_id as usize] = pool;
    }

    pub fn set_pool_amounts(&mut self, pool_id: u64, amounts: Vec<U128>) {
        require!(amounts.len() == 2, "two amounts");
        let mut pool = self.pools[pool_id as usize].clone();
        pool.amounts = amounts;
        self.pools[pool_id as usize] = pool;
    }

    pub fn get_deposit(&self, account_id: AccountId, token_id: AccountId) -> U128 {
        U128(self.deposits.get(&(account_id, token_id)).cloned().unwrap_or(0))
    }

    #[payable]
    pub fn withdraw(&mut self, token_id: AccountId, amount: U128) -> Promise {
        let caller = env::predecessor_account_id();
        let key = (caller.clone(), token_id.clone());
        let cur = self.deposits.get(&key).cloned().unwrap_or(0);
        require!(cur >= amount.0, "insufficient deposit");
        self.deposits.insert(key, cur - amount.0);
        Promise::new(token_id).function_call(
            "ft_transfer".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "receiver_id": caller,
                "amount": amount.0.to_string(),
            }))
            .expect("json"),
            NearToken::from_yoctonear(1),
            Gas::from_tgas(30),
        )
    }

    pub fn set_fail_add_liquidity(&mut self, fail: bool) {
        self.fail_add_liquidity = fail;
    }

    pub fn set_fail_deposit(&mut self, fail: bool) {
        self.fail_deposit = fail;
    }

    pub fn set_fail_pools(&mut self, fail: bool) {
        self.fail_pools = fail;
    }

    pub fn set_swap_rate(&mut self, rate: u128) {
        self.swap_rate = rate;
    }
}
