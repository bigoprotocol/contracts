use near_sdk::base64::{engine::general_purpose::STANDARD, Engine};
use near_sdk::json_types::U128;
use near_sdk::store::{IterableMap, LookupMap};
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

pub const COIN_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;
pub const LAUNCH_DEPOSIT: u128 = 20_000_000_000_000_000_000_000_000;
pub const REGISTRY_FEE: u128 = 1_000_000_000_000_000_000_000_000;
const LAUNCH_GAS_RESERVE: u128 = 100_000_000_000_000_000_000_000;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq)]
pub struct PrizeTier {
    pub multiplier_bps: u32,
    pub odds_bps: u32,
    pub tier: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MachineConfig {
    pub prize_table: Vec<PrizeTier>,
    pub pack_art_url: String,
    pub vrf_pubkey: String,
    pub outlayer_account: AccountId,
    pub vrf_project_id: String,
    pub milestone_threshold: u64,
    pub milestone_new_top_mult_bps: u32,
    pub ft_gas: u64,
    pub curve_coin_allocation: u128,
    pub prize_coin_allocation: u128,
    pub max_pack_price: u128,
    pub scrap_discount_amount: u128,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MachineInfo {
    pub coin: AccountId,
    pub coin_token: AccountId,
    pub machine: AccountId,
    pub creator: AccountId,
    pub quote_token: AccountId,
    pub is_fan_coin: bool,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PendingLaunch {
    pub info: MachineInfo,
    pub coin_id: AccountId,
    pub machine_id: AccountId,
    pub attached: u128,
    pub machine_deposit: u128,
    pub coin_ready: bool,
    pub treasury: AccountId,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct AllowlistEntry {
    pub coin: AccountId,
    pub creator: AccountId,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct ProtocolStats {
    pub total_machines: u64,
    pub total_coins: u64,
    pub total_volume: String,
    pub total_packs_sold: u64,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct CoinMetadata {
    pub spec: String,
    pub name: String,
    pub symbol: String,
    pub icon: Option<String>,
    pub reference: Option<String>,
    pub reference_hash: Option<String>,
    pub decimals: u8,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub machines: LookupMap<u64, AccountId>,
    pub machine_count: u64,
    pub machine_infos: LookupMap<AccountId, MachineInfo>,
    pub coin_to_machine: LookupMap<AccountId, AccountId>,
    pub allowlist: IterableMap<(AccountId, AccountId), bool>,
    pub pending_launches: LookupMap<AccountId, PendingLaunch>,
    pub last_launch_error: String,
    pub total_refunded: u128,
    pub global_contract_hash: Option<String>,
    pub protocol_volume: u128,
    pub protocol_packs: u64,
    pub coin_wasm: Vec<u8>,
    pub machine_wasm: Vec<u8>,
    pub machine_config: MachineConfig,
}

impl Contract {
    fn decode_wasm(encoded: &str) -> Vec<u8> {
        STANDARD.decode(encoded).unwrap_or_default()
    }

    fn emit_event(kind: &str, data: near_sdk::serde_json::Value) {
        env::log_str(&format!(
            "EVENT_JSON:{}",
            near_sdk::serde_json::json!({
                "standard": "bigo",
                "version": "1.0.0",
                "event": kind,
                "data": data,
            })
        ));
    }
}

#[near]
impl Contract {
    // deploy and init must be submitted as one batched transaction. the factory
    // does this with create_account + deploy + new in a single promise chain.
    #[init]
    pub fn new(
        owner: AccountId,
        coin_wasm: String,
        machine_wasm: String,
        machine_config: MachineConfig,
    ) -> Self {
        assert!(!env::state_exists(), "already initialized");
        let coin_wasm = Self::decode_wasm(&coin_wasm);
        let machine_wasm = Self::decode_wasm(&machine_wasm);
        require!(!coin_wasm.is_empty(), "coin wasm required");
        require!(!machine_wasm.is_empty(), "machine wasm required");
        Self {
            owner,
            machines: LookupMap::new(b"m"),
            machine_count: 0,
            machine_infos: LookupMap::new(b"i"),
            coin_to_machine: LookupMap::new(b"c"),
            allowlist: IterableMap::new(b"a"),
            pending_launches: LookupMap::new(b"p"),
            last_launch_error: String::new(),
            total_refunded: 0,
            global_contract_hash: None,
            protocol_volume: 0,
            protocol_packs: 0,
            coin_wasm,
            machine_wasm,
            machine_config,
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

    pub fn set_coin_code(&mut self, wasm: String) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        let wasm = Self::decode_wasm(&wasm);
        require!(!wasm.is_empty(), "wasm empty");
        self.coin_wasm = wasm;
    }

    pub fn set_machine_code(&mut self, wasm: String) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        let wasm = Self::decode_wasm(&wasm);
        require!(!wasm.is_empty(), "wasm empty");
        self.machine_wasm = wasm;
    }

    pub fn set_machine_config(&mut self, config: MachineConfig) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.machine_config = config;
    }

    pub fn get_machine_config(&self) -> MachineConfig {
        self.machine_config.clone()
    }

    pub fn add_to_allowlist(&mut self, coin: AccountId, creator: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.allowlist.insert((coin, creator), true);
    }

    pub fn remove_from_allowlist(&mut self, coin: AccountId, creator: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.allowlist.remove(&(coin, creator));
    }

    pub fn is_allowlisted(&self, coin: AccountId, creator: AccountId) -> bool {
        self.allowlist
            .get(&(coin, creator))
            .cloned()
            .unwrap_or(false)
    }

    pub fn get_allowlist(
        &self,
        from_index: Option<u64>,
        limit: Option<u64>,
    ) -> Vec<AllowlistEntry> {
        let start = from_index.unwrap_or(0) as usize;
        let lim = limit.unwrap_or(50).min(100) as usize;
        self.allowlist
            .iter()
            .filter(|(_, allowed)| **allowed)
            .skip(start)
            .take(lim)
            .map(|((coin, creator), _)| AllowlistEntry {
                coin: coin.clone(),
                creator: creator.clone(),
            })
            .collect()
    }

    /// One call deploys the coin token and the machine.
    /// The attached deposit is a non refundable storage payment:
    /// REGISTRY_FEE (1 NEAR) stays with the factory for the registry records,
    /// the remainder (9.5 NEAR each) funds the coin and machine accounts so
    /// they can pay for their contract code and state.
    #[payable]
    pub fn launch_coin(
        &mut self,
        coin_name: String,
        quote_token: Option<AccountId>,
        is_fan_coin: bool,
        existing_coin: Option<AccountId>,
        treasury: Option<AccountId>,
    ) -> Promise {
        let attached = env::attached_deposit().as_yoctonear();
        require!(
            attached >= LAUNCH_DEPOSIT,
            format!("attach at least {} yocto to launch a coin", LAUNCH_DEPOSIT)
        );
        let creator = env::predecessor_account_id();
        require!(!coin_name.is_empty(), "coin name empty");
        require!(coin_name.len() <= 32, "coin name too long");
        require!(
            coin_name
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-'),
            "coin name must be lowercase alphanumeric, _ or -"
        );
        require!(
            !coin_name.ends_with("-machine"),
            "coin name must not end with -machine"
        );
        require!(
            self.global_contract_hash.is_some(),
            "global hash not set - deploy machine contract first"
        );
        if is_fan_coin {
            require!(
                existing_coin.is_some(),
                "fan coins require an existing coin"
            );
        } else {
            require!(existing_coin.is_none(), "existing_coin only for fan coins");
        }

        let factory = env::current_account_id();
        let machine_id: AccountId = format!("{}-machine.{}", coin_name, factory)
            .parse()
            .expect("invalid machine account");
        let coin_id: AccountId = match existing_coin {
            Some(coin) => coin,
            None => format!("{}.{}", coin_name, factory)
                .parse()
                .expect("invalid coin account"),
        };
        if is_fan_coin {
            require!(
                self.is_allowlisted(coin_id.clone(), creator.clone()),
                "fan coin not allowlisted for this creator"
            );
        }
        require!(
            self.coin_to_machine.get(&coin_id).is_none(),
            "coin already has machine"
        );
        require!(
            self.machine_infos.get(&machine_id).is_none(),
            "machine already registered"
        );
        require!(
            self.pending_launches.get(&machine_id).is_none(),
            "launch already pending"
        );

        let quote = quote_token.unwrap_or_else(|| "usdc.bigo.testnet".parse().unwrap());
        // protocol fees belong to the factory unless the caller names a treasury
        let treasury = treasury.unwrap_or_else(env::current_account_id);
        let info = MachineInfo {
            coin: coin_id.clone(),
            coin_token: coin_id.clone(),
            machine: machine_id.clone(),
            creator: creator.clone(),
            quote_token: quote.clone(),
            is_fan_coin,
        };
        let account_deposit = attached
            .checked_sub(REGISTRY_FEE)
            .expect("deposit below registry fee");
        let machine_deposit = if is_fan_coin {
            account_deposit
        } else {
            account_deposit / 2
        };
        require!(machine_deposit > 0, "deposit too small for accounts");
        self.pending_launches.insert(
            machine_id.clone(),
            PendingLaunch {
                info: info.clone(),
                coin_id: coin_id.clone(),
                machine_id: machine_id.clone(),
                attached,
                machine_deposit,
                coin_ready: false,
                treasury,
            },
        );

        if is_fan_coin {
            if let Some(mut pending) = self.pending_launches.get(&machine_id).cloned() {
                pending.coin_ready = true;
                self.pending_launches.insert(machine_id.clone(), pending);
            }
            return self.deploy_machine(machine_id);
        }
        let metadata = CoinMetadata {
            spec: "ft-1.0.0".to_string(),
            name: format!("{} Coin", coin_name),
            symbol: coin_name.to_uppercase(),
            icon: None,
            reference: None,
            reference_hash: None,
            decimals: 6,
        };
        let coin_init = near_sdk::serde_json::json!({
            "owner": machine_id,
            "total_supply": U128(COIN_TOTAL_SUPPLY),
            "metadata": metadata,
        });
        let coin_wasm = self.coin_wasm.clone();
        Promise::new(coin_id)
            .create_account()
            .transfer(NearToken::from_yoctonear(machine_deposit))
            .deploy_contract(coin_wasm)
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::to_vec(&coin_init).expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(40),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(30))
                    .on_coin_deployed(machine_id),
            )
    }

    #[private]
    pub fn on_coin_deployed(
        &mut self,
        machine_id: AccountId,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) -> Promise {
        let pending = match self.pending_launches.get(&machine_id).cloned() {
            Some(p) => p,
            None => return Promise::new(env::current_account_id()),
        };
        if result.is_err() {
            self.pending_launches.remove(&machine_id);
            self.last_launch_error = format!("coin deploy failed: {:?}", result);
            Self::emit_event(
                "launch_failed",
                near_sdk::serde_json::json!({
                    "machine": machine_id,
                    "coin": pending.coin_id,
                }),
            );
            let refund = pending.attached.saturating_sub(LAUNCH_GAS_RESERVE);
            self.total_refunded = self
                .total_refunded
                .checked_add(refund)
                .expect("refund overflow");
            return Promise::new(pending.info.creator).transfer(NearToken::from_yoctonear(refund));
        }
        if let Some(mut pending) = self.pending_launches.get(&machine_id).cloned() {
            pending.coin_ready = true;
            self.pending_launches.insert(machine_id.clone(), pending);
        }
        Self::emit_event(
            "coin_deployed",
            near_sdk::serde_json::json!({
                "machine": machine_id,
                "coin": pending.coin_id,
            }),
        );
        Promise::new(env::current_account_id())
    }

    /// Deploys the machine after its coin is confirmed. Anyone can pay the gas.
    pub fn deploy_machine(&mut self, machine_id: AccountId) -> Promise {
        let pending = match self.pending_launches.get(&machine_id).cloned() {
            Some(p) if p.coin_ready => p,
            Some(_) => env::panic_str("coin not ready"),
            None => env::panic_str("launch not pending"),
        };
        let cfg = &self.machine_config;
        let info = &pending.info;
        let machine_init = near_sdk::serde_json::json!({
            "owner": info.creator,
            "coin": info.coin,
            "coin_token": info.coin_token,
            "quote_token": info.quote_token,
            "prize_table": cfg.prize_table,
            "pack_art_url": cfg.pack_art_url,
            "vrf_pubkey": cfg.vrf_pubkey,
            "outlayer_account": cfg.outlayer_account,
            "vrf_project_id": cfg.vrf_project_id,
            "milestone_threshold": cfg.milestone_threshold,
            "milestone_new_top_mult_bps": cfg.milestone_new_top_mult_bps,
            "ft_gas": cfg.ft_gas,
            "curve_coin_allocation": U128(cfg.curve_coin_allocation),
            "prize_coin_allocation": U128(cfg.prize_coin_allocation),
            "max_pack_price": U128(cfg.max_pack_price),
            "scrap_discount_amount": U128(cfg.scrap_discount_amount),
            "treasury": pending.treasury,
            "fan_coin": info.is_fan_coin,
            "initial_coin_balance": U128(if info.is_fan_coin { 0 } else { COIN_TOTAL_SUPPLY }),
            "graduation_target": null,
            "rhea": null,
        });
        let machine_wasm = self.machine_wasm.clone();
        Promise::new(machine_id.clone())
            .create_account()
            .transfer(NearToken::from_yoctonear(pending.machine_deposit))
            .deploy_contract(machine_wasm)
            .function_call(
                "new".to_string(),
                near_sdk::serde_json::to_vec(&machine_init).expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(40),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(60))
                    .on_deploy_complete(machine_id),
            )
    }

    #[private]
    pub fn on_deploy_complete(
        &mut self,
        machine_id: AccountId,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        let pending = match self.pending_launches.get(&machine_id).cloned() {
            Some(p) => p,
            None => return,
        };
        self.pending_launches.remove(&machine_id);
        if result.is_err() {
            self.last_launch_error = format!("machine deploy failed: {:?}", result);
            Self::emit_event(
                "launch_failed",
                near_sdk::serde_json::json!({
                    "machine": machine_id,
                    "coin": pending.coin_id,
                }),
            );
            return;
        }
        if self.coin_to_machine.get(&pending.coin_id).is_some()
            || self.machine_infos.get(&machine_id).is_some()
        {
            Self::emit_event(
                "launch_failed",
                near_sdk::serde_json::json!({
                    "machine": machine_id,
                    "reason": "name taken",
                }),
            );
            return;
        }
        self.coin_to_machine
            .insert(pending.coin_id.clone(), machine_id.clone());
        self.machine_infos
            .insert(machine_id.clone(), pending.info.clone());
        self.machines.insert(self.machine_count, machine_id.clone());
        self.machine_count = self
            .machine_count
            .checked_add(1)
            .expect("machine count overflow");
        Self::emit_event(
            "machine_launched",
            near_sdk::serde_json::json!({
                "machine": machine_id,
                "coin": pending.coin_id,
                "creator": pending.info.creator,
                "is_fan_coin": pending.info.is_fan_coin,
            }),
        );
    }

    pub fn get_machines(&self, from_index: Option<u64>, limit: Option<u64>) -> Vec<AccountId> {
        let start = from_index.unwrap_or(0);
        let lim = limit.unwrap_or(50).min(100);
        let mut out = Vec::new();
        let mut i = start;
        while i < self.machine_count && out.len() < lim as usize {
            if let Some(machine) = self.machines.get(&i) {
                out.push(machine.clone());
            }
            i += 1;
        }
        out
    }

    pub fn get_machine_info(&self, machine: AccountId) -> Option<MachineInfo> {
        self.machine_infos.get(&machine).cloned()
    }

    pub fn get_last_launch_error(&self) -> String {
        self.last_launch_error.clone()
    }

    pub fn get_total_refunded(&self) -> U128 {
        U128(self.total_refunded)
    }

    pub fn get_machine_count(&self) -> u64 {
        self.machine_count
    }

    pub fn get_protocol_stats(&self) -> ProtocolStats {
        ProtocolStats {
            total_machines: self.machine_count,
            total_coins: self.machine_count,
            total_volume: self.protocol_volume.to_string(),
            total_packs_sold: self.protocol_packs,
        }
    }

    pub fn add_protocol_volume(&mut self, amount: String) {
        let caller = env::predecessor_account_id();
        require!(
            caller == self.owner || self.machine_infos.get(&caller).is_some(),
            "only owner or a registered machine"
        );
        let amt: u128 = amount.parse().expect("invalid amount");
        self.protocol_volume = self
            .protocol_volume
            .checked_add(amt)
            .expect("volume overflow");
        self.protocol_packs = self.protocol_packs.checked_add(1).expect("packs overflow");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn config() -> MachineConfig {
        MachineConfig {
            prize_table: vec![PrizeTier {
                multiplier_bps: 800,
                odds_bps: 10000,
                tier: "Mythic".into(),
            }],
            pack_art_url: "ipfs://QmTest".into(),
            vrf_pubkey: "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8".into(),
            outlayer_account: "outlayer.testnet".parse().unwrap(),
            vrf_project_id: "bigodeployer.testnet/bigo-vrf".into(),
            milestone_threshold: 50,
            milestone_new_top_mult_bps: 1000,
            ft_gas: 30,
            curve_coin_allocation: 800_000_000_000_000,
            prize_coin_allocation: 200_000_000_000_000,
            max_pack_price: 10,
            scrap_discount_amount: 1,
        }
    }

    fn setup() -> Contract {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(
            accounts(0),
            STANDARD.encode([1u8]),
            STANDARD.encode([2u8]),
            config(),
        )
    }

    fn launch(c: &mut Contract, name: &str, fan: bool) -> Promise {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(LAUNCH_DEPOSIT))
            .build();
        testing_env!(ctx);
        c.launch_coin(name.into(), None, fan, None, None)
    }

    #[test]
    fn owner_is_set() {
        let c = setup();
        assert_eq!(c.get_owner(), accounts(0));
    }

    #[test]
    fn allowlist_management() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.add_to_allowlist(accounts(1), accounts(0));
        assert!(c.is_allowlisted(accounts(1), accounts(0)));
        assert!(!c.is_allowlisted(accounts(1), accounts(2)));
        c.remove_from_allowlist(accounts(1), accounts(0));
        assert!(!c.is_allowlisted(accounts(1), accounts(0)));
    }

    #[test]
    fn launch_deploys_and_registers() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        let _ = launch(&mut c, "duck", false);
        let machine_id: AccountId = format!("duck-machine.{}", env::current_account_id())
            .parse()
            .unwrap();
        c.on_deploy_complete(machine_id, Ok(()));
        assert_eq!(c.get_machine_count(), 1);
        let machines = c.get_machines(None, None);
        assert_eq!(machines.len(), 1);
        let info = c.get_machine_info(machines[0].clone()).unwrap();
        assert_eq!(info.coin_token, info.coin);
        assert_eq!(
            info.coin.to_string(),
            format!("duck.{}", env::current_account_id())
        );
    }

    #[test]
    #[should_panic(expected = "coin already has machine")]
    fn duplicate_coin_guard() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        let _ = launch(&mut c, "duck", false);
        let machine_id: AccountId = format!("duck-machine.{}", env::current_account_id())
            .parse()
            .unwrap();
        c.on_deploy_complete(machine_id, Ok(()));
        let _ = launch(&mut c, "duck", false);
    }

    #[test]
    #[should_panic(expected = "attach at least")]
    fn launch_requires_deposit() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        let _ = c.launch_coin("duck".into(), None, false, None, None);
    }

    #[test]
    #[should_panic(expected = "lowercase")]
    fn launch_rejects_bad_name() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(LAUNCH_DEPOSIT))
            .build();
        testing_env!(ctx);
        let _ = c.launch_coin("Duck!".into(), None, false, None, None);
    }

    #[test]
    #[should_panic(expected = "must not end with -machine")]
    fn launch_rejects_machine_suffix() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.set_global_contract_hash("hash".into());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(LAUNCH_DEPOSIT))
            .build();
        testing_env!(ctx);
        let _ = c.launch_coin("duck-machine".into(), None, false, None, None);
    }
}
