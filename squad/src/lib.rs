use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquadStatus {
    Resolving,
    Invalid,
    Created,
    Funded,
    Bought,
    BuyingFailed,
    Opening,
    Opened,
    Refunded,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct Squad {
    pub id: u64,
    pub creator: AccountId,
    pub machine: AccountId,
    pub quote_token: AccountId,
    pub coin_token: AccountId,
    pub pack_price: u128,
    pub deposited: u128,
    pub members: Vec<AccountId>,
    pub deadline_block: u64,
    pub status: SquadStatus,
    pub prize_total: u128,
    pub draw_id: Option<u64>,
    pub synced_draw_id: Option<u64>,
    pub distributed: bool,
    pub refund_credited: bool,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct SquadView {
    pub id: u64,
    pub creator: AccountId,
    pub machine: AccountId,
    pub quote_token: AccountId,
    pub coin_token: AccountId,
    pub pack_price: U128,
    pub deposited: U128,
    pub members: Vec<AccountId>,
    pub deadline_block: u64,
    pub status: SquadStatus,
    pub prize_total: U128,
    pub draw_id: Option<u64>,
    pub synced_draw_id: Option<u64>,
    pub distributed: bool,
    pub refund_credited: bool,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct FactoryMachineInfo {
    pub coin: AccountId,
    pub coin_token: AccountId,
    pub machine: AccountId,
    pub creator: AccountId,
    pub quote_token: AccountId,
    pub is_fan_coin: bool,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MachineDraw {
    pub draw_id: u64,
    pub owner: AccountId,
    pub status: String,
    pub prize: Option<MachinePrize>,
    pub commit_block: u64,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct MachinePrize {
    pub tier: String,
    pub multiplier_bps: u32,
    pub prize_amount: U128,
}

const VRF_DEPOSIT: u128 = 1_000_000_000_000_000_000_000;
const MACHINE_OPEN_GAS: u64 = 200;
const MACHINE_VIEW_GAS: u64 = 20;
const MACHINE_CLAIM_GAS: u64 = 150;
const FACTORY_VIEW_GAS: u64 = 20;
const FT_GAS: u64 = 30;
const CALLBACK_GAS: u64 = 20;
const MIN_SQUAD_DEPOSIT: u128 = 1_000;
const MIN_PACK_PRICE: u128 = 1;
const MAX_MEMBERS: usize = 50;
const MAX_DEADLINE_BLOCKS: u64 = 100_000;
const STORAGE_DEPOSIT: u128 = 10_000_000_000_000_000_000_000;
const MIN_CREATE_DEPOSIT: u128 = 50_000_000_000_000_000_000_000;
const MIN_COIN_REGISTRATION: u128 = 10_000_000_000_000_000_000_000;
// factory view 20 + callback 20 + ft_transfer_call 150 + callback 20
// + get_draws_by_owner view 20 + callback 20 = 250, plus execution headroom
const SQUAD_BUY_MIN_GAS: u64 = 300;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub factory_account: AccountId,
    pub squads: LookupMap<u64, Squad>,
    pub member_deposits: LookupMap<(u64, AccountId), u128>,
    pub pending_refunds: LookupMap<(u64, AccountId), u128>,
    pub member_claimable: LookupMap<(u64, AccountId), u128>,
    pub last_claim_error: String,
    pub next_id: u64,
}

#[near]
impl Contract {
    // deploy and init must be submitted as one batched transaction. the factory
    // does this with create_account + deploy + new in a single promise chain.
    #[init]
    pub fn new(owner: AccountId, factory_account: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self {
            owner,
            factory_account,
            squads: LookupMap::new(b"s"),
            member_deposits: LookupMap::new(b"m"),
            pending_refunds: LookupMap::new(b"r"),
            member_claimable: LookupMap::new(b"c"),
            last_claim_error: String::new(),
            next_id: 1,
        }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn get_factory_account(&self) -> AccountId {
        self.factory_account.clone()
    }

    pub fn set_factory_account(&mut self, factory_account: AccountId) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        self.factory_account = factory_account;
    }

    /// Creates the squad and resolves machine, quote token and coin token from the factory.
    #[payable]
    pub fn create_squad(
        &mut self,
        machine: AccountId,
        pack_price: String,
        deadline_blocks: Option<u64>,
    ) -> u64 {
        require!(
            env::attached_deposit().as_yoctonear() >= MIN_CREATE_DEPOSIT,
            format!(
                "attach at least {} yocto to create a squad",
                MIN_CREATE_DEPOSIT
            )
        );
        let creator = env::predecessor_account_id();
        let price: u128 = pack_price.parse().expect("invalid pack_price");
        require!(price >= MIN_PACK_PRICE, "pack_price too low");
        let blocks = deadline_blocks.unwrap_or(100);
        require!(
            blocks <= MAX_DEADLINE_BLOCKS,
            format!("deadline above {}", MAX_DEADLINE_BLOCKS)
        );
        require!(blocks > 0, "deadline must be > 0");
        let id = self.next_id;
        self.next_id = self.next_id.checked_add(1).expect("id overflow");
        let squad = Squad {
            id,
            creator,
            machine: machine.clone(),
            quote_token: "usdc.bigo.testnet".parse().unwrap(),
            coin_token: "coin.bigo.testnet".parse().unwrap(),
            pack_price: price,
            deposited: 0,
            members: Vec::new(),
            deadline_block: env::block_height() + blocks,
            status: SquadStatus::Resolving,
            prize_total: 0,
            draw_id: None,
            synced_draw_id: None,
            distributed: false,
            refund_credited: false,
        };
        self.squads.insert(id, squad);
        Promise::new(self.factory_account.clone())
            .function_call(
                "get_machine_info".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({"machine": machine}))
                    .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(FACTORY_VIEW_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_machine_info(id),
            )
            .detach();
        id
    }

    #[private]
    pub fn on_machine_info(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<Option<FactoryMachineInfo>, near_sdk::PromiseError>,
    ) -> bool {
        let mut squad = match self.squads.get(&squad_id).cloned() {
            Some(s) => s,
            None => return false,
        };
        let info = match result {
            Ok(Some(info)) if info.machine == squad.machine => info,
            _ => {
                squad.status = SquadStatus::Invalid;
                self.squads.insert(squad_id, squad);
                return false;
            }
        };
        squad.quote_token = info.quote_token;
        squad.coin_token = info.coin_token;
        squad.status = SquadStatus::Created;
        self.squads.insert(squad_id, squad);
        true
    }

    /// The quote token is the only deposit path. Message: {"action":"join_squad","squad_id":N}
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        let token = env::predecessor_account_id();
        let parsed: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&msg)
            .unwrap_or_else(|_| env::panic_str("invalid msg json"));
        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| env::panic_str("msg.action required"));
        require!(action == "join_squad", format!("unknown action {}", action));
        let squad_id = parsed
            .get("squad_id")
            .and_then(|v| v.as_u64())
            .unwrap_or_else(|| env::panic_str("msg.squad_id required"));
        self.join_squad_internal(sender_id, squad_id, amount.0, token);
        U128(0)
    }

    fn join_squad_internal(
        &mut self,
        joiner: AccountId,
        squad_id: u64,
        amount: u128,
        token: AccountId,
    ) {
        require!(amount >= MIN_SQUAD_DEPOSIT, "deposit too small");
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status != SquadStatus::Resolving, "squad not resolved");
        require!(
            squad.status == SquadStatus::Created || squad.status == SquadStatus::Funded,
            "squad not open for joins"
        );
        require!(
            squad.quote_token == token,
            format!("wrong quote token {} vs {}", token, squad.quote_token)
        );
        require!(
            env::block_height() <= squad.deadline_block,
            "deadline passed"
        );
        if !squad.members.contains(&joiner) {
            require!(
                squad.members.len() < MAX_MEMBERS,
                format!("member cap {}", MAX_MEMBERS)
            );
            squad.members.push(joiner.clone());
        }
        let key = (squad_id, joiner);
        let existing = self.member_deposits.get(&key).cloned().unwrap_or(0);
        self.member_deposits
            .insert(key, existing.checked_add(amount).expect("deposit overflow"));
        squad.deposited = squad
            .deposited
            .checked_add(amount)
            .expect("deposited overflow");
        if squad.deposited >= squad.pack_price {
            squad.status = SquadStatus::Funded;
        }
        self.squads.insert(squad_id, squad);
    }

    /// Verifies factory registration before any token movement, then pays the machine.
    pub fn buy_squad_pack(&mut self, squad_id: u64) -> Promise {
        require!(
            env::prepaid_gas() >= Gas::from_tgas(SQUAD_BUY_MIN_GAS),
            format!(
                "attach at least {} Tgas to buy_squad_pack",
                SQUAD_BUY_MIN_GAS
            )
        );
        let squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status == SquadStatus::Funded, "not funded enough");
        require!(
            env::block_height() <= squad.deadline_block,
            "deadline passed - refund instead"
        );
        Promise::new(self.factory_account.clone())
            .function_call(
                "get_machine_info".to_string(),
                near_sdk::serde_json::to_vec(
                    &near_sdk::serde_json::json!({"machine": squad.machine}),
                )
                .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(FACTORY_VIEW_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_factory_verified_buy(squad_id),
            )
    }

    #[private]
    pub fn on_factory_verified_buy(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<Option<FactoryMachineInfo>, near_sdk::PromiseError>,
    ) -> Promise {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        let info = match result {
            Ok(Some(info)) => info,
            _ => env::panic_str("factory registration missing"),
        };
        require!(info.machine == squad.machine, "machine mismatch");
        require!(
            info.quote_token == squad.quote_token,
            "quote token mismatch"
        );
        require!(info.coin_token == squad.coin_token, "coin token mismatch");
        require!(squad.status == SquadStatus::Funded, "not funded");

        // return any deposit above the pack price pro rata and shrink the ledger
        if squad.deposited > squad.pack_price {
            let excess = squad.deposited - squad.pack_price;
            let total = squad.deposited;
            let mut credited = 0u128;
            for member in squad.members.iter() {
                let key = (squad_id, member.clone());
                let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
                if deposit > 0 {
                    let share = deposit.checked_mul(excess).expect("excess overflow") / total;
                    if share > 0 {
                        let cur = self.pending_refunds.get(&key).cloned().unwrap_or(0);
                        self.pending_refunds.insert(
                            key.clone(),
                            cur.checked_add(share).expect("refund overflow"),
                        );
                        self.member_deposits.insert(key, deposit - share);
                        credited = credited.checked_add(share).expect("excess overflow");
                    }
                }
            }
            // keep deposited equal to the sum of the remaining member deposits
            squad.deposited = squad
                .deposited
                .checked_sub(credited)
                .expect("excess underflow");
        }
        squad.status = SquadStatus::Bought;
        self.squads.insert(squad_id, squad.clone());
        Promise::new(squad.quote_token)
            .function_call(
                "ft_transfer_call".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "receiver_id": squad.machine,
                    "amount": U128(squad.pack_price),
                    "msg": "{\"action\":\"buy_pack\"}",
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(FT_GAS + 120),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_squad_buy_result(squad_id),
            )
    }

    #[private]
    pub fn on_squad_buy_result(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        // ft_transfer_call resolves to the amount used, not the amount unused
        let used = match result {
            Ok(used) => used.0.min(squad.pack_price),
            Err(_) => 0,
        };
        if used == 0 {
            // full failure: every remaining deposit is credited exactly once and zeroed
            for member in squad.members.iter() {
                let key = (squad_id, member.clone());
                let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
                if deposit > 0 {
                    let cur = self.pending_refunds.get(&key).cloned().unwrap_or(0);
                    self.pending_refunds.insert(
                        key.clone(),
                        cur.checked_add(deposit).expect("refund overflow"),
                    );
                    self.member_deposits.insert(key, 0);
                }
            }
            squad.refund_credited = true;
            squad.status = SquadStatus::Refunded;
            self.squads.insert(squad_id, squad);
            Self::emit_event(
                "squad_buy_refunded",
                near_sdk::serde_json::json!({"squad_id": squad_id}),
            );
            return true;
        }
        if used < squad.pack_price {
            // partial use: return the unused part pro rata, keep the deposits
            let unused = squad.pack_price - used;
            for member in squad.members.iter() {
                let key = (squad_id, member.clone());
                let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
                if deposit > 0 {
                    let share =
                        deposit.checked_mul(unused).expect("refund overflow") / squad.deposited;
                    if share > 0 {
                        let cur = self.pending_refunds.get(&key).cloned().unwrap_or(0);
                        self.pending_refunds
                            .insert(key, cur.checked_add(share).expect("refund overflow"));
                    }
                }
            }
        }
        squad.status = SquadStatus::Bought;
        self.squads.insert(squad_id, squad.clone());
        // the machine keeps a per-owner last-draw index, one write per mint
        Promise::new(squad.machine.clone())
            .function_call(
                "get_my_last_draw".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account": env::current_account_id(),
                }))
                .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(MACHINE_VIEW_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_last_draw(squad_id),
            )
            .detach();
        true
    }

    #[private]
    pub fn on_last_draw(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<Option<u64>, near_sdk::PromiseError>,
    ) -> bool {
        let mut squad = match self.squads.get(&squad_id).cloned() {
            Some(s) => s,
            None => return false,
        };
        if squad.status != SquadStatus::Bought {
            return false;
        }
        match result.ok().flatten() {
            Some(draw_id) => {
                squad.draw_id = Some(draw_id);
                self.squads.insert(squad_id, squad);
                true
            }
            None => {
                squad.status = SquadStatus::BuyingFailed;
                self.squads.insert(squad_id, squad);
                Self::emit_event(
                    "squad_buying_failed",
                    near_sdk::serde_json::json!({"squad_id": squad_id}),
                );
                false
            }
        }
    }

    /// Creator recovery from a buy whose draw could not be identified.
    pub fn recover_to_funded(&mut self, squad_id: u64) -> bool {
        let caller = env::predecessor_account_id();
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(caller == squad.creator, "only creator");
        require!(squad.status == SquadStatus::BuyingFailed, "not failed");
        squad.status = SquadStatus::Funded;
        squad.draw_id = None;
        self.squads.insert(squad_id, squad);
        true
    }

    /// Owner recovery for a squad stuck in Bought without a stored draw id.
    pub fn recover_stuck_squad(&mut self, squad_id: u64) -> bool {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status == SquadStatus::Bought, "not bought");
        require!(squad.draw_id.is_none(), "draw already stored");
        squad.status = SquadStatus::Funded;
        self.squads.insert(squad_id, squad);
        true
    }

    /// Only a member or the creator can open. A failed machine call reverts to Bought.
    #[payable]
    pub fn open_squad_pack(&mut self, squad_id: u64) -> Promise {
        require!(
            env::attached_deposit().as_yoctonear() >= VRF_DEPOSIT,
            format!("attach at least {} yocto for the VRF request", VRF_DEPOSIT)
        );
        let caller = env::predecessor_account_id();
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(
            caller == squad.creator || squad.members.contains(&caller),
            "only a member or the creator can open"
        );
        require!(squad.status == SquadStatus::Bought, "not bought yet");
        let draw_id = squad.draw_id.expect("draw id missing");
        squad.status = SquadStatus::Opening;
        self.squads.insert(squad_id, squad.clone());

        Promise::new(squad.machine)
            .function_call(
                "open_pack".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({"token_id": draw_id}))
                    .expect("json"),
                NearToken::from_yoctonear(VRF_DEPOSIT),
                Gas::from_tgas(MACHINE_OPEN_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_open_result(squad_id),
            )
    }

    #[private]
    pub fn on_open_result(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<bool, near_sdk::PromiseError>,
    ) -> bool {
        if result.is_err() {
            if let Some(mut squad) = self.squads.get(&squad_id).cloned() {
                if squad.status == SquadStatus::Opening {
                    squad.status = SquadStatus::Bought;
                    self.squads.insert(squad_id, squad);
                }
            }
            return false;
        }
        true
    }

    /// Permissionless. Reads the settled draw and pulls real coins from the machine.
    pub fn sync_squad_prize(&mut self, squad_id: u64) -> Promise {
        let squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status == SquadStatus::Opening, "squad is not opening");
        let token_id = squad.draw_id.expect("draw id missing");
        Promise::new(squad.machine)
            .function_call(
                "get_draw".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({"draw_id": token_id}))
                    .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(MACHINE_VIEW_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_draw_synced(squad_id),
            )
    }

    #[private]
    pub fn on_draw_synced(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<Option<MachineDraw>, near_sdk::PromiseError>,
    ) -> Promise {
        let draw = match result {
            Ok(Some(d)) => d,
            _ => env::panic_str("draw not found"),
        };
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(
            draw.owner == env::current_account_id(),
            "draw owner is not the squad"
        );
        require!(Some(draw.draw_id) == squad.draw_id, "wrong pack id");
        if squad.synced_draw_id == Some(draw.draw_id) {
            env::panic_str("draw already synced");
        }
        let settled = draw.status == "Settled" || draw.status == "FallbackSettled";
        if !settled {
            env::panic_str("draw not settled");
        }
        let prize = draw.prize.map(|p| p.prize_amount.0).unwrap_or(0);
        let coin_token = squad.coin_token.clone();
        squad.synced_draw_id = Some(draw.draw_id);
        squad.prize_total = prize;
        self.squads.insert(squad_id, squad);

        Promise::new(coin_token)
            .function_call(
                "storage_deposit".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account_id": env::current_account_id(),
                    "registration_only": true,
                }))
                .expect("json"),
                NearToken::from_millinear(10),
                Gas::from_tgas(30),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_coin_registered_for_claim(squad_id, draw.draw_id),
            )
    }

    #[private]
    pub fn on_coin_registered_for_claim(
        &mut self,
        squad_id: u64,
        draw_id: u64,
        #[callback_result] result: Result<near_sdk::serde_json::Value, near_sdk::PromiseError>,
    ) -> Promise {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        if result.is_err() {
            squad.synced_draw_id = None;
            self.squads.insert(squad_id, squad);
            return Promise::new(env::current_account_id());
        }
        Promise::new(squad.machine)
            .function_call(
                "claim_draw".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({"draw_id": draw_id}))
                    .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(MACHINE_CLAIM_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_machine_claim_result(squad_id),
            )
    }

    #[private]
    pub fn on_machine_claim_result(
        &mut self,
        squad_id: u64,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        if result.is_err() {
            self.last_claim_error = format!("{:?}", result);
            squad.synced_draw_id = None;
            self.squads.insert(squad_id, squad);
            return false;
        }
        if squad.distributed || squad.deposited == 0 {
            return false;
        }
        let prize = squad.prize_total;
        let deposited = squad.deposited;
        let coin_token = squad.coin_token.clone();
        squad.distributed = true;
        squad.status = SquadStatus::Opened;
        self.squads.insert(squad_id, squad.clone());

        for member in squad.members.iter() {
            let key = (squad_id, member.clone());
            let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
            if deposit == 0 {
                continue;
            }
            let share = deposit.checked_mul(prize).expect("share overflow") / deposited;
            if share > 0 {
                let cur = self.member_claimable.get(&key).cloned().unwrap_or(0);
                self.member_claimable
                    .insert(key, cur.checked_add(share).expect("share overflow"));
            }
        }
        Self::emit_event(
            "squad_distributed",
            near_sdk::serde_json::json!({
                "squad_id": squad_id,
                "prize": prize.to_string(),
                "coin_token": coin_token,
            }),
        );
        true
    }

    #[payable]
    pub fn claim_squad_share(&mut self, squad_id: u64) -> Promise {
        let required = FT_GAS + CALLBACK_GAS + 20;
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let caller = env::predecessor_account_id();
        let key = (squad_id, caller.clone());
        let amount = self.member_claimable.get(&key).cloned().unwrap_or(0);
        require!(amount > 0, "nothing to claim");
        self.member_claimable.insert(key, 0);
        let squad = self.squads.get(&squad_id).expect("squad not found").clone();
        Promise::new(squad.coin_token)
            .function_call(
                "ft_transfer".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "receiver_id": caller,
                    "amount": U128(amount),
                    "memo": format!("squad {} prize", squad_id),
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(FT_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_share_claim_result(squad_id, caller, U128(amount)),
            )
    }

    #[private]
    pub fn on_share_claim_result(
        &mut self,
        squad_id: u64,
        account: AccountId,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            let key = (squad_id, account);
            let cur = self.member_claimable.get(&key).cloned().unwrap_or(0);
            self.member_claimable
                .insert(key, cur.checked_add(amount.0).expect("restore overflow"));
        }
    }

    /// Registers the squad with the machine so the machine accepts its transfer.
    #[payable]
    pub fn register_with_machine(&mut self, machine: AccountId) -> Promise {
        require!(
            env::attached_deposit().as_yoctonear() >= STORAGE_DEPOSIT,
            format!(
                "attach at least {} yocto for machine storage",
                STORAGE_DEPOSIT
            )
        );
        Promise::new(machine).function_call(
            "storage_deposit".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "account_id": env::current_account_id(),
                "registration_only": true,
            }))
            .expect("json"),
            env::attached_deposit(),
            Gas::from_tgas(30),
        )
    }

    /// Registers the squad with its coin so claim_draw can pay it.
    #[payable]
    pub fn register_with_coin(&mut self, squad_id: u64) -> Promise {
        require!(
            env::attached_deposit().as_yoctonear() >= MIN_COIN_REGISTRATION,
            format!(
                "attach at least {} yocto for coin storage",
                MIN_COIN_REGISTRATION
            )
        );
        let squad = self.squads.get(&squad_id).expect("squad not found").clone();
        Promise::new(squad.coin_token).function_call(
            "storage_deposit".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "account_id": env::current_account_id(),
                "registration_only": true,
            }))
            .expect("json"),
            env::attached_deposit(),
            Gas::from_tgas(30),
        )
    }

    pub fn refund_squad(&mut self, squad_id: u64) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(
            squad.status == SquadStatus::Created || squad.status == SquadStatus::Funded,
            "already bought, cannot refund"
        );
        require!(
            env::block_height() > squad.deadline_block,
            "deadline not passed yet"
        );
        require!(!squad.refund_credited, "already refunded");
        squad.refund_credited = true;
        squad.status = SquadStatus::Refunded;
        for member in squad.members.iter() {
            let key = (squad_id, member.clone());
            let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
            if deposit > 0 {
                let cur = self.pending_refunds.get(&key).cloned().unwrap_or(0);
                self.pending_refunds.insert(
                    key.clone(),
                    cur.checked_add(deposit).expect("refund overflow"),
                );
                self.member_deposits.insert(key, 0);
            }
        }
        self.squads.insert(squad_id, squad);
        true
    }

    #[payable]
    pub fn withdraw_refund(&mut self, squad_id: u64) -> Promise {
        let required = FT_GAS + CALLBACK_GAS + 20;
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let caller = env::predecessor_account_id();
        let key = (squad_id, caller.clone());
        let amount = self.pending_refunds.get(&key).cloned().unwrap_or(0);
        require!(amount > 0, "no pending refund");
        self.pending_refunds.insert(key, 0);
        let squad = self.squads.get(&squad_id).expect("squad not found").clone();

        Promise::new(squad.quote_token)
            .function_call(
                "ft_transfer".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "receiver_id": caller,
                    "amount": amount.to_string(),
                    "memo": format!("squad {} refund", squad_id),
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(FT_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_refund_result(squad_id, caller, U128(amount)),
            )
    }

    #[private]
    pub fn on_refund_result(
        &mut self,
        squad_id: u64,
        account: AccountId,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            let key = (squad_id, account);
            let cur = self.pending_refunds.get(&key).cloned().unwrap_or(0);
            self.pending_refunds
                .insert(key, cur.checked_add(amount.0).expect("restore overflow"));
        }
    }

    pub fn get_squad(&self, squad_id: u64) -> Option<SquadView> {
        self.squads.get(&squad_id).map(|s| SquadView {
            id: s.id,
            creator: s.creator.clone(),
            machine: s.machine.clone(),
            quote_token: s.quote_token.clone(),
            coin_token: s.coin_token.clone(),
            pack_price: U128(s.pack_price),
            deposited: U128(s.deposited),
            members: s.members.clone(),
            deadline_block: s.deadline_block,
            status: s.status.clone(),
            prize_total: U128(s.prize_total),
            draw_id: s.draw_id,
            synced_draw_id: s.synced_draw_id,
            distributed: s.distributed,
            refund_credited: s.refund_credited,
        })
    }

    pub fn get_member_deposit(&self, squad_id: u64, account: AccountId) -> U128 {
        U128(
            self.member_deposits
                .get(&(squad_id, account))
                .cloned()
                .unwrap_or(0),
        )
    }

    pub fn get_last_claim_error(&self) -> String {
        self.last_claim_error.clone()
    }

    pub fn get_member_claimable(&self, squad_id: u64, account: AccountId) -> U128 {
        U128(
            self.member_claimable
                .get(&(squad_id, account))
                .cloned()
                .unwrap_or(0),
        )
    }

    pub fn get_pending_refund(&self, squad_id: u64, account: AccountId) -> U128 {
        U128(
            self.pending_refunds
                .get(&(squad_id, account))
                .cloned()
                .unwrap_or(0),
        )
    }
}

impl Contract {
    fn emit_event(event: &str, data: near_sdk::serde_json::Value) {
        let payload = near_sdk::serde_json::json!({
            "standard": "bigo",
            "version": "1.0.0",
            "event": event,
            "data": data,
        });
        env::log_str(&format!("EVENT_JSON:{}", payload));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn setup() -> Contract {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(accounts(0), "factory.sandbox".parse().unwrap())
    }

    fn resolved_squad(c: &mut Contract, id: u64) {
        let mut squad = c.squads.get(&id).unwrap().clone();
        squad.status = SquadStatus::Created;
        c.squads.insert(id, squad);
    }

    #[test]
    fn create_starts_resolving() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(MIN_CREATE_DEPOSIT))
            .build();
        testing_env!(ctx);
        let id = c.create_squad(accounts(2), "100".into(), Some(10));
        assert_eq!(c.get_squad(id).unwrap().status, SquadStatus::Resolving);
    }

    #[test]
    fn machine_info_resolves_tokens() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(MIN_CREATE_DEPOSIT))
            .build();
        testing_env!(ctx);
        let id = c.create_squad(accounts(2), "100".into(), Some(10));
        let info = FactoryMachineInfo {
            coin: accounts(3),
            coin_token: accounts(3),
            machine: accounts(2),
            creator: accounts(1),
            quote_token: accounts(4),
            is_fan_coin: false,
        };
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        assert!(c.on_machine_info(id, Ok(Some(info))));
        let squad = c.get_squad(id).unwrap();
        assert_eq!(squad.status, SquadStatus::Created);
        assert_eq!(squad.coin_token, accounts(3));
        assert_eq!(squad.quote_token, accounts(4));
    }

    #[test]
    #[should_panic(expected = "deadline above")]
    fn deadline_cap_rejected() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(MIN_CREATE_DEPOSIT))
            .build();
        testing_env!(ctx);
        c.create_squad(accounts(2), "100".into(), Some(MAX_DEADLINE_BLOCKS + 1));
    }

    #[test]
    fn member_cap_enforced() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(MIN_CREATE_DEPOSIT))
            .build();
        testing_env!(ctx);
        let id = c.create_squad(accounts(2), "100".into(), Some(100));
        resolved_squad(&mut c, id);
        let mut squad = c.squads.get(&id).unwrap().clone();
        squad.quote_token = accounts(4);
        c.squads.insert(id, squad);
        for i in 0..MAX_MEMBERS {
            let member: AccountId = format!("m{}.sandbox", i).parse().unwrap();
            let ctx = VMContextBuilder::new()
                .predecessor_account_id(accounts(4))
                .build();
            testing_env!(ctx);
            let msg = format!(r#"{{"action":"join_squad","squad_id":{}}}"#, id);
            c.ft_on_transfer(member, U128(1000), msg);
        }
        let extra: AccountId = "extra.sandbox".parse().unwrap();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(4))
            .build();
        testing_env!(ctx);
        let msg = format!(r#"{{"action":"join_squad","squad_id":{}}}"#, id);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            c.ft_on_transfer(extra, U128(1), msg);
        }));
        assert!(res.is_err());
    }

    fn squad_fixture(id: u64, status: SquadStatus) -> Squad {
        Squad {
            id,
            creator: accounts(1),
            machine: accounts(2),
            quote_token: accounts(4),
            coin_token: accounts(3),
            pack_price: 100,
            deposited: 100,
            members: vec![accounts(1), accounts(2)],
            deadline_block: 100,
            status,
            prize_total: 0,
            draw_id: None,
            synced_draw_id: None,
            distributed: false,
            refund_credited: false,
        }
    }

    #[test]
    fn full_failure_marks_refunded_once() {
        let mut c = setup();
        let id = 7;
        c.squads.insert(id, squad_fixture(id, SquadStatus::Funded));
        c.member_deposits.insert((id, accounts(1)), 50);
        c.member_deposits.insert((id, accounts(2)), 50);
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.on_squad_buy_result(id, Err(near_sdk::PromiseError::Failed));
        let squad = c.squads.get(&id).unwrap();
        assert_eq!(squad.status, SquadStatus::Refunded);
        assert!(squad.refund_credited);
        assert_eq!(
            c.member_deposits
                .get(&(id, accounts(1)))
                .cloned()
                .unwrap_or(0),
            0
        );
        assert_eq!(
            c.member_deposits
                .get(&(id, accounts(2)))
                .cloned()
                .unwrap_or(0),
            0
        );
        assert_eq!(
            c.pending_refunds
                .get(&(id, accounts(1)))
                .cloned()
                .unwrap_or(0),
            50
        );
        assert_eq!(
            c.pending_refunds
                .get(&(id, accounts(2)))
                .cloned()
                .unwrap_or(0),
            50
        );
        let second = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            c.refund_squad(id);
        }));
        assert!(second.is_err(), "a second refund must panic");
    }

    #[test]
    fn draw_race_goes_to_buying_failed_then_recovers() {
        let mut c = setup();
        let id = 8;
        c.squads.insert(id, squad_fixture(id, SquadStatus::Bought));
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        assert!(!c.on_last_draw(id, Ok(None)));
        assert_eq!(c.squads.get(&id).unwrap().status, SquadStatus::BuyingFailed);
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .build();
        testing_env!(ctx);
        assert!(c.recover_to_funded(id));
        assert_eq!(c.squads.get(&id).unwrap().status, SquadStatus::Funded);
    }

    #[test]
    fn recover_stuck_squad_requires_no_draw() {
        let mut c = setup();
        let id = 9;
        c.squads.insert(id, squad_fixture(id, SquadStatus::Bought));
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        assert!(c.recover_stuck_squad(id));
        assert_eq!(c.squads.get(&id).unwrap().status, SquadStatus::Funded);
    }

    #[test]
    fn second_sync_rejected() {
        let mut c = setup();
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .attached_deposit(NearToken::from_yoctonear(MIN_CREATE_DEPOSIT))
            .build();
        testing_env!(ctx);
        let id = c.create_squad(accounts(2), "100".into(), Some(100));
        resolved_squad(&mut c, id);
        let mut squad = c.squads.get(&id).unwrap().clone();
        squad.status = SquadStatus::Opening;
        squad.draw_id = Some(1);
        c.squads.insert(id, squad);
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        let draw = MachineDraw {
            draw_id: 1,
            owner: env::current_account_id(),
            status: "Settled".into(),
            prize: Some(MachinePrize {
                tier: "Rare".into(),
                multiplier_bps: 200,
                prize_amount: U128(200),
            }),
            commit_block: env::block_height(),
        };
        let _ = c.on_draw_synced(id, Ok(Some(draw.clone())));
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _ = c.on_draw_synced(id, Ok(Some(draw)));
        }));
        assert!(res.is_err());
    }
}
