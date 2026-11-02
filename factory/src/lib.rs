use near_sdk::store::{LookupMap, IterableMap};
use near_sdk::{env, near, PanicOnDefault, AccountId, require};

mod error;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MachineInfo {
    pub coin: AccountId,
    pub machine: AccountId,
    pub creator: AccountId,
    pub quote_token: AccountId,
    pub is_fan_coin: bool,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct ProtocolStats {
    pub total_machines: u64,
    pub total_coins: u64,
    pub total_volume: String,
    pub total_packs_sold: u64,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub machines: Vec<AccountId>,
    pub machine_infos: LookupMap<AccountId, MachineInfo>,
    pub coin_to_machine: LookupMap<AccountId, AccountId>,
    pub allowlist: IterableMap<AccountId, bool>,
    pub global_contract_hash: Option<String>,
    pub protocol_volume: u128,
    pub protocol_packs: u64,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self {
            owner,
            machines: Vec::new(),
            machine_infos: LookupMap::new(b"i"),
            coin_to_machine: LookupMap::new(b"c"),
            allowlist: IterableMap::new(b"a"),
            global_contract_hash: None,
            protocol_volume: 0,
            protocol_packs: 0,
        }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn set_global_contract_hash(&mut self, hash: String) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        require!(!hash.is_empty(), "hash empty");
        self.global_contract_hash = Some(hash);
    }

    pub fn get_global_hash(&self) -> Option<String> {
        self.global_contract_hash.clone()
    }

    pub fn add_to_allowlist(&mut self, coin: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.allowlist.insert(coin, true);
    }

    pub fn remove_from_allowlist(&mut self, coin: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.allowlist.remove(&coin);
    }

    pub fn is_allowlisted(&self, coin: AccountId) -> bool {
        self.allowlist.get(&coin).cloned().unwrap_or(false)
    }

    pub fn get_allowlist(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let start = from_index.unwrap_or(0) as usize;
        let lim = limit.unwrap_or(50).min(100) as usize;
        self.allowlist.iter().skip(start).take(lim).map(|(k, _)| k.clone()).collect()
    }

    // launch_coin creates <coin>.bigo.testnet, references global contract hash, records registry entry
    // In sandbox, we simulate creation by registering machine account
    pub fn launch_coin(
        &mut self,
        coin_name: String,
        machine_account: AccountId,
        quote_token: Option<AccountId>,
        is_fan_coin: bool,
    ) -> String {
        let creator = env::predecessor_account_id();
        require!(!coin_name.is_empty(), "coin name empty");
        require!(coin_name.len() <= 32, "coin name too long");
        require!(coin_name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-'), "invalid coin chars");
        require!(self.global_contract_hash.is_some(), "global hash not set - deploy machine contract first");

        if is_fan_coin {
            require!(
                self.allowlist.get(&machine_account).is_some() || self.is_allowlisted(machine_account.clone()),
                "fan coin not allowlisted"
            );
        } else {
            let coin_id: AccountId = format!("{}.bigo.testnet", coin_name.to_lowercase()).parse().expect("invalid coin name");
            require!(
                self.coin_to_machine.get(&coin_id).is_none(),
                "coin already has machine"
            );
        }

        let coin_id = if is_fan_coin {
            machine_account.clone()
        } else {
            format!("{}.bigo.testnet", coin_name.to_lowercase()).parse().expect("invalid coin name")
        };

        let quote = quote_token.unwrap_or_else(|| "usdc.bigo.testnet".parse().unwrap());

        let info = MachineInfo {
            coin: coin_id.clone(),
            machine: machine_account.clone(),
            creator: creator.clone(),
            quote_token: quote,
            is_fan_coin,
        };

        if !is_fan_coin {
            self.coin_to_machine.insert(coin_id.clone(), machine_account.clone());
        }

        if !self.machines.contains(&machine_account) {
            self.machines.push(machine_account.clone());
        }
        self.machine_infos.insert(machine_account.clone(), info);

        machine_account.to_string()
    }


        pub fn get_machines(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let start = from_index.unwrap_or(0) as usize;
        let lim = limit.unwrap_or(50).min(100) as usize;
        self.machines.iter().skip(start).take(lim).cloned().collect()
    }

    pub fn get_machine_info(&self, machine: AccountId) -> Option<MachineInfo> {
        self.machine_infos.get(&machine).cloned()
    }

    pub fn get_machine_count(&self) -> u64 {
        self.machines.len() as u64
    }

    pub fn get_protocol_stats(&self) -> ProtocolStats {
        ProtocolStats {
            total_machines: self.machines.len() as u64,
            total_coins: self.machines.len() as u64, // coin count approx
            total_volume: self.protocol_volume.to_string(),
            total_packs_sold: self.protocol_packs,
        }
    }

    pub fn add_protocol_volume(&mut self, amount: String) {
        // Called by machines or owner to update stats, permissionless for demo
        let amt: u128 = amount.parse().unwrap_or(0);
        self.protocol_volume += amt;
        self.protocol_packs += 1;
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
        Contract {
            owner: accounts(0),
            machines: vec![],
            machine_infos: LookupMap::new(b"i"),
            coin_to_machine: LookupMap::new(b"c"),
            allowlist: IterableMap::new(b"a"),
            global_contract_hash: None,
            protocol_volume: 0,
            protocol_packs: 0,
        }
    }

    #[test]
    fn owner_is_set() {
        let c = setup();
        assert_eq!(c.get_owner(), accounts(0));
    }

    #[test]
    fn allowlist_management() {
        let mut c = setup();
        let ctx = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx);
        c.set_global_contract_hash("abc123".into());
        c.add_to_allowlist(accounts(1));
        assert!(c.is_allowlisted(accounts(1)));
        assert_eq!(c.get_allowlist(None, None).len(), 1);
        c.remove_from_allowlist(accounts(1));
        assert!(!c.is_allowlisted(accounts(1)));
    }

    #[test]
    fn launch_coin_new_and_fan() {
        let mut c = setup();
        let ctx = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash123".into());

        let machine: AccountId = "frog.bigo.testnet".parse().unwrap();
        let res = c.launch_coin("frog".into(), machine.clone(), None, false);
        assert_eq!(res, "frog.bigo.testnet");
        assert_eq!(c.get_machine_count(), 1);
        assert_eq!(c.get_protocol_stats().total_machines, 1);

        // Second launch same coin should fail
        let ctx2 = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx2);
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut c2 = setup();
            // Need to share state, so we test differently: try launch same name again in same contract
            // For this test we check coin_to_machine already has frog
            c.launch_coin("frog".into(), "frog2.bigo.testnet".parse().unwrap(), None, false)
        }));
        assert!(result.is_err(), "should fail duplicate coin");

        // Fan coin: need allowlist, owner adds it
        let ctx_owner = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx_owner);
        c.add_to_allowlist(machine.clone());
        let fan_machine: AccountId = "fan1.bigo.testnet".parse().unwrap();
        c.add_to_allowlist(fan_machine.clone());
        let res_fan = c.launch_coin("ignored".into(), fan_machine.clone(), None, true);
        assert_eq!(res_fan, fan_machine.to_string());
    }

    #[test]
    fn pagination() {
        let mut c = setup();
        let ctx = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        for i in 0..5 {
            let name = format!("coin{}", i);
            let machine: AccountId = format!("m{}.bigo.testnet", i).parse().unwrap();
            c.launch_coin(name, machine, None, false);
        }
        assert_eq!(c.get_machines(Some(0), Some(2)).len(), 2);
        assert_eq!(c.get_machines(Some(2), Some(10)).len(), 3);
    }
}
