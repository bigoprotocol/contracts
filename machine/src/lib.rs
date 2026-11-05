use near_sdk::json_types::U128;
use near_sdk::store::LookupMap;
use near_sdk::{env, near, require, AccountId, Gas, NearToken, PanicOnDefault, Promise};

mod fees;

pub const COIN_TOTAL_SUPPLY: u128 = 1_000_000_000_000_000;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq)]
pub struct PrizeTier {
    pub multiplier_bps: u32,
    pub odds_bps: u32,
    pub tier: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawStatus {
    Committed,
    VrfRequested,
    Settled,
    FallbackSettled,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PackStatus {
    Unopened,
    Opened,
    Listed,
    PendingSale,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GraduationStatus {
    CurveActive,
    GraduationPending,
    Graduated,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PrizeResult {
    pub tier: String,
    pub multiplier_bps: u32,
    pub prize_amount: U128,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct DrawRecord {
    pub draw_id: u64,
    pub owner: AccountId,
    pub pack_coins: u128,
    pub worst_liability: u128,
    pub status: DrawStatus,
    pub prize: Option<PrizeResult>,
    pub commit_block: u64,
    pub table_snapshot_hash: String,
    pub table_snapshot: Vec<PrizeTier>,
    pub claimed: bool,
    pub vrf_request_id: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PackRecord {
    pub token_id: u64,
    pub owner: AccountId,
    pub status: PackStatus,
    pub draw_id: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct DrawView {
    pub draw_id: u64,
    pub owner: AccountId,
    pub pack_coins: U128,
    pub status: DrawStatus,
    pub prize: Option<PrizeResult>,
    pub commit_block: u64,
    pub table_snapshot_hash: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PackView {
    pub token_id: u64,
    pub owner: AccountId,
    pub status: PackStatus,
    pub draw_id: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct MachineView {
    pub owner: AccountId,
    pub coin: AccountId,
    pub coin_token: AccountId,
    pub quote_token: AccountId,
    pub table_hash: String,
    pub pack_art_url: String,
    pub prize_table: Vec<PrizeTier>,
    pub vrf_pubkey: String,
    pub prize_pool: U128,
    pub prize_reserved: U128,
    pub claimable_total: U128,
    pub curve_coin_reserve: U128,
    pub curve_quote_reserve: U128,
    pub paused: bool,
    pub total_packs_sold: u64,
    pub total_packs_opened: u64,
    pub megapack_pot: U128,
    pub megapack_distributed_total: U128,
    pub treasury: AccountId,
    pub graduation_pool_id: Option<u64>,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct TopOpener {
    pub account: AccountId,
    pub opens: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct TopBuyer {
    pub account: AccountId,
    pub packs: u64,
    pub volume: U128,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MyStats {
    pub packs_bought: u64,
    pub packs_opened: u64,
    pub volume: U128,
    pub claimable: U128,
    pub scrap_credits: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct RecentEntry {
    pub draw_id: u64,
    pub owner: AccountId,
    pub prize: Option<PrizeResult>,
    pub block_height: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct Candle {
    pub open: U128,
    pub high: U128,
    pub low: U128,
    pub close: U128,
    pub timestamp: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MachineSummary {
    pub coin: AccountId,
    pub coin_token: AccountId,
    pub total_packs_sold: u64,
    pub total_packs_opened: u64,
    pub total_volume: U128,
    pub prize_pool: U128,
    pub megapack_pot: U128,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct NftTokenView {
    pub token_id: String,
    pub owner_id: AccountId,
    pub metadata: NftMetadata,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct NftMetadata {
    pub title: Option<String>,
    pub description: Option<String>,
    pub media: Option<String>,
    pub reference: Option<String>,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct MilestoneInfo {
    pub threshold: u64,
    pub new_top_mult_bps: u32,
    pub triggered: bool,
    pub current_table: Vec<PrizeTier>,
    pub table_hash: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PackListing {
    pub seller: AccountId,
    pub price: U128,
    pub token_id: u64,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct OutlayerResult {
    pub value: u64,
    pub signature_hex: String,
    pub alpha: String,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct OutlayerEnvelope {
    pub results: Vec<OutlayerResult>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct RheaPoolInfo {
    pub pool_id: u64,
    pub token_account_ids: Vec<AccountId>,
    pub amounts: Vec<U128>,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct CurveState {
    pub quote_reserve: U128,
    pub coin_reserve: U128,
    pub total_sold: U128,
    pub spot_price_num: U128,
    pub spot_price_den: U128,
    pub graduation_target: U128,
    pub is_graduated: bool,
    pub graduation_pool_id: Option<u64>,
    pub tradable_on_rhea: bool,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct PrizePoolView {
    pub prize_pool: U128,
    pub prize_reserved: U128,
    pub claimable_total: U128,
    pub coin_token: AccountId,
}

#[near(serializers = [json])]
#[derive(Debug, Clone)]
pub struct StorageBalance {
    pub total: U128,
    pub available: U128,
}

const MIN_PACK_PRICE: u128 = 1_000;
const PACK_STORAGE_COST: u128 = 5_000_000_000_000_000_000_000;
const MAX_TIERS: usize = 10;
const MAX_TABLE_TOTAL: u64 = 855_000;
const BASE_EV_TOTAL: u64 = 950_000;
const VRF_DEPOSIT: u128 = 1_000_000_000_000_000_000_000;
const VRF_GAS_REQUEST: u64 = 120;
const VRF_GAS_CALLBACK: u64 = 40;
const CALLBACK_GAS: u64 = 15;
const GAS_HEADROOM: u64 = 20;
const FALLBACK_DELAY_BLOCKS: u64 = 50;
const MEGAPACK_HOLD_BLOCKS: u64 = 1000;
const MEGAPACK_CLAIM_COOLDOWN: u64 = 100;
const PACK_FEE_BPS: u32 = 20;
const RESALE_FEE_BPS: u32 = 15;
const TRADE_FEE_BPS: u32 = 100;
const INITIAL_QUOTE_RESERVE: u128 = 100_000_000;
const DEFAULT_CURVE_ALLOCATION: u128 = 800_000_000_000_000;
const DEFAULT_PRIZE_ALLOCATION: u128 = 200_000_000_000_000;
const DEFAULT_MAX_PACK_PRICE: u128 = 1_000_000;
const DEFAULT_SCRAP_DISCOUNT: u128 = 1;
const GRADUATION_TARGET: u128 = 100_000_000_000;
const RHEA_CALL_GAS: u64 = 30;
const RHEA_VIEW_GAS: u64 = 20;
const DEPOSIT_GAS: u64 = 180;
const GRADUATION_CB: u64 = 20;
const RHEA_STORAGE_DEPOSIT: NearToken = NearToken::from_yoctonear(100_000_000_000_000_000_000_000);
const RHEA_POOL_FEE: u32 = 25;
const GRADUATION_SLIPPAGE_BPS: u128 = 500;
const STORAGE_DEPOSIT_MIN: u128 = 10_000_000_000_000_000_000_000;

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub coin: AccountId,
    pub coin_token: AccountId,
    pub quote_token: AccountId,
    pub prize_table: Vec<PrizeTier>,
    pub table_hash: String,
    pub pack_art_url: String,
    pub vrf_pubkey: String,
    pub outlayer_account: AccountId,
    pub vrf_project_id: String,
    pub prize_pool: u128,
    pub prize_reserved: u128,
    pub claimable: LookupMap<AccountId, u128>,
    pub claimable_total: u128,
    pub pending_refunds: LookupMap<AccountId, u128>,
    pub draws: LookupMap<u64, DrawRecord>,
    pub packs: LookupMap<u64, PackRecord>,
    pub next_draw_id: u64,
    pub recent_draws: Vec<RecentEntry>,
    pub candles: Vec<Candle>,
    pub open_counts: LookupMap<AccountId, u64>,
    pub buy_counts: LookupMap<AccountId, u64>,
    pub buy_volume: LookupMap<AccountId, u128>,
    pub scrap_credit: LookupMap<AccountId, u64>,
    pub megapack_pot: u128,
    pub megapack_distributed_total: u128,
    pub paused: bool,
    pub total_packs_sold: u64,
    pub total_packs_opened: u64,
    pub total_volume: u128,
    pub protocol_fees: u128,
    pub creator_fees: u128,
    pub protocol_fees_coin: u128,
    pub creator_fees_coin: u128,
    pub reserved_discounts: LookupMap<AccountId, u128>,
    pub pending_coin_refunds: LookupMap<AccountId, u128>,
    pub pending_coin_refunds_total: u128,
    pub pending_protocol_fees: u128,
    pub owner_tokens: LookupMap<AccountId, Vec<u64>>,
    pub last_draw_by_owner: LookupMap<AccountId, u64>,
    pub vrf_request_ids: LookupMap<String, u64>,
    pub treasury: AccountId,
    pub fan_coin: bool,
    pub graduation_target: u128,
    pub graduation_pool_id: Option<u64>,
    pub graduation_coin_amount: u128,
    pub graduation_quote_amount: u128,
    pub graduation_coin_seeded: bool,
    pub graduation_phase: u8,
    pub graduation_coin_first: bool,
    pub graduation_progress_block: u64,
    pub stranded_coin: u128,
    pub stranded_quote: u128,
    pub graduation_error: String,
    pub rhea: AccountId,
    pub listings: LookupMap<u64, PackListing>,
    pub max_mult_bps: u32,
    pub min_mult_bps: u32,
    pub curve_quote_reserve: u128,
    pub curve_coin_reserve: u128,
    pub curve_total_sold: u128,
    pub curve_coin_allocation: u128,
    pub prize_coin_allocation: u128,
    pub max_pack_price: u128,
    pub scrap_discount_amount: u128,
    pub coin_balance_cached: u128,
    pub graduation_status: GraduationStatus,
    pub graduation_block: Option<u64>,
    pub rhea_pool_exists: bool,
    pub lp_locked: bool,
    pub largest_pack_unlocked: bool,
    pub milestone_threshold: u64,
    pub milestone_new_top_mult_bps: u32,
    pub milestone_triggered: bool,
    pub scrap_discount_next: LookupMap<AccountId, bool>,
    pub megapack_eligible_from_block: LookupMap<AccountId, u64>,
    pub megapack_last_claim_block: LookupMap<AccountId, u64>,
    pub megapack_holders: LookupMap<AccountId, u64>,
    pub megapack_holder_count: u64,
    pub top_openers_vec: Vec<TopOpener>,
    pub top_buyers_vec: Vec<TopBuyer>,
    pub ft_gas: u64,
    pub storage_registered: LookupMap<AccountId, bool>,
    pub storage_balances: LookupMap<AccountId, u128>,
}

impl Contract {
    fn assert_not_paused(&self) {
        require!(!self.paused, "contract is paused");
        require!(
            self.graduation_status != GraduationStatus::GraduationPending,
            "graduation pending - purchases paused"
        );
    }

    /// The machine's cached coin balance must cover the curve partition plus the
    /// prize partition. Refreshed with sync_coin_balance after external transfers.
    pub(crate) fn assert_coin_solvent(&self) {
        let required = self
            .curve_coin_reserve
            .checked_add(self.prize_pool)
            .expect("coin required overflow")
            .checked_add(self.pending_coin_refunds_total)
            .expect("coin required overflow");
        require!(
            self.coin_balance_cached >= required,
            format!(
                "coin insolvency: balance {} < curve {} + prize {}",
                self.coin_balance_cached, self.curve_coin_reserve, self.prize_pool
            )
        );
        let reserved = self
            .prize_reserved
            .checked_add(self.claimable_total)
            .expect("prize reserved overflow");
        require!(
            reserved <= self.prize_pool,
            format!(
                "prize pool breach: reserved {} + claimable {} > pool {}",
                self.prize_reserved, self.claimable_total, self.prize_pool
            )
        );
    }

    pub(crate) fn hex_decode(s: &str) -> Vec<u8> {
        let t = s.trim().trim_start_matches("0x");
        if !t.len().is_multiple_of(2) {
            return Vec::new();
        }
        let mut out = Vec::with_capacity(t.len() / 2);
        let mut i = 0;
        while i < t.len() {
            match u8::from_str_radix(&t[i..i + 2], 16) {
                Ok(b) => out.push(b),
                Err(_) => return Vec::new(),
            }
            i += 2;
        }
        out
    }

    fn add_owner_token(&mut self, account: &AccountId, id: u64) {
        let mut ids = self.owner_tokens.get(account).cloned().unwrap_or_default();
        if !ids.contains(&id) {
            ids.push(id);
            self.owner_tokens.insert(account.clone(), ids);
        }
    }

    fn remove_owner_token(&mut self, account: &AccountId, id: u64) {
        if let Some(mut ids) = self.owner_tokens.get(account).cloned() {
            ids.retain(|t| *t != id);
            self.owner_tokens.insert(account.clone(), ids);
        }
    }

    fn emit_event(event: &str, data: near_sdk::serde_json::Value) {
        let payload = near_sdk::serde_json::json!({
            "standard": "bigo",
            "version": "1.0.0",
            "event": event,
            "data": data,
        });
        env::log_str(&format!("EVENT_JSON:{}", payload));
    }

    fn validate_table(table: &[PrizeTier]) -> Result<(), String> {
        if table.is_empty() {
            return Err("empty prize table".into());
        }
        if table.len() > MAX_TIERS {
            return Err(format!("tiers {}>{}", table.len(), MAX_TIERS));
        }
        let mut total: u64 = 0;
        let mut odds_sum: u64 = 0;
        for t in table {
            if t.multiplier_bps == 0 {
                return Err("multiplier must be >0".into());
            }
            if t.odds_bps == 0 {
                return Err("odds must be >0".into());
            }
            odds_sum = odds_sum
                .checked_add(t.odds_bps as u64)
                .ok_or("odds overflow")?;
            let prod = (t.odds_bps as u64)
                .checked_mul(t.multiplier_bps as u64)
                .ok_or("product overflow")?;
            total = total.checked_add(prod).ok_or("total overflow")?;
        }
        if odds_sum != 10000 {
            return Err(format!("odds must sum to 10000, got {}", odds_sum));
        }
        if total > BASE_EV_TOTAL {
            return Err(format!(
                "EV total {} exceeds hard cap {}",
                total, BASE_EV_TOTAL
            ));
        }
        if total > MAX_TABLE_TOTAL {
            return Err(format!(
                "EV total {} exceeds discounted cap {}",
                total, MAX_TABLE_TOTAL
            ));
        }
        Ok(())
    }

    fn check_prize_table(table: &[PrizeTier]) {
        if let Err(msg) = Self::validate_table(table) {
            env::panic_str(&msg);
        }
    }

    fn table_hash(table: &[PrizeTier]) -> String {
        let table_str = table
            .iter()
            .map(|t| format!("{}:{}:{};", t.tier, t.multiplier_bps, t.odds_bps))
            .collect::<String>();
        let hash_bytes = env::sha256(table_str.as_bytes());
        hash_bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }

    fn upgraded_table(table: &[PrizeTier], new_top_mult_bps: u32) -> Option<Vec<PrizeTier>> {
        let mut new_table = table.to_vec();
        let mut max_idx: Option<usize> = None;
        let mut max_mult = 0u32;
        for (i, t) in new_table.iter().enumerate() {
            if t.multiplier_bps > max_mult {
                max_mult = t.multiplier_bps;
                max_idx = Some(i);
            }
        }
        let idx = max_idx?;
        if max_mult >= new_top_mult_bps {
            return None;
        }
        let old_odds = new_table[idx].odds_bps;
        let new_odds =
            ((old_odds as u64 * max_mult as u64 / new_top_mult_bps as u64) as u32).max(1);
        let odds_diff = old_odds.checked_sub(new_odds)?;
        let mut min_idx: Option<usize> = None;
        let mut min_mult = u32::MAX;
        for (i, t) in new_table.iter().enumerate() {
            if i != idx && t.multiplier_bps < min_mult {
                min_mult = t.multiplier_bps;
                min_idx = Some(i);
            }
        }
        new_table[idx].multiplier_bps = new_top_mult_bps;
        new_table[idx].odds_bps = new_odds;
        if let Some(mi) = min_idx {
            new_table[mi].odds_bps = new_table[mi].odds_bps.checked_add(odds_diff)?;
        }
        Some(new_table)
    }

    fn apply_milestone(&mut self) {
        if self.milestone_triggered {
            return;
        }
        let Some(new_table) =
            Self::upgraded_table(&self.prize_table, self.milestone_new_top_mult_bps)
        else {
            return;
        };
        if Self::validate_table(&new_table).is_err() {
            Self::emit_event(
                "milestone_skipped",
                near_sdk::serde_json::json!({"reason": "table failed validation"}),
            );
            return;
        }
        self.prize_table = new_table;
        self.max_mult_bps = self
            .prize_table
            .iter()
            .map(|t| t.multiplier_bps)
            .max()
            .unwrap_or(0);
        self.min_mult_bps = self
            .prize_table
            .iter()
            .map(|t| t.multiplier_bps)
            .min()
            .unwrap_or(0);
        self.table_hash = Self::table_hash(&self.prize_table);
        self.milestone_triggered = true;
        Self::emit_event(
            "milestone",
            near_sdk::serde_json::json!({
                "table_hash": self.table_hash,
                "new_top_mult_bps": self.milestone_new_top_mult_bps,
            }),
        );
    }

    fn update_top_buyer(&mut self, account: AccountId, packs: u64, volume: u128) {
        let volume = U128(volume);
        if let Some(entry) = self
            .top_buyers_vec
            .iter_mut()
            .find(|e| e.account == account)
        {
            entry.packs = packs;
            entry.volume = volume;
        } else if self.top_buyers_vec.len() < 10 {
            self.top_buyers_vec.push(TopBuyer {
                account,
                packs,
                volume,
            });
        } else {
            let mut min_idx = 0usize;
            let mut min_packs = u64::MAX;
            for (i, e) in self.top_buyers_vec.iter().enumerate() {
                if e.packs < min_packs {
                    min_packs = e.packs;
                    min_idx = i;
                }
            }
            if packs > min_packs {
                self.top_buyers_vec[min_idx] = TopBuyer {
                    account,
                    packs,
                    volume,
                };
            } else {
                return;
            }
        }
        self.top_buyers_vec
            .sort_by_key(|b| std::cmp::Reverse(b.packs));
    }

    fn update_top_opener(&mut self, account: AccountId, opens: u64) {
        if let Some(entry) = self
            .top_openers_vec
            .iter_mut()
            .find(|e| e.account == account)
        {
            entry.opens = opens;
        } else if self.top_openers_vec.len() < 20 {
            self.top_openers_vec.push(TopOpener { account, opens });
        } else {
            let mut min_idx = 0usize;
            let mut min_opens = u64::MAX;
            for (i, e) in self.top_openers_vec.iter().enumerate() {
                if e.opens < min_opens {
                    min_opens = e.opens;
                    min_idx = i;
                }
            }
            if opens > min_opens {
                self.top_openers_vec[min_idx] = TopOpener { account, opens };
            } else {
                return;
            }
        }
        self.top_openers_vec
            .sort_by_key(|b| std::cmp::Reverse(b.opens));
    }

    fn require_registered(&self, account: &AccountId) {
        require!(
            self.storage_registered
                .get(account)
                .cloned()
                .unwrap_or(false),
            format!("storage deposit required for {}", account)
        );
    }

    fn coin_transfer(&self, receiver: AccountId, amount: u128, memo: &str) -> Promise {
        Promise::new(self.coin_token.clone()).function_call(
            "ft_transfer".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "receiver_id": receiver,
                "amount": U128(amount),
                "memo": memo,
            }))
            .expect("json"),
            NearToken::from_yoctonear(1),
            Gas::from_tgas(self.ft_gas),
        )
    }

    fn quote_transfer(&self, receiver: AccountId, amount: u128, memo: &str) -> Promise {
        Promise::new(self.quote_token.clone()).function_call(
            "ft_transfer".to_string(),
            near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                "receiver_id": receiver,
                "amount": amount.to_string(),
                "memo": memo,
            }))
            .expect("json"),
            NearToken::from_yoctonear(1),
            Gas::from_tgas(self.ft_gas),
        )
    }

    fn settle_draw_internal(&mut self, draw_id: u64, chosen: PrizeTier, fallback: bool) -> u128 {
        let draw = self.draws.get(&draw_id).expect("draw not found").clone();
        self.prize_reserved = self
            .prize_reserved
            .checked_sub(draw.worst_liability)
            .unwrap_or_else(|| env::panic_str("prize reserved underflow"));
        let prize = draw
            .pack_coins
            .checked_mul(chosen.multiplier_bps as u128)
            .expect("prize overflow")
            / 100;
        let prize_result = PrizeResult {
            tier: chosen.tier.clone(),
            multiplier_bps: chosen.multiplier_bps,
            prize_amount: U128(prize),
        };
        let mut settled = draw.clone();
        settled.prize = Some(prize_result.clone());
        settled.status = if fallback {
            DrawStatus::FallbackSettled
        } else {
            DrawStatus::Settled
        };
        self.draws.insert(draw_id, settled);

        let cur = self.claimable.get(&draw.owner).cloned().unwrap_or(0);
        self.claimable.insert(
            draw.owner.clone(),
            cur.checked_add(prize).expect("claim overflow"),
        );
        self.claimable_total = self
            .claimable_total
            .checked_add(prize)
            .expect("claimable overflow");
        self.assert_coin_solvent();

        if chosen.multiplier_bps == 100 {
            let cur_scrap = self.scrap_credit.get(&draw.owner).cloned().unwrap_or(0);
            self.scrap_credit.insert(
                draw.owner.clone(),
                cur_scrap.checked_add(1).expect("scrap overflow"),
            );
        }
        for entry in self.recent_draws.iter_mut() {
            if entry.draw_id == draw_id {
                entry.prize = Some(prize_result.clone());
                break;
            }
        }
        prize
    }

    fn choose_tier(table: &[PrizeTier], roll: u64) -> PrizeTier {
        let mut cumulative: u64 = 0;
        for tier in table {
            cumulative = cumulative
                .checked_add(tier.odds_bps as u64)
                .expect("odds overflow");
            if roll < cumulative {
                return tier.clone();
            }
        }
        table.last().expect("empty table").clone()
    }

    fn curve_quote_out(&self, net: u128) -> u128 {
        let numerator = self
            .curve_quote_reserve
            .checked_mul(net)
            .expect("curve overflow");
        let denominator = self
            .curve_coin_reserve
            .checked_add(net)
            .expect("curve overflow");
        numerator / denominator
    }

    fn curve_coins_out(&self, quote_net: u128) -> u128 {
        let numerator = self
            .curve_coin_reserve
            .checked_mul(quote_net)
            .expect("curve overflow");
        let denominator = self
            .curve_quote_reserve
            .checked_add(quote_net)
            .expect("curve overflow");
        if denominator == 0 {
            return 0;
        }
        numerator / denominator
    }

    fn buy_pack_internal_quote(&mut self, buyer: AccountId, amount: u128) -> u128 {
        require!(
            self.graduation_status != GraduationStatus::Graduated,
            "packs paused after graduation"
        );
        self.assert_not_paused();
        self.require_registered(&buyer);
        let storage = self.storage_balances.get(&buyer).cloned().unwrap_or(0);
        require!(
            storage >= PACK_STORAGE_COST,
            format!(
                "insufficient storage balance, need {} yocto, deposit more",
                PACK_STORAGE_COST
            )
        );
        self.storage_balances
            .insert(buyer.clone(), storage - PACK_STORAGE_COST);
        require!(
            amount >= MIN_PACK_PRICE,
            format!("min pack price {}", MIN_PACK_PRICE)
        );
        require!(
            amount <= self.max_pack_price,
            format!("pack price {} exceeds max {}", amount, self.max_pack_price)
        );

        let fee = fees::fee_amount(amount, PACK_FEE_BPS);
        let net = amount.checked_sub(fee).expect("fee exceeds pack");
        // the pack buys coins on the internal market, the machine already holds both assets
        let coins_out = self.curve_coins_out(net);
        require!(coins_out > 0, "dust coin output");
        let worst = coins_out
            .checked_mul(self.max_mult_bps as u128)
            .expect("worst liability overflow")
            / 100;
        let free = self
            .prize_pool
            .checked_sub(self.prize_reserved)
            .and_then(|v| v.checked_sub(self.claimable_total))
            .unwrap_or(0);
        require!(
            free.checked_add(coins_out).unwrap_or(0) >= worst,
            format!(
                "insufficient pool: free {} + bought {} < worst {}",
                free, coins_out, worst
            )
        );

        self.curve_quote_reserve = self
            .curve_quote_reserve
            .checked_add(net)
            .expect("reserve overflow");
        self.curve_coin_reserve = self
            .curve_coin_reserve
            .checked_sub(coins_out)
            .expect("reserve underflow");
        self.curve_total_sold = self
            .curve_total_sold
            .checked_add(coins_out)
            .expect("sold overflow");
        self.prize_pool = self
            .prize_pool
            .checked_add(coins_out)
            .expect("prize pool overflow");
        self.prize_reserved = self
            .prize_reserved
            .checked_add(worst)
            .expect("reserved overflow");

        // pack buys are paused after graduation, so the megapack share is not
        // reachable here. the whole fee goes to protocol fees.
        self.protocol_fees = self.protocol_fees.checked_add(fee).expect("fee overflow");

        let mut refund = 0u128;
        if self
            .scrap_discount_next
            .get(&buyer)
            .cloned()
            .unwrap_or(false)
        {
            let available = self.reserved_discounts.get(&buyer).cloned().unwrap_or(0);
            let discount = (amount / 10).min(available);
            if discount > 0 {
                self.reserved_discounts
                    .insert(buyer.clone(), available - discount);
                refund = discount;
            }
            self.scrap_discount_next.insert(buyer.clone(), false);
        }

        let draw_id = self.next_draw_id;
        self.next_draw_id = self.next_draw_id.checked_add(1).expect("draw id overflow");
        self.draws.insert(
            draw_id,
            DrawRecord {
                draw_id,
                owner: buyer.clone(),
                pack_coins: coins_out,
                worst_liability: worst,
                status: DrawStatus::Committed,
                prize: None,
                commit_block: env::block_height(),
                table_snapshot_hash: self.table_hash.clone(),
                table_snapshot: self.prize_table.clone(),
                claimed: false,
                vrf_request_id: String::new(),
            },
        );
        self.packs.insert(
            draw_id,
            PackRecord {
                token_id: draw_id,
                owner: buyer.clone(),
                status: PackStatus::Unopened,
                draw_id,
            },
        );
        self.add_owner_token(&buyer, draw_id);
        self.last_draw_by_owner.insert(buyer.clone(), draw_id);

        let cnt = self.buy_counts.get(&buyer).cloned().unwrap_or(0) + 1;
        self.buy_counts.insert(buyer.clone(), cnt);
        let vol = self.buy_volume.get(&buyer).cloned().unwrap_or(0) + amount;
        self.buy_volume.insert(buyer.clone(), vol);
        self.update_top_buyer(buyer.clone(), cnt, vol);

        self.total_packs_sold = self.total_packs_sold.checked_add(1).expect("sold overflow");
        self.total_volume = self
            .total_volume
            .checked_add(amount)
            .expect("volume overflow");
        self.assert_coin_solvent();

        Self::emit_event(
            "pack_bought",
            near_sdk::serde_json::json!({
                "account": buyer,
                "token_id": draw_id,
                "draw_id": draw_id,
                "quote_in": amount.to_string(),
                "coins_out": coins_out.to_string(),
                "fee": fee.to_string(),
                "refund": refund.to_string(),
            }),
        );
        refund
    }

    fn curve_buy_internal_quote(
        &mut self,
        buyer: AccountId,
        quote_in: u128,
        min_coins_out: Option<u128>,
    ) -> u128 {
        self.assert_not_paused();
        self.require_registered(&buyer);
        require!(quote_in > 0, "quote amount must be >0");
        let fee_total = quote_in / (10000 / TRADE_FEE_BPS as u128);
        let creator_fee = fee_total / 2;
        let protocol_fee = fee_total.checked_sub(creator_fee).expect("fee split");
        let net = quote_in.checked_sub(fee_total).expect("fee exceeds quote");
        require!(net > 0, "net quote too small");
        let coins_out = self.curve_coins_out(net);
        require!(coins_out > 0, "dust coin output");
        if let Some(min) = min_coins_out {
            require!(
                coins_out >= min,
                format!("slippage: got {} < min {}", coins_out, min)
            );
        }

        // no reserve mutation until the coin transfer commits
        self.coin_transfer(buyer.clone(), coins_out, "curve buy")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_curve_buy_result(
                        buyer.clone(),
                        U128(coins_out),
                        U128(quote_in),
                        U128(creator_fee),
                        U128(protocol_fee),
                        U128(net),
                    ),
            )
            .detach();
        0
    }

    fn curve_sell_internal_coin(
        &mut self,
        seller: AccountId,
        coin_in: u128,
        min_quote_out: Option<u128>,
    ) -> u128 {
        require!(
            self.graduation_status != GraduationStatus::Graduated,
            format!(
                "curve retired, trade on Rhea pool {}",
                self.graduation_pool_id.unwrap_or(0)
            )
        );
        self.assert_not_paused();
        self.require_registered(&seller);
        require!(coin_in > 0, "coin amount must be >0");
        let fee_total = coin_in / (10000 / TRADE_FEE_BPS as u128);
        let creator_fee = fee_total / 2;
        let protocol_fee = fee_total.checked_sub(creator_fee).expect("fee split");
        let net = coin_in.checked_sub(fee_total).expect("fee exceeds coin");
        require!(net > 0, "net coin too small");
        let numerator = self
            .curve_quote_reserve
            .checked_mul(net)
            .expect("curve overflow");
        let denominator = self
            .curve_coin_reserve
            .checked_add(net)
            .expect("curve overflow");
        let quote_out = numerator / denominator;
        require!(quote_out > 0, "dust quote output");
        if let Some(min) = min_quote_out {
            require!(
                quote_out >= min,
                format!("slippage: got {} < min {}", quote_out, min)
            );
        }
        require!(
            quote_out < self.curve_quote_reserve,
            "trade would zero the quote reserve"
        );

        // the coins just entered the machine, so the cache grows now
        self.coin_balance_cached = self
            .coin_balance_cached
            .checked_add(coin_in)
            .expect("coin balance overflow");
        // no reserve mutation until the quote transfer commits
        self.quote_transfer(seller.clone(), quote_out, "curve sell")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_sell_result(
                        seller,
                        U128(coin_in),
                        U128(net),
                        U128(protocol_fee),
                        U128(creator_fee),
                        U128(quote_out),
                    ),
            )
            .detach();

        0
    }

    fn buy_listing_internal(&mut self, buyer: AccountId, token_id: u64, amount: u128) -> u128 {
        self.assert_not_paused();
        self.require_registered(&buyer);
        let listing = self.listings.get(&token_id).expect("not listed").clone();
        require!(listing.seller != buyer, "cannot buy own listing");
        require!(
            amount >= listing.price.0,
            format!("paid {} below price {}", amount, listing.price.0)
        );
        let pack = self.packs.get(&token_id).expect("pack not found").clone();
        require!(pack.owner == listing.seller, "seller no longer owns pack");
        require!(pack.status == PackStatus::Listed, "pack is not listed");

        let price = listing.price.0;
        let fee = fees::fee_amount(price, RESALE_FEE_BPS);
        let seller_proceeds = price.checked_sub(fee).expect("fee exceeds price");
        // hold the sale: ownership moves only after the seller is paid
        let mut pending = pack.clone();
        pending.status = PackStatus::PendingSale;
        self.packs.insert(token_id, pending);
        self.pending_protocol_fees = self
            .pending_protocol_fees
            .checked_add(fee)
            .expect("pending fee overflow");

        let seller = listing.seller.clone();
        self.quote_transfer(seller.clone(), seller_proceeds, "pack sale")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_seller_paid(buyer, seller, token_id, U128(price)),
            )
            .detach();

        amount - price
    }

    fn encode_svg(svg: &str) -> String {
        let mut out = String::with_capacity(svg.len() * 2);
        for c in svg.chars() {
            match c {
                '%' => out.push_str("%25"),
                '#' => out.push_str("%23"),
                '<' => out.push_str("%3C"),
                '>' => out.push_str("%3E"),
                '"' => out.push_str("%22"),
                ' ' => out.push_str("%20"),
                _ => out.push(c),
            }
        }
        out
    }
}

#[near]
impl Contract {
    // deploy and init must be submitted as one batched transaction. the factory
    // does this with create_account + deploy + new in a single promise chain.
    #[init]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        owner: AccountId,
        coin: AccountId,
        coin_token: AccountId,
        quote_token: Option<AccountId>,
        prize_table: Vec<PrizeTier>,
        pack_art_url: String,
        vrf_pubkey: String,
        outlayer_account: AccountId,
        vrf_project_id: String,
        milestone_threshold: Option<u64>,
        milestone_new_top_mult_bps: Option<u32>,
        ft_gas: Option<u64>,
        curve_coin_allocation: Option<U128>,
        prize_coin_allocation: Option<U128>,
        max_pack_price: Option<U128>,
        scrap_discount_amount: Option<U128>,
        treasury: Option<AccountId>,
        fan_coin: Option<bool>,
        initial_coin_balance: Option<U128>,
        graduation_target: Option<U128>,
        rhea: Option<AccountId>,
    ) -> Self {
        assert!(!env::state_exists(), "already initialized");
        require!(!pack_art_url.is_empty(), "empty art url");
        require!(
            pack_art_url.starts_with("ipfs://") || pack_art_url.starts_with("https://"),
            format!(
                "invalid art url: must start with ipfs:// or https:// got {}",
                pack_art_url
            )
        );
        Self::check_prize_table(&prize_table);
        let pk = Self::hex_decode(&vrf_pubkey);
        require!(
            pk.len() == 32,
            "vrf_pubkey must be 64 hex characters (32 bytes)"
        );
        require!(
            !vrf_project_id.is_empty(),
            "vrf_project_id must not be empty"
        );

        let curve_alloc = curve_coin_allocation
            .map(|v| v.0)
            .unwrap_or(DEFAULT_CURVE_ALLOCATION);
        let prize_alloc = prize_coin_allocation
            .map(|v| v.0)
            .unwrap_or(DEFAULT_PRIZE_ALLOCATION);
        require!(curve_alloc > 0, "curve allocation must be > 0");
        require!(prize_alloc > 0, "prize allocation must be > 0");
        let allocated = curve_alloc
            .checked_add(prize_alloc)
            .expect("allocation overflow");
        let fan_coin = fan_coin.unwrap_or(false);
        let initial_balance = initial_coin_balance
            .map(|v| v.0)
            .unwrap_or(COIN_TOTAL_SUPPLY);
        if !fan_coin {
            require!(
                allocated <= initial_balance,
                format!(
                    "allocations {} exceed machine coin balance {}",
                    allocated, initial_balance
                )
            );
        }

        let max_pack_price = max_pack_price
            .map(|v| v.0)
            .unwrap_or(DEFAULT_MAX_PACK_PRICE);
        require!(max_pack_price > 0, "max_pack_price must be > 0");
        let max_fee = fees::fee_amount(max_pack_price, PACK_FEE_BPS);
        let max_net = max_pack_price
            .checked_sub(max_fee)
            .expect("fee exceeds max pack");
        require!(max_net > 0, "max pack net too small");
        let coins_for_max = {
            let numerator = curve_alloc.checked_mul(max_net).expect("curve overflow");
            let denominator = INITIAL_QUOTE_RESERVE
                .checked_add(max_net)
                .expect("curve overflow");
            numerator / denominator
        };
        require!(coins_for_max > 0, "max pack buys no coins");
        // price impact <= 10 percent: exec <= 1.1 * spot, cross multiplied
        let lhs = max_net
            .checked_mul(curve_alloc)
            .expect("impact overflow")
            .checked_mul(10)
            .expect("impact overflow");
        let rhs = INITIAL_QUOTE_RESERVE
            .checked_mul(coins_for_max)
            .expect("impact overflow")
            .checked_mul(11)
            .expect("impact overflow");
        require!(
            lhs <= rhs,
            format!(
                "max pack price {} has more than 10 percent price impact",
                max_pack_price
            )
        );
        let max_mult = prize_table
            .iter()
            .map(|t| t.multiplier_bps)
            .max()
            .unwrap_or(0);
        let max_prize_coins = coins_for_max
            .checked_mul(max_mult as u128)
            .expect("prize guard overflow")
            / 100;
        let required_prize = max_prize_coins
            .checked_mul(30)
            .expect("prize guard overflow");
        require!(
            prize_alloc >= required_prize,
            format!(
                "prize allocation {} does not cover 30 max prizes ({})",
                prize_alloc, required_prize
            )
        );

        let milestone_threshold = milestone_threshold.unwrap_or(50);
        let milestone_new_top_mult_bps = milestone_new_top_mult_bps.unwrap_or(1000);
        if let Some(upgraded) = Self::upgraded_table(&prize_table, milestone_new_top_mult_bps) {
            Self::check_prize_table(&upgraded);
        }
        let table_hash = Self::table_hash(&prize_table);
        let min_mult = prize_table
            .iter()
            .map(|t| t.multiplier_bps)
            .min()
            .unwrap_or(0);
        let quote = quote_token.unwrap_or_else(|| "usdc.bigo.testnet".parse().unwrap());
        let treasury = treasury.unwrap_or_else(|| owner.clone());
        Self {
            owner,
            coin,
            coin_token,
            quote_token: quote,
            prize_table,
            table_hash,
            pack_art_url,
            vrf_pubkey,
            outlayer_account,
            vrf_project_id,
            prize_pool: prize_alloc,
            prize_reserved: 0,
            claimable: LookupMap::new(b"c"),
            claimable_total: 0,
            pending_refunds: LookupMap::new(b"r"),
            draws: LookupMap::new(b"d"),
            packs: LookupMap::new(b"p"),
            next_draw_id: 1,
            recent_draws: Vec::new(),
            candles: Vec::new(),
            open_counts: LookupMap::new(b"o"),
            buy_counts: LookupMap::new(b"b"),
            buy_volume: LookupMap::new(b"v"),
            scrap_credit: LookupMap::new(b"s"),
            megapack_pot: 0,
            megapack_distributed_total: 0,
            paused: false,
            total_packs_sold: 0,
            total_packs_opened: 0,
            total_volume: 0,
            protocol_fees: 0,
            creator_fees: 0,
            protocol_fees_coin: 0,
            creator_fees_coin: 0,
            reserved_discounts: LookupMap::new(b"z"),
            pending_coin_refunds: LookupMap::new(b"x"),
            pending_coin_refunds_total: 0,
            pending_protocol_fees: 0,
            owner_tokens: LookupMap::new(b"y"),
            last_draw_by_owner: LookupMap::new(b"w"),
            vrf_request_ids: LookupMap::new(b"v"),
            treasury,
            fan_coin,
            graduation_target: graduation_target.map(|v| v.0).unwrap_or(GRADUATION_TARGET),
            graduation_pool_id: None,
            graduation_coin_amount: 0,
            graduation_quote_amount: 0,
            graduation_coin_seeded: false,
            graduation_phase: 0,
            graduation_coin_first: true,
            graduation_progress_block: 0,
            stranded_coin: 0,
            stranded_quote: 0,
            graduation_error: String::new(),
            rhea: rhea.unwrap_or_else(|| "ref-finance-101.testnet".parse().unwrap()),
            listings: LookupMap::new(b"l"),
            max_mult_bps: max_mult,
            min_mult_bps: min_mult,
            curve_quote_reserve: INITIAL_QUOTE_RESERVE,
            curve_coin_reserve: curve_alloc,
            curve_total_sold: 0,
            curve_coin_allocation: curve_alloc,
            prize_coin_allocation: prize_alloc,
            max_pack_price,
            scrap_discount_amount: scrap_discount_amount
                .map(|v| v.0)
                .unwrap_or(DEFAULT_SCRAP_DISCOUNT),
            coin_balance_cached: initial_balance,
            graduation_status: GraduationStatus::CurveActive,
            graduation_block: None,
            rhea_pool_exists: false,
            lp_locked: false,
            largest_pack_unlocked: false,
            milestone_threshold,
            milestone_new_top_mult_bps,
            milestone_triggered: false,
            scrap_discount_next: LookupMap::new(b"q"),
            megapack_eligible_from_block: LookupMap::new(b"e"),
            megapack_last_claim_block: LookupMap::new(b"f"),
            megapack_holders: LookupMap::new(b"h"),
            megapack_holder_count: 0,
            top_openers_vec: Vec::new(),
            top_buyers_vec: Vec::new(),
            ft_gas: ft_gas.unwrap_or(30),
            storage_registered: LookupMap::new(b"u"),
            storage_balances: LookupMap::new(b"t"),
        }
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
        let existing = self.storage_balances.get(&account).cloned().unwrap_or(0);
        let total = existing
            .checked_add(amount)
            .expect("storage balance overflow");
        if !self
            .storage_registered
            .get(&account)
            .cloned()
            .unwrap_or(false)
        {
            require!(
                total >= STORAGE_DEPOSIT_MIN,
                format!("minimum storage deposit is {} yocto", STORAGE_DEPOSIT_MIN)
            );
            self.storage_registered.insert(account.clone(), true);
        }
        self.storage_balances.insert(account.clone(), total);
        StorageBalance {
            total: U128(total),
            available: U128(0),
        }
    }

    pub fn storage_balance_of(&self, account_id: AccountId) -> Option<StorageBalance> {
        self.storage_balances
            .get(&account_id)
            .map(|total| StorageBalance {
                total: U128(*total),
                available: U128(0),
            })
    }

    /// Called by the quote token or the coin token. Actions:
    /// quote: buy_pack, seed_pool, buy_listing, curve_buy
    /// coin: curve_sell
    pub fn ft_on_transfer(&mut self, sender_id: AccountId, amount: U128, msg: String) -> U128 {
        let token = env::predecessor_account_id();
        let parsed: near_sdk::serde_json::Value = near_sdk::serde_json::from_str(&msg)
            .unwrap_or_else(|_| env::panic_str("invalid msg json"));
        let action = parsed
            .get("action")
            .and_then(|v| v.as_str())
            .unwrap_or_else(|| env::panic_str("msg.action required"));
        if token == self.quote_token {
            return match action {
                "buy_pack" => U128(self.buy_pack_internal_quote(sender_id, amount.0)),
                "seed_pool" => {
                    require!(sender_id == self.owner, "only the machine owner can seed");
                    self.curve_quote_reserve = self
                        .curve_quote_reserve
                        .checked_add(amount.0)
                        .expect("reserve overflow");
                    U128(0)
                }
                "buy_listing" => {
                    let token_id = parsed
                        .get("token_id")
                        .and_then(|v| v.as_u64())
                        .unwrap_or_else(|| env::panic_str("msg.token_id required"));
                    U128(self.buy_listing_internal(sender_id, token_id, amount.0))
                }
                "curve_buy" => {
                    let min_out = parsed.get("min_coins_out").and_then(|v| {
                        v.as_u64()
                            .map(|n| n as u128)
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    });
                    U128(self.curve_buy_internal_quote(sender_id, amount.0, min_out))
                }
                _ => env::panic_str(&format!("unknown quote action {}", action)),
            };
        }
        if token == self.coin_token {
            return match action {
                "curve_sell" => {
                    let min_out = parsed.get("min_quote_out").and_then(|v| {
                        v.as_u64()
                            .map(|n| n as u128)
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    });
                    U128(self.curve_sell_internal_coin(sender_id, amount.0, min_out))
                }
                _ => env::panic_str(&format!("unknown coin action {}", action)),
            };
        }
        env::panic_str(&format!("unsupported token {}", token))
    }

    #[private]
    #[allow(clippy::too_many_arguments)]
    pub fn on_curve_buy_result(
        &mut self,
        buyer: AccountId,
        coins_out: U128,
        quote_in: U128,
        creator_fee: U128,
        protocol_fee: U128,
        net: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            // nothing was mutated: only credit the buyer's quote refund
            let cur = self.pending_refunds.get(&buyer).cloned().unwrap_or(0);
            self.pending_refunds.insert(
                buyer.clone(),
                cur.checked_add(quote_in.0).expect("refund overflow"),
            );
            Self::emit_event(
                "curve_buy_reverted",
                near_sdk::serde_json::json!({
                    "account": buyer,
                    "quote_in": quote_in.0.to_string(),
                }),
            );
            return;
        }
        // commit the trade now that the coin transfer succeeded. a completed
        // transfer cannot be clawed back, so the request-time slippage guard is
        // the bound. if the current price moved more than 2 percent, record it
        // and still account for the actual tokens moved.
        let now_out = self.curve_coins_out(net.0);
        if now_out.abs_diff(coins_out.0).saturating_mul(100) > coins_out.0.saturating_mul(2) {
            Self::emit_event(
                "curve_buy_price_moved",
                near_sdk::serde_json::json!({
                    "account": buyer,
                    "sent": coins_out.0.to_string(),
                    "current": now_out.to_string(),
                }),
            );
        }
        self.curve_quote_reserve = self.curve_quote_reserve.saturating_add(net.0);
        self.curve_coin_reserve = self.curve_coin_reserve.saturating_sub(coins_out.0);
        self.curve_total_sold = self.curve_total_sold.saturating_add(coins_out.0);
        self.creator_fees = self.creator_fees.saturating_add(creator_fee.0);
        self.protocol_fees = self.protocol_fees.saturating_add(protocol_fee.0);
        self.coin_balance_cached = self.coin_balance_cached.saturating_sub(coins_out.0);
        Self::emit_event(
            "curve_buy",
            near_sdk::serde_json::json!({
                "account": buyer,
                "quote_in": quote_in.0.to_string(),
                "coin_out": coins_out.0.to_string(),
                "creator_fee": creator_fee.0.to_string(),
                "protocol_fee": protocol_fee.0.to_string(),
            }),
        );
    }

    #[private]
    #[allow(clippy::too_many_arguments)]
    pub fn on_sell_result(
        &mut self,
        seller: AccountId,
        coin_in: U128,
        net: U128,
        protocol_fee: U128,
        creator_fee: U128,
        quote_out: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            // the coins are owed back to the seller, nothing else was mutated
            let cur = self.pending_coin_refunds.get(&seller).cloned().unwrap_or(0);
            self.pending_coin_refunds.insert(
                seller.clone(),
                cur.checked_add(coin_in.0).expect("coin refund overflow"),
            );
            self.pending_coin_refunds_total = self
                .pending_coin_refunds_total
                .checked_add(coin_in.0)
                .expect("coin refund total overflow");
            Self::emit_event(
                "curve_sell_reverted",
                near_sdk::serde_json::json!({
                    "account": seller,
                    "coin_in": coin_in.0.to_string(),
                }),
            );
            return;
        }
        let now_out = self.curve_quote_out(net.0);
        if now_out.abs_diff(quote_out.0).saturating_mul(100) > quote_out.0.saturating_mul(2) {
            Self::emit_event(
                "curve_sell_price_moved",
                near_sdk::serde_json::json!({
                    "account": seller,
                    "sent": quote_out.0.to_string(),
                    "current": now_out.to_string(),
                }),
            );
        }
        self.curve_coin_reserve = self.curve_coin_reserve.saturating_add(net.0);
        self.protocol_fees_coin = self.protocol_fees_coin.saturating_add(protocol_fee.0);
        self.creator_fees_coin = self.creator_fees_coin.saturating_add(creator_fee.0);
        self.curve_quote_reserve = self.curve_quote_reserve.saturating_sub(quote_out.0);
        self.curve_total_sold = self.curve_total_sold.saturating_sub(net.0);
        Self::emit_event(
            "curve_sell",
            near_sdk::serde_json::json!({
                "account": seller,
                "coin_in": coin_in.0.to_string(),
                "quote_out": quote_out.0.to_string(),
                "creator_fee": creator_fee.0.to_string(),
                "protocol_fee": protocol_fee.0.to_string(),
            }),
        );
    }

    #[payable]
    pub fn claim_coin_refund(&mut self) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas to claim_coin_refund", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let account = env::predecessor_account_id();
        let amount = self
            .pending_coin_refunds
            .get(&account)
            .cloned()
            .unwrap_or(0);
        require!(amount > 0, "no pending coin refund");
        self.pending_coin_refunds.insert(account.clone(), 0);
        self.pending_coin_refunds_total = self
            .pending_coin_refunds_total
            .checked_sub(amount)
            .expect("coin refund total underflow");
        self.coin_transfer(account.clone(), amount, "coin refund")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_coin_refund_result(account, U128(amount)),
            )
    }

    #[private]
    pub fn on_coin_refund_result(
        &mut self,
        account: AccountId,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            let cur = self
                .pending_coin_refunds
                .get(&account)
                .cloned()
                .unwrap_or(0);
            self.pending_coin_refunds.insert(
                account,
                cur.checked_add(amount.0).expect("restore overflow"),
            );
            self.pending_coin_refunds_total = self
                .pending_coin_refunds_total
                .checked_add(amount.0)
                .expect("restore overflow");
        } else {
            // the coins left the contract
            self.coin_balance_cached = self
                .coin_balance_cached
                .checked_sub(amount.0)
                .expect("coin balance underflow");
        }
    }

    #[payable]
    pub fn open_pack(&mut self, token_id: u64) -> Promise {
        require!(
            env::attached_deposit().as_yoctonear() == VRF_DEPOSIT,
            format!("attach exactly {} yocto for the VRF request", VRF_DEPOSIT)
        );
        let opener = env::predecessor_account_id();
        let pack = self.packs.get(&token_id).expect("pack not found").clone();
        require!(pack.owner == opener, "not pack owner");
        require!(pack.status != PackStatus::PendingSale, "sale pending");
        require!(pack.status == PackStatus::Unopened, "pack already opened");
        let draw_id = pack.draw_id;
        let mut draw = self.draws.get(&draw_id).expect("draw not found").clone();
        require!(
            draw.status == DrawStatus::Committed,
            "draw not in Committed state"
        );

        if !self.milestone_triggered && self.total_packs_opened == self.milestone_threshold {
            self.apply_milestone();
        }

        self.packs.insert(
            token_id,
            PackRecord {
                token_id,
                owner: opener.clone(),
                status: PackStatus::Opened,
                draw_id,
            },
        );
        self.remove_owner_token(&opener, token_id);
        let open_cnt = self.open_counts.get(&opener).cloned().unwrap_or(0) + 1;
        self.open_counts.insert(opener.clone(), open_cnt);
        self.update_top_opener(opener.clone(), open_cnt);

        draw.status = DrawStatus::VrfRequested;
        draw.commit_block = env::block_height();
        self.draws.insert(draw_id, draw);

        self.recent_draws.push(RecentEntry {
            draw_id,
            owner: opener.clone(),
            prize: None,
            block_height: env::block_height(),
        });
        if self.recent_draws.len() > 50 {
            self.recent_draws.remove(0);
        }
        self.total_packs_opened = self
            .total_packs_opened
            .checked_add(1)
            .expect("opened overflow");
        self.assert_coin_solvent();

        Self::emit_event(
            "pack_opened",
            near_sdk::serde_json::json!({
                "account": opener,
                "token_id": token_id,
                "draw_id": draw_id,
            }),
        );

        let args = near_sdk::serde_json::json!({
            "source": {"Project": {"project_id": self.vrf_project_id, "version_key": null}},
            "resource_limits": {"max_instructions": 100000000, "max_memory_mb": 100, "max_execution_seconds": 10},
            "input_data": near_sdk::serde_json::to_string(&near_sdk::serde_json::json!({"seed": draw_id.to_string(), "max": 9999, "count": 1})).expect("json"),
            "response_format": "Json"
        });

        Promise::new(self.outlayer_account.clone())
            .function_call(
                "request_execution".to_string(),
                near_sdk::serde_json::to_vec(&args).expect("json"),
                NearToken::from_yoctonear(VRF_DEPOSIT),
                Gas::from_tgas(VRF_GAS_REQUEST),
            )
            .then(
                Promise::new(env::current_account_id()).function_call(
                    "on_vrf_result".to_string(),
                    near_sdk::serde_json::to_vec(
                        &near_sdk::serde_json::json!({"token_id": token_id}),
                    )
                    .expect("json"),
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(VRF_GAS_CALLBACK),
                ),
            )
    }

    #[private]
    pub fn on_vrf_result(
        &mut self,
        token_id: u64,
        #[callback_result] result: Result<OutlayerEnvelope, near_sdk::PromiseError>,
    ) -> bool {
        require!(
            env::predecessor_account_id() == env::current_account_id(),
            "only self can call on_vrf_result"
        );
        let pack = match self.packs.get(&token_id).cloned() {
            Some(p) => p,
            None => return false,
        };
        let draw_id = pack.draw_id;
        let draw = match self.draws.get(&draw_id).cloned() {
            Some(d) => d,
            None => return false,
        };
        if draw.status != DrawStatus::VrfRequested {
            return false;
        }
        let envelope = match result {
            Ok(e) => e,
            Err(_) => return false,
        };
        let Some(output) = envelope.results.first().cloned() else {
            return false;
        };

        let alpha = output.alpha;
        let parts: Vec<&str> = alpha.split(':').collect();
        require!(parts.len() == 4, format!("invalid alpha format: {}", alpha));
        require!(parts[0] == "vrf", "alpha must start with vrf");
        require!(
            parts[2] == env::current_account_id().as_str(),
            format!(
                "alpha machine {} does not match {}",
                parts[2],
                env::current_account_id()
            )
        );
        let alpha_draw_id: u64 = parts[3]
            .parse()
            .unwrap_or_else(|_| env::panic_str("alpha draw id is not a number"));
        require!(
            alpha_draw_id == draw_id,
            format!("alpha draw {} mismatch {}", alpha_draw_id, draw_id)
        );
        let request_id = parts[1];
        require!(!request_id.is_empty(), "alpha request id empty");
        require!(
            self.vrf_request_ids.get(request_id).is_none(),
            "alpha request id already used"
        );
        self.vrf_request_ids.insert(request_id.to_string(), draw_id);
        let mut updated = draw.clone();
        updated.vrf_request_id = request_id.to_string();
        self.draws.insert(draw_id, updated);

        let sig_bytes = Self::hex_decode(&output.signature_hex);
        let pubkey_bytes = Self::hex_decode(&self.vrf_pubkey);
        if sig_bytes.len() != 64 || pubkey_bytes.len() != 32 {
            return false;
        }
        let sig: [u8; 64] = match sig_bytes.clone().try_into() {
            Ok(s) => s,
            Err(_) => return false,
        };
        let pubkey: [u8; 32] = match pubkey_bytes.try_into() {
            Ok(p) => p,
            Err(_) => return false,
        };
        if !env::ed25519_verify(&sig, alpha.as_bytes(), &pubkey) {
            return false;
        }

        let hash = env::sha256(&sig_bytes);
        let roll = (u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % 10000) as u64;
        let chosen = Self::choose_tier(&draw.table_snapshot, roll);
        let owner = draw.owner.clone();
        let multiplier = chosen.multiplier_bps;
        self.settle_draw_internal(draw_id, chosen, false);
        Self::emit_event(
            "draw_settled",
            near_sdk::serde_json::json!({
                "draw_id": draw_id,
                "account": owner,
                "multiplier_bps": multiplier,
            }),
        );
        true
    }

    pub fn settle_draw_fallback(&mut self, draw_id: u64) -> bool {
        let draw = self.draws.get(&draw_id).expect("draw not found").clone();
        require!(
            draw.status == DrawStatus::VrfRequested,
            "draw not VrfRequested"
        );
        let current = env::block_height();
        require!(
            current >= draw.commit_block + FALLBACK_DELAY_BLOCKS,
            format!(
                "too early: need {} blocks, have {}",
                FALLBACK_DELAY_BLOCKS,
                current - draw.commit_block
            )
        );
        let seed = env::random_seed();
        let roll = if seed.is_empty() {
            0u64
        } else {
            let hash = env::sha256(&seed);
            (u32::from_be_bytes([hash[0], hash[1], hash[2], hash[3]]) % 10000) as u64
        };
        let chosen = Self::choose_tier(&draw.table_snapshot, roll);
        let owner = draw.owner.clone();
        let multiplier = chosen.multiplier_bps;
        self.settle_draw_internal(draw_id, chosen, true);
        Self::emit_event(
            "draw_fallback",
            near_sdk::serde_json::json!({
                "draw_id": draw_id,
                "account": owner,
                "multiplier_bps": multiplier,
            }),
        );
        true
    }

    #[payable]
    pub fn claim(&mut self) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas to claim", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let account = env::predecessor_account_id();
        let amount = self.claimable.get(&account).cloned().unwrap_or(0);
        require!(amount > 0, "no claimable balance");
        // mark every settled draw covered by this claim so claim_draw cannot pay twice
        let mut marked: Vec<u64> = Vec::new();
        for id in self.owner_tokens.get(&account).cloned().unwrap_or_default() {
            if let Some(mut draw) = self.draws.get(&id).cloned() {
                if !draw.claimed && draw.prize.is_some() {
                    draw.claimed = true;
                    self.draws.insert(id, draw);
                    marked.push(id);
                }
            }
        }
        self.claimable.insert(account.clone(), 0);
        self.claimable_total = self
            .claimable_total
            .checked_sub(amount)
            .expect("claimable underflow");
        self.prize_pool = self
            .prize_pool
            .checked_sub(amount)
            .expect("prize pool underflow");
        self.coin_balance_cached = self
            .coin_balance_cached
            .checked_sub(amount)
            .expect("coin balance underflow");
        self.assert_coin_solvent();

        self.coin_transfer(account.clone(), amount, "prize claim")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_claim_result(account, U128(amount), marked),
            )
    }

    #[payable]
    pub fn claim_draw(&mut self, draw_id: u64) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas to claim_draw", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let account = env::predecessor_account_id();
        let draw = self.draws.get(&draw_id).expect("draw not found").clone();
        require!(draw.owner == account, "not draw owner");
        require!(!draw.claimed, "draw already claimed");
        let prize = draw.prize.as_ref().map(|p| p.prize_amount.0).unwrap_or(0);
        require!(prize > 0, "draw not settled");
        let mut claimed = draw.clone();
        claimed.claimed = true;
        self.draws.insert(draw_id, claimed);
        let cur = self.claimable.get(&account).cloned().unwrap_or(0);
        require!(cur >= prize, "claimable below draw prize");
        self.claimable.insert(account.clone(), cur - prize);
        self.claimable_total = self
            .claimable_total
            .checked_sub(prize)
            .expect("claimable underflow");
        self.prize_pool = self
            .prize_pool
            .checked_sub(prize)
            .expect("prize pool underflow");
        self.coin_balance_cached = self
            .coin_balance_cached
            .checked_sub(prize)
            .expect("coin balance underflow");
        self.assert_coin_solvent();
        self.coin_transfer(account.clone(), prize, "draw claim")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_claim_draw_result(draw_id, account, U128(prize)),
            )
    }

    #[private]
    pub fn on_claim_draw_result(
        &mut self,
        draw_id: u64,
        account: AccountId,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            if let Some(mut draw) = self.draws.get(&draw_id).cloned() {
                draw.claimed = false;
                self.draws.insert(draw_id, draw);
            }
            let cur = self.claimable.get(&account).cloned().unwrap_or(0);
            self.claimable.insert(
                account.clone(),
                cur.checked_add(amount.0).expect("restore overflow"),
            );
            self.claimable_total = self
                .claimable_total
                .checked_add(amount.0)
                .expect("restore overflow");
            self.prize_pool = self
                .prize_pool
                .checked_add(amount.0)
                .expect("restore overflow");
            self.coin_balance_cached = self
                .coin_balance_cached
                .checked_add(amount.0)
                .expect("restore overflow");
        } else {
            Self::emit_event(
                "draw_claimed",
                near_sdk::serde_json::json!({
                    "draw_id": draw_id,
                    "account": account,
                    "amount": amount.0.to_string(),
                }),
            );
        }
    }

    #[private]
    pub fn on_claim_result(
        &mut self,
        account: AccountId,
        amount: U128,
        marked: Vec<u64>,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            for id in marked {
                if let Some(mut draw) = self.draws.get(&id).cloned() {
                    draw.claimed = false;
                    self.draws.insert(id, draw);
                }
            }
            let cur = self.claimable.get(&account).cloned().unwrap_or(0);
            self.claimable.insert(
                account,
                cur.checked_add(amount.0).expect("restore overflow"),
            );
            self.claimable_total = self
                .claimable_total
                .checked_add(amount.0)
                .expect("restore overflow");
            self.prize_pool = self
                .prize_pool
                .checked_add(amount.0)
                .expect("restore overflow");
            self.coin_balance_cached = self
                .coin_balance_cached
                .checked_add(amount.0)
                .expect("restore overflow");
        } else {
            Self::emit_event(
                "prize_claimed",
                near_sdk::serde_json::json!({
                    "account": account,
                    "amount": amount.0.to_string(),
                }),
            );
        }
    }

    pub fn pause(&mut self) {
        require!(
            env::predecessor_account_id() == self.owner,
            "only owner can pause"
        );
        require!(!self.paused, "already paused");
        self.paused = true;
    }

    pub fn unpause(&mut self) {
        require!(
            env::predecessor_account_id() == self.owner,
            "only owner can unpause"
        );
        require!(self.paused, "not paused");
        self.paused = false;
    }

    pub fn list_pack(&mut self, token_id: u64, price: String) {
        self.assert_not_paused();
        let seller = env::predecessor_account_id();
        let pack = self.packs.get(&token_id).expect("pack not found").clone();
        require!(pack.owner == seller, "not pack owner");
        require!(pack.status != PackStatus::PendingSale, "sale pending");
        require!(pack.status == PackStatus::Unopened, "pack already opened");
        require!(self.listings.get(&token_id).is_none(), "already listed");
        let price_amt: u128 = price.parse().expect("invalid price");
        require!(
            price_amt >= MIN_PACK_PRICE,
            format!("price below minimum {}", MIN_PACK_PRICE)
        );
        let mut listed = pack.clone();
        listed.status = PackStatus::Listed;
        self.packs.insert(token_id, listed);
        self.listings.insert(
            token_id,
            PackListing {
                seller: seller.clone(),
                price: U128(price_amt),
                token_id,
            },
        );
        Self::emit_event(
            "pack_listed",
            near_sdk::serde_json::json!({
                "account": seller,
                "token_id": token_id,
                "price": price_amt.to_string(),
            }),
        );
    }

    pub fn unlist_pack(&mut self, token_id: u64) {
        let caller = env::predecessor_account_id();
        let listing = self.listings.get(&token_id).expect("not listed").clone();
        require!(listing.seller == caller, "not seller");
        let mut pack = self.packs.get(&token_id).expect("pack not found").clone();
        require!(pack.status == PackStatus::Listed, "not listed state");
        pack.status = PackStatus::Unopened;
        self.packs.insert(token_id, pack);
        self.listings.remove(&token_id);
    }

    #[private]
    pub fn on_seller_paid(
        &mut self,
        buyer: AccountId,
        seller: AccountId,
        token_id: u64,
        price: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        let mut pack = match self.packs.get(&token_id).cloned() {
            Some(p) => p,
            None => return,
        };
        let fee = fees::fee_amount(price.0, RESALE_FEE_BPS);
        if result.is_ok() {
            if pack.owner != seller || pack.status != PackStatus::PendingSale {
                return;
            }
            pack.owner = buyer.clone();
            pack.status = PackStatus::Unopened;
            self.packs.insert(token_id, pack.clone());
            if let Some(mut draw) = self.draws.get(&pack.draw_id).cloned() {
                draw.owner = buyer.clone();
                self.draws.insert(pack.draw_id, draw);
            }
            self.pending_protocol_fees = self.pending_protocol_fees.saturating_sub(fee);
            self.protocol_fees = self.protocol_fees.checked_add(fee).expect("fee overflow");
            self.listings.remove(&token_id);
            self.remove_owner_token(&seller, token_id);
            self.add_owner_token(&buyer, token_id);
            Self::emit_event(
                "pack_sold",
                near_sdk::serde_json::json!({
                    "token_id": token_id,
                    "seller": seller,
                    "buyer": buyer,
                    "price": price.0.to_string(),
                }),
            );
            return;
        }
        pack.status = PackStatus::Listed;
        self.packs.insert(token_id, pack);
        self.pending_protocol_fees = self.pending_protocol_fees.saturating_sub(fee);
        let cur = self.pending_refunds.get(&buyer).cloned().unwrap_or(0);
        self.pending_refunds.insert(
            buyer.clone(),
            cur.checked_add(price.0).expect("refund overflow"),
        );
        Self::emit_event(
            "pack_sale_reverted",
            near_sdk::serde_json::json!({
                "token_id": token_id,
                "buyer": buyer,
                "seller": seller,
            }),
        );
    }

    #[payable]
    pub fn withdraw_refund(&mut self) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas to withdraw_refund", required)
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let account = env::predecessor_account_id();
        let amount = self.pending_refunds.get(&account).cloned().unwrap_or(0);
        require!(amount > 0, "no pending refund");
        self.pending_refunds.insert(account.clone(), 0);
        self.quote_transfer(account.clone(), amount, "refund").then(
            Self::ext(env::current_account_id())
                .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                .on_refund_result(account, U128(amount)),
        )
    }

    #[private]
    pub fn on_refund_result(
        &mut self,
        account: AccountId,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            let cur = self.pending_refunds.get(&account).cloned().unwrap_or(0);
            self.pending_refunds.insert(
                account,
                cur.checked_add(amount.0).expect("restore overflow"),
            );
        }
    }

    #[payable]
    pub fn nft_transfer(&mut self, receiver_id: AccountId, token_id: String, memo: Option<String>) {
        let _ = memo;
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let predecessor = env::predecessor_account_id();
        let id: u64 = token_id.parse().expect("invalid token_id");
        let mut pack = self.packs.get(&id).expect("pack not found").clone();
        require!(pack.owner == predecessor, "not owner");
        require!(pack.status != PackStatus::PendingSale, "sale pending");
        require!(
            pack.status == PackStatus::Unopened,
            "only unopened packs transferable"
        );
        require!(
            self.listings.get(&id).is_none(),
            "pack is listed, unlist first"
        );
        pack.owner = receiver_id.clone();
        self.packs.insert(id, pack.clone());
        self.remove_owner_token(&predecessor, id);
        self.add_owner_token(&receiver_id, id);
        if let Some(mut draw) = self.draws.get(&pack.draw_id).cloned() {
            draw.owner = receiver_id;
            self.draws.insert(pack.draw_id, draw);
        }
    }

    pub fn redeem_scrap(&mut self) {
        let account = env::predecessor_account_id();
        let credits = self.scrap_credit.get(&account).cloned().unwrap_or(0);
        require!(
            credits >= 3,
            format!("not enough scrap credits: have {} need 3", credits)
        );
        let reserve = self.scrap_discount_amount;
        require!(
            self.protocol_fees >= reserve,
            "not enough protocol fees to reserve discount"
        );
        self.protocol_fees -= reserve;
        let cur = self.reserved_discounts.get(&account).cloned().unwrap_or(0);
        self.reserved_discounts.insert(
            account.clone(),
            cur.checked_add(reserve)
                .expect("reserved discount overflow"),
        );
        self.scrap_credit.insert(account.clone(), credits - 3);
        self.scrap_discount_next.insert(account, true);
    }

    pub fn megapack_claim(&mut self) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::Graduated,
            "not graduated"
        );
        let account = env::predecessor_account_id();
        let start = self
            .megapack_eligible_from_block
            .get(&account)
            .cloned()
            .unwrap_or(0);
        require!(
            self.megapack_holders.get(&account).is_some(),
            "not a megapack holder"
        );
        require!(
            env::block_height() >= start + MEGAPACK_HOLD_BLOCKS,
            format!(
                "need {} blocks of holding, have {}",
                MEGAPACK_HOLD_BLOCKS,
                env::block_height() - start
            )
        );
        let last = self
            .megapack_last_claim_block
            .get(&account)
            .cloned()
            .unwrap_or(0);
        require!(
            env::block_height() >= last + MEGAPACK_CLAIM_COOLDOWN,
            format!("megapack cooldown: wait {} blocks", MEGAPACK_CLAIM_COOLDOWN)
        );
        let pot = self.megapack_pot;
        require!(pot > 0, "megapack pot empty");
        let eligible = self.megapack_holder_count as u128;
        require!(eligible > 0, "no eligible holders");
        let share = (pot / eligible).max(1).min(pot);
        self.megapack_pot = self.megapack_pot.checked_sub(share).expect("pot underflow");
        self.megapack_distributed_total = self
            .megapack_distributed_total
            .checked_add(share)
            .expect("distributed overflow");
        self.megapack_last_claim_block
            .insert(account.clone(), env::block_height());
        self.quote_transfer(account.clone(), share, "megapack share")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_megapack_claim_result(U128(share)),
            )
    }

    #[private]
    pub fn on_megapack_claim_result(
        &mut self,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            self.megapack_pot = self
                .megapack_pot
                .checked_add(amount.0)
                .expect("restore overflow");
            self.megapack_distributed_total = self
                .megapack_distributed_total
                .checked_sub(amount.0)
                .expect("restore underflow");
        }
    }

    #[payable]
    pub fn withdraw_protocol_fees(&mut self, amount: String) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!(
                "attach at least {} Tgas to withdraw_protocol_fees",
                required
            )
        );
        require!(
            env::predecessor_account_id() == self.treasury,
            "only treasury"
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let amt: u128 = amount.parse().expect("invalid amount");
        require!(amt > 0, "amount must be >0");
        require!(
            amt <= self.protocol_fees,
            format!(
                "amount {} exceeds protocol fees {}",
                amt, self.protocol_fees
            )
        );
        self.protocol_fees -= amt;
        self.quote_transfer(self.treasury.clone(), amt, "protocol fees")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_withdraw_protocol_result(U128(amt)),
            )
    }

    #[private]
    pub fn on_withdraw_protocol_result(
        &mut self,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            self.protocol_fees = self
                .protocol_fees
                .checked_add(amount.0)
                .expect("restore overflow");
        }
    }

    #[payable]
    pub fn withdraw_creator_fees(&mut self, amount: String) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!("attach at least {} Tgas to withdraw_creator_fees", required)
        );
        require!(env::predecessor_account_id() == self.owner, "only owner");
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let amt: u128 = amount.parse().expect("invalid amount");
        require!(amt > 0, "amount must be >0");
        require!(
            amt <= self.creator_fees,
            format!("amount {} exceeds creator fees {}", amt, self.creator_fees)
        );
        self.creator_fees -= amt;
        self.quote_transfer(self.owner.clone(), amt, "creator fees")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_withdraw_creator_result(U128(amt)),
            )
    }

    #[private]
    pub fn on_withdraw_creator_result(
        &mut self,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            self.creator_fees = self
                .creator_fees
                .checked_add(amount.0)
                .expect("restore overflow");
        }
    }

    #[payable]
    pub fn withdraw_protocol_fees_coin(&mut self, amount: String) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!(
                "attach at least {} Tgas to withdraw_protocol_fees_coin",
                required
            )
        );
        require!(
            env::predecessor_account_id() == self.treasury,
            "only treasury"
        );
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let amt: u128 = amount.parse().expect("invalid amount");
        require!(amt > 0, "amount must be >0");
        require!(
            amt <= self.protocol_fees_coin,
            format!(
                "amount {} exceeds protocol coin fees {}",
                amt, self.protocol_fees_coin
            )
        );
        self.protocol_fees_coin -= amt;
        self.coin_balance_cached = self
            .coin_balance_cached
            .checked_sub(amt)
            .expect("coin balance underflow");
        self.assert_coin_solvent();
        self.coin_transfer(self.treasury.clone(), amt, "protocol coin fees")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_withdraw_protocol_coin_result(U128(amt)),
            )
    }

    #[private]
    pub fn on_withdraw_protocol_coin_result(
        &mut self,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            self.protocol_fees_coin = self
                .protocol_fees_coin
                .checked_add(amount.0)
                .expect("restore overflow");
            self.coin_balance_cached = self
                .coin_balance_cached
                .checked_add(amount.0)
                .expect("restore overflow");
        }
    }

    #[payable]
    pub fn withdraw_creator_fees_coin(&mut self, amount: String) -> Promise {
        let required = self
            .ft_gas
            .checked_add(CALLBACK_GAS + GAS_HEADROOM)
            .expect("gas overflow");
        require!(
            env::prepaid_gas() >= Gas::from_tgas(required),
            format!(
                "attach at least {} Tgas to withdraw_creator_fees_coin",
                required
            )
        );
        require!(env::predecessor_account_id() == self.owner, "only owner");
        require!(
            env::attached_deposit().as_yoctonear() == 1,
            "attach exactly 1 yocto"
        );
        let amt: u128 = amount.parse().expect("invalid amount");
        require!(amt > 0, "amount must be >0");
        require!(
            amt <= self.creator_fees_coin,
            format!(
                "amount {} exceeds creator coin fees {}",
                amt, self.creator_fees_coin
            )
        );
        self.creator_fees_coin -= amt;
        self.coin_balance_cached = self
            .coin_balance_cached
            .checked_sub(amt)
            .expect("coin balance underflow");
        self.assert_coin_solvent();
        self.coin_transfer(self.owner.clone(), amt, "creator coin fees")
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(CALLBACK_GAS))
                    .on_withdraw_creator_coin_result(U128(amt)),
            )
    }

    #[private]
    pub fn on_withdraw_creator_coin_result(
        &mut self,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            self.creator_fees_coin = self
                .creator_fees_coin
                .checked_add(amount.0)
                .expect("restore overflow");
            self.coin_balance_cached = self
                .coin_balance_cached
                .checked_add(amount.0)
                .expect("restore overflow");
        }
    }

    /// Refreshes the cached coin balance from the coin contract.
    pub fn sync_coin_balance(&mut self) -> Promise {
        Promise::new(self.coin_token.clone())
            .function_call(
                "ft_balance_of".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account_id": env::current_account_id(),
                }))
                .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(30),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(50))
                    .on_coin_balance(),
            )
    }

    #[private]
    pub fn on_coin_balance(
        &mut self,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) {
        if let Ok(balance) = result {
            // exact sync: a shortfall must surface on the next state change
            self.coin_balance_cached = balance.0;
        }
    }

    pub fn snapshot_candle(&mut self) {
        let quote = self.curve_quote_reserve;
        let coin = self.curve_coin_reserve;
        let price = if coin == 0 {
            0u128
        } else {
            quote
                .checked_mul(1_000_000)
                .expect("price overflow")
                .checked_div(coin)
                .unwrap_or(0)
        };
        let timestamp = env::block_height();
        if let Some(last) = self.candles.last_mut() {
            if timestamp < last.timestamp + 100 {
                let high = last.high.0;
                let low = last.low.0;
                if price > high {
                    last.high = U128(price);
                }
                if price < low {
                    last.low = U128(price);
                }
                last.close = U128(price);
                return;
            }
        }
        self.candles.push(Candle {
            open: U128(price),
            high: U128(price),
            low: U128(price),
            close: U128(price),
            timestamp,
        });
        if self.candles.len() > 288 {
            self.candles.remove(0);
        }
    }

    pub fn graduate(&mut self) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::CurveActive,
            "not in curve phase"
        );
        require!(self.curve_coin_reserve > 0, "no coin reserve");
        let market_cap = self
            .curve_quote_reserve
            .checked_mul(COIN_TOTAL_SUPPLY)
            .expect("market cap overflow")
            / self.curve_coin_reserve;
        require!(
            market_cap >= self.graduation_target,
            format!(
                "market cap {} below target {}",
                market_cap, self.graduation_target
            )
        );
        // fee coins stay in the machine for later withdrawal, do not seed them
        let coin_amount = self
            .coin_balance_cached
            .checked_sub(self.prize_pool)
            .expect("coin balance below prize pool")
            .checked_sub(self.protocol_fees_coin)
            .expect("coin balance below protocol fees")
            .checked_sub(self.creator_fees_coin)
            .expect("coin balance below creator fees")
            .checked_sub(self.pending_coin_refunds_total)
            .expect("coin balance below pending refunds");
        require!(coin_amount > 0, "no coins to seed");
        let quote_amount = self.curve_quote_reserve;
        require!(quote_amount > 0, "no quote to seed");
        self.graduation_status = GraduationStatus::GraduationPending;
        self.graduation_block = Some(env::block_height());
        self.graduation_coin_amount = coin_amount;
        self.graduation_quote_amount = quote_amount;
        self.graduation_coin_seeded = false;
        self.graduation_phase = 0;
        self.graduation_progress_block = env::block_height();
        self.stranded_coin = 0;
        self.stranded_quote = 0;
        Self::emit_event(
            "graduation_started",
            near_sdk::serde_json::json!({
                "coin_amount": coin_amount.to_string(),
                "quote_amount": quote_amount.to_string(),
                "market_cap": market_cap.to_string(),
            }),
        );
        Promise::new(self.rhea.clone())
            .function_call(
                "storage_deposit".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account_id": env::current_account_id(),
                    "registration_only": true,
                }))
                .expect("json"),
                RHEA_STORAGE_DEPOSIT,
                Gas::from_tgas(RHEA_CALL_GAS),
            )
            .then(
                Promise::new(self.rhea.clone()).function_call(
                    "get_pools".to_string(),
                    near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                        "from_index": 0,
                        "limit": 100,
                    }))
                    .expect("json"),
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(RHEA_VIEW_GAS),
                ),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(130))
                    .on_pools_checked(0, 2),
            )
    }

    #[private]
    pub fn on_pools_checked(
        &mut self,
        offset: u64,
        pages_left: u8,
        #[callback_result] result: Result<Vec<RheaPoolInfo>, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        let pools = match result {
            Ok(p) => p,
            Err(_) => {
                self.fail_graduation("get_pools failed");
                return Promise::new(env::current_account_id());
            }
        };
        let coin = self.coin_token.clone();
        let quote = self.quote_token.clone();
        let found = pools.iter().find(|p| {
            p.token_account_ids.len() == 2
                && ((p.token_account_ids[0] == coin && p.token_account_ids[1] == quote)
                    || (p.token_account_ids[0] == quote && p.token_account_ids[1] == coin))
        });
        if let Some(pool) = found {
            let (pool_coin, pool_quote) = if pool.token_account_ids[0] == coin {
                (pool.amounts[0].0, pool.amounts[1].0)
            } else {
                (pool.amounts[1].0, pool.amounts[0].0)
            };
            if pool_coin == 0 || pool_quote == 0 {
                self.fail_graduation("existing pool is empty");
                return Promise::new(env::current_account_id());
            }
            let lhs = pool_quote
                .checked_mul(self.curve_coin_reserve)
                .expect("price overflow");
            let rhs = self
                .curve_quote_reserve
                .checked_mul(pool_coin)
                .expect("price overflow");
            let upper = rhs
                .checked_mul(10_000 + GRADUATION_SLIPPAGE_BPS)
                .expect("price overflow");
            let lower = rhs
                .checked_mul(10_000 - GRADUATION_SLIPPAGE_BPS)
                .expect("price overflow");
            let scaled = lhs.checked_mul(10_000).expect("price overflow");
            if scaled > upper || scaled < lower {
                self.fail_graduation("existing pool price is skewed");
                return Promise::new(env::current_account_id());
            }
            self.graduation_pool_id = Some(pool.pool_id);
            self.graduation_coin_first = pool.token_account_ids[0] == coin;
            self.rhea_pool_exists = true;
            self.graduation_progress_block = env::block_height();
            Self::emit_event(
                "graduation_pool_ready",
                near_sdk::serde_json::json!({
                    "pool_id": pool.pool_id,
                    "created": false,
                }),
            );
            return Promise::new(env::current_account_id());
        }
        if pools.len() == 100 && pages_left > 0 {
            let next_offset = offset + 100;
            let next_gas = if pages_left >= 2 { 100 } else { 70 };
            return Promise::new(self.rhea.clone())
                .function_call(
                    "get_pools".to_string(),
                    near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                        "from_index": next_offset,
                        "limit": 100,
                    }))
                    .expect("json"),
                    NearToken::from_yoctonear(0),
                    Gas::from_tgas(RHEA_VIEW_GAS),
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(next_gas))
                        .on_pools_checked(next_offset, pages_left - 1),
                );
        }
        Promise::new(self.rhea.clone())
            .function_call(
                "add_simple_pool".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "tokens": [coin, quote],
                    "fee": RHEA_POOL_FEE,
                }))
                .expect("json"),
                RHEA_STORAGE_DEPOSIT,
                Gas::from_tgas(RHEA_CALL_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(GRADUATION_CB))
                    .on_pool_created(),
            )
    }

    #[private]
    pub fn on_pool_created(
        &mut self,
        #[callback_result] result: Result<u64, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        match result {
            Ok(pool_id) => {
                self.graduation_pool_id = Some(pool_id);
                self.rhea_pool_exists = true;
                self.graduation_progress_block = env::block_height();
                Self::emit_event(
                    "graduation_pool_ready",
                    near_sdk::serde_json::json!({
                        "pool_id": pool_id,
                        "created": true,
                    }),
                );
            }
            Err(_) => self.fail_graduation("add_simple_pool failed"),
        }
        Promise::new(env::current_account_id())
    }

    /// Registers rhea with the coin and deposits the coin side. Permissionless.
    pub fn seed_graduation(&mut self) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_pool_id.is_some(), "pool not ready");
        require!(self.graduation_phase == 0, "graduation step already active");
        require!(!self.graduation_coin_seeded, "coin already seeded");
        require!(
            self.stranded_coin == 0 && self.stranded_quote == 0,
            "stranded funds must be recovered first"
        );
        self.graduation_phase = 1;
        self.graduation_progress_block = env::block_height();
        let rhea = self.rhea.clone();
        Promise::new(self.coin_token.clone())
            .function_call(
                "storage_deposit".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account_id": rhea,
                    "registration_only": true,
                }))
                .expect("json"),
                NearToken::from_millinear(10),
                Gas::from_tgas(RHEA_CALL_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(220))
                    .on_coin_registered(),
            )
    }

    #[private]
    pub fn on_coin_registered(
        &mut self,
        #[callback_result] result: Result<near_sdk::serde_json::Value, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_phase == 1, "coin seed not active");
        if result.is_err() {
            self.fail_graduation("coin storage_deposit failed");
            self.graduation_phase = 0;
            return Promise::new(env::current_account_id());
        }
        self.deposit_coin()
    }

    fn deposit_coin(&mut self) -> Promise {
        let rhea = self.rhea.clone();
        let amount = self.graduation_coin_amount;
        Promise::new(self.coin_token.clone())
            .function_call(
                "ft_transfer_call".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "receiver_id": rhea,
                    "amount": amount.to_string(),
                    "msg": "",
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(DEPOSIT_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(GRADUATION_CB))
                    .on_coin_deposited(),
            )
    }

    #[private]
    pub fn on_coin_deposited(
        &mut self,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_phase == 1, "coin seed not active");
        match result {
            Ok(used) if used.0 == self.graduation_coin_amount => {
                self.graduation_coin_seeded = true;
                self.graduation_phase = 2;
                self.stranded_coin = self.graduation_coin_amount;
                self.graduation_progress_block = env::block_height();
                Self::emit_event(
                    "graduation_coin_seeded",
                    near_sdk::serde_json::json!({
                        "amount": self.graduation_coin_amount.to_string(),
                    }),
                );
            }
            Ok(used) => {
                if used.0 > 0 {
                    self.stranded_coin = used.0;
                }
                self.graduation_phase = 0;
                self.fail_graduation(&format!(
                    "coin deposit used {} of {}",
                    used.0, self.graduation_coin_amount
                ));
            }
            Err(_) => {
                self.graduation_phase = 0;
                self.fail_graduation("coin deposit error");
            }
        }
        Promise::new(env::current_account_id())
    }

    /// Verifies the quote backing, then deposits the quote side. Permissionless.
    pub fn seed_quote_graduation(&mut self) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_coin_seeded, "coin not seeded");
        require!(self.graduation_phase == 2, "quote seed not ready");
        self.graduation_phase = 3;
        self.graduation_progress_block = env::block_height();
        Promise::new(self.quote_token.clone())
            .function_call(
                "ft_balance_of".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "account_id": env::current_account_id(),
                }))
                .expect("json"),
                NearToken::from_yoctonear(0),
                Gas::from_tgas(RHEA_VIEW_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(260))
                    .on_quote_balance_checked(),
            )
    }

    #[private]
    pub fn on_quote_balance_checked(
        &mut self,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_phase == 3, "quote seed not active");
        let balance = match result {
            Ok(b) => b.0,
            Err(_) => {
                self.graduation_phase = 2;
                self.fail_graduation("quote balance check failed");
                return Promise::new(env::current_account_id());
            }
        };
        if balance < self.graduation_quote_amount {
            self.graduation_phase = 2;
            self.fail_graduation("insufficient quote backing");
            return Promise::new(env::current_account_id());
        }
        let rhea = self.rhea.clone();
        let amount = self.graduation_quote_amount;
        Promise::new(self.quote_token.clone())
            .function_call(
                "ft_transfer_call".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "receiver_id": rhea,
                    "amount": amount.to_string(),
                    "msg": "",
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(DEPOSIT_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(70))
                    .on_quote_deposited(),
            )
    }

    #[private]
    pub fn on_quote_deposited(
        &mut self,
        #[callback_result] result: Result<U128, near_sdk::PromiseError>,
    ) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_phase == 3, "quote seed not active");
        let expected = self.graduation_quote_amount;
        match result {
            Ok(used) if used.0.abs_diff(expected) <= 1 => {
                self.stranded_quote = used.0;
                self.graduation_progress_block = env::block_height();
                self.finish_graduation()
            }
            Ok(used) => {
                if used.0 > 0 {
                    self.stranded_quote = used.0;
                }
                self.fail_graduation(&format!("quote deposit used {} of {}", used.0, expected));
                Promise::new(env::current_account_id())
            }
            Err(_) => {
                self.fail_graduation("quote deposit error");
                Promise::new(env::current_account_id())
            }
        }
    }

    /// Adds liquidity once both sides are seeded. Permissionless.
    pub fn finish_graduation(&mut self) -> Promise {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(self.graduation_phase == 3, "liquidity step not ready");
        self.add_liquidity()
    }

    fn add_liquidity(&mut self) -> Promise {
        let pool_id = self.graduation_pool_id.expect("pool id");
        let coin = self.graduation_coin_amount;
        let quote = self.graduation_quote_amount;
        let amounts = if self.graduation_coin_first {
            vec![coin.to_string(), quote.to_string()]
        } else {
            vec![quote.to_string(), coin.to_string()]
        };
        Promise::new(self.rhea.clone())
            .function_call(
                "add_liquidity".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "pool_id": pool_id,
                    "amounts": amounts,
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(RHEA_CALL_GAS),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(GRADUATION_CB))
                    .on_liquidity_added(pool_id),
            )
    }

    #[private]
    pub fn on_liquidity_added(
        &mut self,
        pool_id: u64,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        if result.is_err() {
            self.fail_graduation("add_liquidity failed");
            return;
        }
        self.curve_coin_reserve = 0;
        self.curve_quote_reserve = 0;
        // the seed leaves the machine, the prize pool and fee coins stay
        self.coin_balance_cached = self
            .prize_pool
            .checked_add(self.protocol_fees_coin)
            .expect("cache overflow")
            .checked_add(self.creator_fees_coin)
            .expect("cache overflow")
            .checked_add(self.pending_coin_refunds_total)
            .expect("cache overflow");
        self.graduation_phase = 0;
        self.graduation_status = GraduationStatus::Graduated;
        self.graduation_pool_id = Some(pool_id);
        self.rhea_pool_exists = true;
        self.lp_locked = true;
        self.largest_pack_unlocked = true;
        self.stranded_coin = 0;
        self.stranded_quote = 0;
        self.graduation_progress_block = env::block_height();
        Self::emit_event(
            "graduated",
            near_sdk::serde_json::json!({
                "pool_id": pool_id,
                "coin_amount": self.graduation_coin_amount.to_string(),
                "quote_amount": self.graduation_quote_amount.to_string(),
            }),
        );
    }

    fn fail_graduation(&mut self, reason: &str) {
        self.graduation_error = reason.to_string();
        Self::emit_event(
            "graduation_failed",
            near_sdk::serde_json::json!({
                "reason": reason,
            }),
        );
    }

    /// Owner-only recovery of coins and quote stranded in the Rhea deposit.
    pub fn recover_stranded_graduation_funds(&mut self) -> Promise {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(
            self.stranded_coin > 0 || self.stranded_quote > 0,
            "no stranded funds"
        );
        let rhea = self.rhea.clone();
        if self.stranded_quote > 0 {
            let amount = self.stranded_quote;
            Promise::new(rhea.clone())
                .function_call(
                    "withdraw".to_string(),
                    near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                        "token_id": self.quote_token,
                        "amount": amount.to_string(),
                    }))
                    .expect("json"),
                    NearToken::from_yoctonear(1),
                    Gas::from_tgas(50),
                )
                .then(
                    Self::ext(env::current_account_id())
                        .with_static_gas(Gas::from_tgas(GRADUATION_CB))
                        .on_stranded_withdraw_result(false, U128(amount)),
                )
                .detach();
        }
        let amount = self.stranded_coin;
        Promise::new(rhea)
            .function_call(
                "withdraw".to_string(),
                near_sdk::serde_json::to_vec(&near_sdk::serde_json::json!({
                    "token_id": self.coin_token,
                    "amount": amount.to_string(),
                }))
                .expect("json"),
                NearToken::from_yoctonear(1),
                Gas::from_tgas(50),
            )
            .then(
                Self::ext(env::current_account_id())
                    .with_static_gas(Gas::from_tgas(GRADUATION_CB))
                    .on_stranded_withdraw_result(true, U128(amount)),
            )
    }

    #[private]
    pub fn on_stranded_withdraw_result(
        &mut self,
        is_coin: bool,
        amount: U128,
        #[callback_result] result: Result<(), near_sdk::PromiseError>,
    ) {
        if result.is_err() {
            return;
        }
        if is_coin {
            self.stranded_coin = self.stranded_coin.saturating_sub(amount.0);
        } else {
            self.stranded_quote = self.stranded_quote.saturating_sub(amount.0);
        }
        Self::emit_event(
            "stranded_recovered",
            near_sdk::serde_json::json!({
                "is_coin": is_coin,
                "amount": amount.0.to_string(),
            }),
        );
    }

    /// Permissionless abort after 10,000 blocks without progress.
    pub fn abort_graduation(&mut self) {
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(
            env::block_height() > self.graduation_progress_block + 10_000,
            "graduation timeout not reached"
        );
        require!(
            self.stranded_coin == 0 && self.stranded_quote == 0,
            "funds stranded, use recovery"
        );
        self.graduation_status = GraduationStatus::CurveActive;
        self.graduation_phase = 0;
        self.graduation_block = None;
        self.graduation_pool_id = None;
        self.graduation_coin_seeded = false;
        Self::emit_event(
            "graduation_aborted",
            near_sdk::serde_json::json!({
                "block": env::block_height(),
            }),
        );
    }

    pub fn recover_from_pending(&mut self) {
        require!(env::predecessor_account_id() == self.owner, "only owner");
        require!(
            self.graduation_status == GraduationStatus::GraduationPending,
            "not pending"
        );
        require!(
            self.stranded_coin == 0 && self.stranded_quote == 0,
            "funds stranded, use recovery"
        );
        self.graduation_status = GraduationStatus::CurveActive;
        self.graduation_block = None;
        self.graduation_phase = 0;
    }

    pub fn get_machine(&self) -> MachineView {
        MachineView {
            owner: self.owner.clone(),
            coin: self.coin.clone(),
            coin_token: self.coin_token.clone(),
            quote_token: self.quote_token.clone(),
            table_hash: self.table_hash.clone(),
            pack_art_url: self.pack_art_url.clone(),
            prize_table: self.prize_table.clone(),
            vrf_pubkey: self.vrf_pubkey.clone(),
            prize_pool: U128(self.prize_pool),
            prize_reserved: U128(self.prize_reserved),
            claimable_total: U128(self.claimable_total),
            curve_coin_reserve: U128(self.curve_coin_reserve),
            curve_quote_reserve: U128(self.curve_quote_reserve),
            paused: self.paused,
            total_packs_sold: self.total_packs_sold,
            total_packs_opened: self.total_packs_opened,
            megapack_pot: U128(self.megapack_pot),
            megapack_distributed_total: U128(self.megapack_distributed_total),
            treasury: self.treasury.clone(),
            graduation_pool_id: self.graduation_pool_id,
        }
    }

    pub fn get_prize_pool(&self) -> PrizePoolView {
        PrizePoolView {
            prize_pool: U128(self.prize_pool),
            prize_reserved: U128(self.prize_reserved),
            claimable_total: U128(self.claimable_total),
            coin_token: self.coin_token.clone(),
        }
    }

    pub fn get_table_hash(&self) -> String {
        self.table_hash.clone()
    }

    pub fn get_fan_coin_funding_status(&self) -> near_sdk::serde_json::Value {
        let required = self
            .curve_coin_allocation
            .saturating_add(self.prize_coin_allocation);
        let balance = self.coin_balance_cached;
        let shortfall = required.saturating_sub(balance);
        near_sdk::serde_json::json!({
            "required": U128(required),
            "balance": U128(balance),
            "shortfall": U128(shortfall),
            "funded": shortfall == 0,
        })
    }

    pub fn get_my_last_draw(&self, account: AccountId) -> Option<u64> {
        self.last_draw_by_owner.get(&account).cloned()
    }

    pub fn get_draws_by_owner(&self, account: AccountId) -> Vec<DrawView> {
        let ids = self.owner_tokens.get(&account).cloned().unwrap_or_default();
        ids.into_iter().filter_map(|id| self.get_draw(id)).collect()
    }

    pub fn get_draw(&self, draw_id: u64) -> Option<DrawView> {
        self.draws.get(&draw_id).map(|d| DrawView {
            draw_id: d.draw_id,
            owner: d.owner.clone(),
            pack_coins: U128(d.pack_coins),
            status: d.status.clone(),
            prize: d.prize.clone(),
            commit_block: d.commit_block,
            table_snapshot_hash: d.table_snapshot_hash.clone(),
        })
    }

    pub fn get_pack(&self, token_id: u64) -> Option<PackView> {
        self.packs.get(&token_id).map(|p| PackView {
            token_id: p.token_id,
            owner: p.owner.clone(),
            status: p.status.clone(),
            draw_id: p.draw_id,
        })
    }

    pub fn get_top_openers(&self, limit: Option<u64>) -> Vec<TopOpener> {
        let lim = limit.unwrap_or(20).min(20) as usize;
        let mut v = self.top_openers_vec.clone();
        v.truncate(lim);
        v
    }

    pub fn get_top_pack_buyers(&self, limit: Option<u64>) -> Vec<TopBuyer> {
        let lim = limit.unwrap_or(10).min(10) as usize;
        let mut v = self.top_buyers_vec.clone();
        v.truncate(lim);
        v
    }

    pub fn get_my_stats(&self, account: AccountId) -> MyStats {
        MyStats {
            packs_bought: *self.buy_counts.get(&account).unwrap_or(&0),
            packs_opened: *self.open_counts.get(&account).unwrap_or(&0),
            volume: U128(self.buy_volume.get(&account).cloned().unwrap_or(0)),
            claimable: U128(self.claimable.get(&account).cloned().unwrap_or(0)),
            scrap_credits: *self.scrap_credit.get(&account).unwrap_or(&0),
        }
    }

    pub fn get_recent_draws(&self, limit: Option<u64>) -> Vec<RecentEntry> {
        let lim = limit.unwrap_or(50).min(50) as usize;
        self.recent_draws.iter().rev().take(lim).cloned().collect()
    }

    pub fn get_machine_summary(&self) -> MachineSummary {
        MachineSummary {
            coin: self.coin.clone(),
            coin_token: self.coin_token.clone(),
            total_packs_sold: self.total_packs_sold,
            total_packs_opened: self.total_packs_opened,
            total_volume: U128(self.total_volume),
            prize_pool: U128(self.prize_pool),
            megapack_pot: U128(self.megapack_pot),
        }
    }

    pub fn get_claimable(&self, account: AccountId) -> U128 {
        U128(self.claimable.get(&account).cloned().unwrap_or(0))
    }

    pub fn get_pending_protocol_fees(&self) -> U128 {
        U128(self.pending_protocol_fees)
    }

    pub fn get_pending_coin_refund(&self, account: AccountId) -> U128 {
        U128(
            self.pending_coin_refunds
                .get(&account)
                .cloned()
                .unwrap_or(0),
        )
    }

    pub fn get_pending_refund(&self, account: AccountId) -> U128 {
        U128(self.pending_refunds.get(&account).cloned().unwrap_or(0))
    }

    pub fn get_reserved_discounts(&self, account: AccountId) -> U128 {
        U128(self.reserved_discounts.get(&account).cloned().unwrap_or(0))
    }

    pub fn get_coin_balance_cached(&self) -> U128 {
        U128(self.coin_balance_cached)
    }

    pub fn get_pool_free(&self) -> U128 {
        U128(
            self.prize_pool
                .checked_sub(self.prize_reserved)
                .and_then(|v| v.checked_sub(self.claimable_total))
                .unwrap_or(0),
        )
    }

    pub fn is_paused(&self) -> bool {
        self.paused
    }

    pub fn get_pack_art_url(&self) -> String {
        self.pack_art_url.clone()
    }

    pub fn get_candles(&self, limit: Option<u64>) -> Vec<Candle> {
        let lim = limit.unwrap_or(288).min(288) as usize;
        self.candles.iter().rev().take(lim).cloned().collect()
    }

    pub fn nft_token(&self, token_id: String) -> Option<NftTokenView> {
        let id: u64 = token_id.parse().ok()?;
        let pack = self.packs.get(&id)?;
        let media = if pack.status == PackStatus::Unopened || pack.status == PackStatus::Listed {
            self.pack_art_url.clone()
        } else {
            let (tier, mult) = self
                .draws
                .get(&id)
                .and_then(|d| d.prize.as_ref().map(|p| (p.tier.clone(), p.multiplier_bps)))
                .unwrap_or_else(|| ("Pending".to_string(), 0));
            let color = match tier.as_str() {
                "Rare" => "#2ED573",
                "Epic" => "#3E9CFF",
                "Mythic" => "#FF4AA6",
                _ => "#C9CED6",
            };
            let svg = format!(
                "<svg xmlns='http://www.w3.org/2000/svg' width='400' height='600'><rect width='100%' height='100%' fill='#0B0B10'/><circle cx='200' cy='250' r='80' fill='{color}'/><text x='200' y='400' text-anchor='middle' fill='white' font-size='24'>{tier}</text><text x='200' y='440' text-anchor='middle' fill='{color}' font-size='20'>{mult}x</text></svg>",
                mult = mult as f64 / 100.0
            );
            format!("data:image/svg+xml,{}", Self::encode_svg(&svg))
        };
        Some(NftTokenView {
            token_id: token_id.clone(),
            owner_id: pack.owner.clone(),
            metadata: NftMetadata {
                title: Some(format!("Pack #{}", token_id)),
                description: Some("Bigo pack".into()),
                media: Some(media),
                reference: None,
            },
        })
    }

    pub fn nft_tokens_for_owner(
        &self,
        account_id: AccountId,
        from_index: Option<String>,
        limit: Option<u64>,
    ) -> Vec<NftTokenView> {
        let start: u64 = from_index.and_then(|s| s.parse().ok()).unwrap_or(0);
        let lim = limit.unwrap_or(50).min(100) as usize;
        let mut ids: Vec<u64> = self
            .owner_tokens
            .get(&account_id)
            .cloned()
            .unwrap_or_default();
        ids.sort_unstable();
        ids.dedup();
        ids.into_iter()
            .skip(start as usize)
            .take(lim)
            .filter_map(|id| self.nft_token(id.to_string()))
            .collect()
    }

    pub fn get_protocol_fees(&self) -> U128 {
        U128(self.protocol_fees)
    }

    pub fn get_creator_fees(&self) -> U128 {
        U128(self.creator_fees)
    }

    pub fn get_protocol_fees_coin(&self) -> U128 {
        U128(self.protocol_fees_coin)
    }

    pub fn get_creator_fees_coin(&self) -> U128 {
        U128(self.creator_fees_coin)
    }

    pub fn get_listing(&self, token_id: u64) -> Option<PackListing> {
        self.listings.get(&token_id).cloned()
    }

    pub fn get_milestone_info(&self) -> MilestoneInfo {
        MilestoneInfo {
            threshold: self.milestone_threshold,
            new_top_mult_bps: self.milestone_new_top_mult_bps,
            triggered: self.milestone_triggered,
            current_table: self.prize_table.clone(),
            table_hash: self.table_hash.clone(),
        }
    }

    pub fn get_scrap_credits(&self, account: AccountId) -> u64 {
        self.scrap_credit.get(&account).cloned().unwrap_or(0)
    }

    pub fn has_scrap_discount(&self, account: AccountId) -> bool {
        self.scrap_discount_next
            .get(&account)
            .cloned()
            .unwrap_or(false)
    }

    pub fn get_megapack_pot(&self) -> U128 {
        U128(self.megapack_pot)
    }

    pub fn is_megapack_eligible(&self, account: AccountId) -> bool {
        if self.graduation_status != GraduationStatus::Graduated {
            return false;
        }
        if self.megapack_holders.get(&account).is_none() {
            return false;
        }
        let start = self
            .megapack_eligible_from_block
            .get(&account)
            .cloned()
            .unwrap_or(0);
        env::block_height() >= start + MEGAPACK_HOLD_BLOCKS
    }

    pub fn get_next_draw_id(&self) -> u64 {
        self.next_draw_id
    }

    pub fn get_graduation_status(&self) -> GraduationStatus {
        self.graduation_status.clone()
    }

    pub fn get_graduation(&self) -> near_sdk::serde_json::Value {
        near_sdk::serde_json::json!({
            "status": self.graduation_status,
            "target": U128(self.graduation_target),
            "pool_id": self.graduation_pool_id,
            "pool_ready": self.graduation_pool_id.is_some(),
            "coin_seeded": self.graduation_coin_seeded,
            "phase": self.graduation_phase,
            "stranded_coin": U128(self.stranded_coin),
            "stranded_quote": U128(self.stranded_quote),
            "error": self.graduation_error,
            "tradable_on_rhea": self.graduation_status == GraduationStatus::Graduated,
        })
    }

    pub fn is_lp_locked(&self) -> bool {
        self.lp_locked
    }

    pub fn is_largest_pack_unlocked(&self) -> bool {
        self.largest_pack_unlocked
    }

    pub fn get_curve_state(&self) -> CurveState {
        let den = self.curve_coin_reserve.max(1);
        CurveState {
            quote_reserve: U128(self.curve_quote_reserve),
            coin_reserve: U128(self.curve_coin_reserve),
            total_sold: U128(self.curve_total_sold),
            spot_price_num: U128(self.curve_quote_reserve),
            spot_price_den: U128(den),
            graduation_target: U128(self.graduation_target),
            is_graduated: self.graduation_status == GraduationStatus::Graduated,
            graduation_pool_id: self.graduation_pool_id,
            tradable_on_rhea: self.graduation_status == GraduationStatus::Graduated,
        }
    }

    pub fn get_outlayer_account(&self) -> AccountId {
        self.outlayer_account.clone()
    }

    pub fn get_vrf_project_id(&self) -> String {
        self.vrf_project_id.clone()
    }

    pub fn get_ft_gas(&self) -> u64 {
        self.ft_gas
    }

    pub fn get_allocations(&self) -> near_sdk::serde_json::Value {
        near_sdk::serde_json::json!({
            "curve_coin_allocation": U128(self.curve_coin_allocation),
            "prize_coin_allocation": U128(self.prize_coin_allocation),
            "max_pack_price": U128(self.max_pack_price),
            "scrap_discount_amount": U128(self.scrap_discount_amount),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn good_table() -> Vec<PrizeTier> {
        vec![
            PrizeTier {
                multiplier_bps: 25,
                odds_bps: 6000,
                tier: "Common".into(),
            },
            PrizeTier {
                multiplier_bps: 100,
                odds_bps: 2500,
                tier: "Common".into(),
            },
            PrizeTier {
                multiplier_bps: 200,
                odds_bps: 1000,
                tier: "Rare".into(),
            },
            PrizeTier {
                multiplier_bps: 400,
                odds_bps: 400,
                tier: "Epic".into(),
            },
            PrizeTier {
                multiplier_bps: 800,
                odds_bps: 100,
                tier: "Mythic".into(),
            },
        ]
    }

    fn init_contract() -> Contract {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(
            accounts(0),
            accounts(1),
            accounts(1),
            Some(accounts(2)),
            good_table(),
            "ipfs://QmTest".into(),
            "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8".into(),
            accounts(3),
            "bigodeployer.testnet/bigo-vrf".into(),
            Some(50),
            Some(1000),
            Some(30),
            None,
            None,
            Some(U128(1000)),
            None,
            None,
            None,
            None,
            None,
            None,
        )
    }

    fn register(c: &mut Contract, account: AccountId) {
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(account.clone())
            .attached_deposit(NearToken::from_yoctonear(STORAGE_DEPOSIT_MIN))
            .build();
        testing_env!(ctx);
        c.storage_deposit(None, None);
    }

    #[test]
    fn init_splits_allocations() {
        let c = init_contract();
        assert_eq!(c.curve_coin_reserve, DEFAULT_CURVE_ALLOCATION);
        assert_eq!(c.prize_pool, DEFAULT_PRIZE_ALLOCATION);
        assert_eq!(c.coin_balance_cached, COIN_TOTAL_SUPPLY);
        c.assert_coin_solvent();
    }

    #[test]
    #[should_panic(expected = "price impact")]
    fn launch_rejects_oversized_pack() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(
            accounts(0),
            accounts(1),
            accounts(1),
            Some(accounts(2)),
            good_table(),
            "ipfs://QmTest".into(),
            "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8".into(),
            accounts(3),
            "p".into(),
            None,
            None,
            None,
            None,
            None,
            Some(U128(100_000_000)),
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }

    #[test]
    #[should_panic(expected = "does not cover 30 max prizes")]
    fn launch_rejects_under_seeded_prize_pool() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(
            accounts(0),
            accounts(1),
            accounts(1),
            Some(accounts(2)),
            good_table(),
            "ipfs://QmTest".into(),
            "c0c5aeae1688c2f7cc2350aae1ee3ce96eddc0de84d678db873de48268f8cfa8".into(),
            accounts(3),
            "p".into(),
            None,
            None,
            None,
            None,
            Some(U128(1)),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
    }

    #[test]
    fn buy_pack_moves_coins_into_prize_pool() {
        let mut c = init_contract();
        let buyer = accounts(3);
        register(&mut c, buyer.clone());
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build());
        c.ft_on_transfer(buyer.clone(), U128(1000), r#"{"action":"buy_pack"}"#.into());
        let draw = c.draws.get(&1).unwrap().clone();
        assert!(draw.pack_coins > 0);
        assert_eq!(c.prize_pool, DEFAULT_PRIZE_ALLOCATION + draw.pack_coins);
        assert_eq!(
            c.curve_coin_reserve,
            DEFAULT_CURVE_ALLOCATION - draw.pack_coins
        );
        assert_eq!(c.prize_reserved, draw.pack_coins * 8);
        c.assert_coin_solvent();
    }

    #[test]
    fn settle_credits_claimable_in_coin() {
        let mut c = init_contract();
        let buyer = accounts(3);
        register(&mut c, buyer.clone());
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build());
        c.ft_on_transfer(buyer.clone(), U128(1000), r#"{"action":"buy_pack"}"#.into());
        let draw = c.draws.get(&1).unwrap().clone();
        let tier = PrizeTier {
            multiplier_bps: 200,
            odds_bps: 1000,
            tier: "Rare".into(),
        };
        let prize = c.settle_draw_internal(1, tier, false);
        assert_eq!(prize, draw.pack_coins * 2);
        assert_eq!(c.get_claimable(buyer).0, prize);
        assert_eq!(c.prize_reserved, 0);
        c.assert_coin_solvent();
    }

    #[test]
    fn curve_buy_transfers_real_coins() {
        let mut c = init_contract();
        let buyer = accounts(3);
        register(&mut c, buyer.clone());
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build());
        c.ft_on_transfer(
            buyer.clone(),
            U128(1000),
            r#"{"action":"curve_buy"}"#.into(),
        );
        let net = 990u128;
        let coins_out = c.curve_coins_out(net);
        c.on_curve_buy_result(
            buyer,
            U128(coins_out),
            U128(1000),
            U128(5),
            U128(5),
            U128(net),
            Ok(()),
        );
        assert!(c.curve_coin_reserve < DEFAULT_CURVE_ALLOCATION);
        assert!(c.curve_quote_reserve > INITIAL_QUOTE_RESERVE);
        assert_eq!(c.creator_fees, 5);
        assert_eq!(c.protocol_fees, 5);
        c.assert_coin_solvent();
    }

    #[test]
    fn curve_sell_from_coin_side() {
        let mut c = init_contract();
        let seller = accounts(3);
        register(&mut c, seller.clone());
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(1))
            .build();
        testing_env!(ctx);
        let coin_in = 1_000_000_000u128;
        c.ft_on_transfer(
            seller.clone(),
            U128(coin_in),
            r#"{"action":"curve_sell"}"#.into(),
        );
        let fee = coin_in / 100;
        let net = coin_in - fee;
        let quote_out = c.curve_quote_reserve * net / (c.curve_coin_reserve + net);
        c.on_sell_result(
            seller,
            U128(coin_in),
            U128(net),
            U128(fee / 2),
            U128(fee - fee / 2),
            U128(quote_out),
            Ok(()),
        );
        assert!(c.curve_coin_reserve > DEFAULT_CURVE_ALLOCATION - coin_in);
        assert!(c.curve_quote_reserve < INITIAL_QUOTE_RESERVE);
        assert_eq!(c.protocol_fees_coin + c.creator_fees_coin, coin_in / 100);
        c.assert_coin_solvent();
    }

    #[test]
    fn coin_fees_withdraw_are_separate_ledgers() {
        let mut c = init_contract();
        c.protocol_fees_coin = 30;
        c.creator_fees_coin = 20;
        c.coin_balance_cached = COIN_TOTAL_SUPPLY + 50;
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(1))
            .prepaid_gas(Gas::from_tgas(200))
            .build();
        testing_env!(ctx);
        let _ = c.withdraw_protocol_fees_coin("30".into());
        assert_eq!(c.protocol_fees_coin, 0);
        assert_eq!(c.creator_fees_coin, 20);
        let _ = c.withdraw_creator_fees_coin("10".into());
        assert_eq!(c.creator_fees_coin, 10);
        assert_eq!(c.coin_balance_cached, COIN_TOTAL_SUPPLY + 10);
    }

    #[test]
    #[should_panic(expected = "exceeds protocol fees")]
    fn pending_fee_not_withdrawable() {
        let mut c = init_contract();
        c.protocol_fees = 10;
        c.pending_protocol_fees = 50;
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(0))
            .attached_deposit(NearToken::from_yoctonear(1))
            .prepaid_gas(Gas::from_tgas(200))
            .build();
        testing_env!(ctx);
        let _ = c.withdraw_protocol_fees("50".into());
    }

    #[test]
    fn seller_failure_zeroes_pending_fee() {
        let mut c = init_contract();
        c.packs.insert(
            1,
            PackRecord {
                token_id: 1,
                owner: accounts(3),
                status: PackStatus::PendingSale,
                draw_id: 1,
            },
        );
        c.pending_protocol_fees = 1;
        c.protocol_fees = 0;
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.on_seller_paid(
            accounts(4),
            accounts(3),
            1,
            U128(1000),
            Err(near_sdk::PromiseError::Failed),
        );
        assert_eq!(
            c.pending_protocol_fees, 0,
            "the pending fee is dropped on failure"
        );
        assert_eq!(
            c.protocol_fees, 0,
            "no fee is credited to the treasury ledger"
        );
        assert_eq!(
            c.pending_refunds.get(&accounts(4)).cloned().unwrap_or(0),
            1000
        );
        assert_eq!(c.packs.get(&1).unwrap().status, PackStatus::Listed);
    }

    #[test]
    fn refund_success_decrements_cache() {
        let mut c = init_contract();
        c.coin_balance_cached = COIN_TOTAL_SUPPLY + 1000;
        c.pending_coin_refunds.insert(accounts(3), 500);
        c.pending_coin_refunds_total = 500;
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.on_coin_refund_result(accounts(3), U128(500), Ok(()));
        assert_eq!(
            c.coin_balance_cached,
            COIN_TOTAL_SUPPLY + 500,
            "a paid refund leaves the contract"
        );
    }

    #[test]
    fn sync_sets_exact_balance() {
        let mut c = init_contract();
        c.coin_balance_cached = COIN_TOTAL_SUPPLY;
        let ctx = VMContextBuilder::new()
            .current_account_id(accounts(0))
            .predecessor_account_id(accounts(0))
            .build();
        testing_env!(ctx);
        c.on_coin_balance(Ok(U128(COIN_TOTAL_SUPPLY - 1000)));
        assert_eq!(
            c.coin_balance_cached,
            COIN_TOTAL_SUPPLY - 1000,
            "a shortfall is recorded exactly"
        );
        c.on_coin_balance(Ok(U128(COIN_TOTAL_SUPPLY + 500)));
        assert_eq!(
            c.coin_balance_cached,
            COIN_TOTAL_SUPPLY + 500,
            "a surplus is recorded exactly"
        );
    }

    #[test]
    fn pending_refund_is_not_claimable() {
        let mut c = init_contract();
        c.pending_refunds.insert(accounts(3), 500);
        assert_eq!(c.get_pending_refund(accounts(3)).0, 500);
        assert_eq!(c.get_claimable(accounts(3)).0, 0);
    }

    #[test]
    fn scrap_reserves_discounts() {
        let mut c = init_contract();
        c.protocol_fees = 100;
        c.scrap_discount_amount = 10;
        c.scrap_credit.insert(accounts(3), 3);
        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(3))
            .build();
        testing_env!(ctx);
        c.redeem_scrap();
        assert_eq!(c.get_reserved_discounts(accounts(3)).0, 10);
        assert_eq!(c.protocol_fees, 90);
        assert!(c.has_scrap_discount(accounts(3)));
    }

    #[test]
    fn nft_transfer_needs_exactly_one_yocto() {
        let mut c = init_contract();
        let buyer = accounts(3);
        register(&mut c, buyer.clone());
        testing_env!(VMContextBuilder::new()
            .predecessor_account_id(accounts(2))
            .build());
        c.ft_on_transfer(buyer.clone(), U128(1000), r#"{"action":"buy_pack"}"#.into());

        let ctx = VMContextBuilder::new()
            .predecessor_account_id(accounts(4))
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            c.nft_transfer(accounts(3), "1".into(), None);
        }));
        assert!(res.is_err(), "a non-owner transfer must fail");

        let ctx = VMContextBuilder::new()
            .predecessor_account_id(buyer.clone())
            .attached_deposit(NearToken::from_yoctonear(1))
            .build();
        testing_env!(ctx);
        c.nft_transfer(accounts(4), "1".into(), None);
        assert_eq!(c.packs.get(&1).unwrap().owner, accounts(4));
    }

    #[test]
    fn u128_views_serialize_as_strings() {
        let c = init_contract();
        let view = near_sdk::serde_json::to_value(c.get_machine()).unwrap();
        assert!(view["prize_pool"].is_string());
        assert!(view["prize_reserved"].is_string());
        assert!(view["claimable_total"].is_string());
        assert!(view["curve_coin_reserve"].is_string());
        let curve = near_sdk::serde_json::to_value(c.get_curve_state()).unwrap();
        assert!(curve["quote_reserve"].is_string());
        assert!(curve["coin_reserve"].is_string());
        assert!(curve["spot_price_num"].is_string());
        let pool = near_sdk::serde_json::to_value(c.get_prize_pool()).unwrap();
        assert!(pool["prize_pool"].is_string());
        assert!(pool["claimable_total"].is_string());
    }

    #[test]
    fn hex_decode_roundtrip() {
        assert_eq!(Contract::hex_decode("0x0102ff"), vec![1, 2, 255]);
    }
}
