use near_sdk::store::LookupMap;
use near_sdk::{env, near, PanicOnDefault, AccountId, require};

mod error;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SquadStatus {
    Created,
    Funded,
    Bought,
    Opened,
    Refunded,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct Squad {
    pub id: u64,
    pub creator: AccountId,
    pub machine: AccountId,
    pub pack_price: u128,
    pub target: u128,
    pub deposited: u128,
    pub members: Vec<AccountId>,
    pub deadline_block: u64,
    pub status: SquadStatus,
    pub max_quote_budget: Option<u128>,
    pub min_net_tokens: Option<u128>,
    pub prize_total: u128,
}

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct SquadView {
    pub id: u64,
    pub creator: AccountId,
    pub machine: AccountId,
    pub pack_price: String,
    pub target: String,
    pub deposited: String,
    pub members: Vec<AccountId>,
    pub deadline_block: u64,
    pub status: SquadStatus,
    pub prize_total: String,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    pub owner: AccountId,
    pub squads: LookupMap<u64, Squad>,
    pub member_deposits: LookupMap<(u64, AccountId), u128>,
    pub next_id: u64,
    pub claimable: LookupMap<(u64, AccountId), u128>,
}

#[near]
impl Contract {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        assert!(!env::state_exists(), "already initialized");
        Self {
            owner,
            squads: LookupMap::new(b"s"),
            member_deposits: LookupMap::new(b"m"),
            next_id: 1,
            claimable: LookupMap::new(b"c"),
        }
    }

    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    pub fn create_squad(
        &mut self,
        machine: AccountId,
        pack_price: String,
        deadline_blocks: Option<u64>,
        max_quote_budget: Option<String>,
        min_net_tokens: Option<String>,
    ) -> u64 {
        let creator = env::predecessor_account_id();
        let price: u128 = pack_price.parse().expect("invalid pack_price");
        require!(price > 0, "pack_price must be >0");
        let deadline = env::block_height() + deadline_blocks.unwrap_or(100);
        let id = self.next_id;
        self.next_id += 1;

        let max_q = max_quote_budget.map(|s| s.parse::<u128>().expect("invalid max_quote_budget"));
        let min_n = min_net_tokens.map(|s| s.parse::<u128>().expect("invalid min_net_tokens"));

        let squad = Squad {
            id,
            creator: creator.clone(),
            machine,
            pack_price: price,
            target: price,
            deposited: 0,
            members: vec![],
            deadline_block: deadline,
            status: SquadStatus::Created,
            max_quote_budget: max_q,
            min_net_tokens: min_n,
            prize_total: 0,
        };
        self.squads.insert(id, squad);
        id
    }

    #[payable]
    pub fn join_squad(&mut self, squad_id: u64, deposit_amount: Option<String>) -> String {
        let joiner = env::predecessor_account_id();
        require!(env::attached_deposit().as_yoctonear() >= 1, "attach 1 yocto");
        let amount: u128 = if let Some(a) = deposit_amount {
            a.parse().expect("invalid deposit_amount")
        } else {
            0
        };
        require!(amount > 0, "deposit must be >0");

        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        if squad.status == SquadStatus::Bought || squad.status == SquadStatus::Opened || squad.status == SquadStatus::Refunded {
            env::panic_str("already bought, cannot join");
        }
        require!(
            squad.status == SquadStatus::Created || squad.status == SquadStatus::Funded,
            "squad not open for joins"
        );
        require!(env::block_height() <= squad.deadline_block, "deadline passed");

        let key = (squad_id, joiner.clone());
        let existing = self.member_deposits.get(&key).cloned().unwrap_or(0);
        self.member_deposits.insert(key, existing + amount);

        squad.deposited += amount;
        if !squad.members.contains(&joiner) {
            squad.members.push(joiner);
        }

        if squad.deposited >= squad.target {
            squad.status = SquadStatus::Funded;
        }

        self.squads.insert(squad_id, squad.clone());
        squad.deposited.to_string()
    }

    pub fn buy_squad_pack(&mut self, squad_id: u64) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status == SquadStatus::Funded, "not funded enough");
        require!(
            env::block_height() <= squad.deadline_block,
            "deadline passed - refund instead"
        );
        if squad.status == SquadStatus::Bought || squad.status == SquadStatus::Opened || squad.status == SquadStatus::Refunded {
            env::panic_str("already bought or refunded");
        }
        if let Some(max_q) = squad.max_quote_budget {
            require!(squad.pack_price <= max_q, "pack price exceeds max_quote_budget");
        }
        if let Some(min_n) = squad.min_net_tokens {
            require!(squad.pack_price >= min_n, "pack price below min_net_tokens");
        }
        squad.status = SquadStatus::Bought;
        self.squads.insert(squad_id, squad);
        true
    }

    pub fn open_squad_pack(&mut self, squad_id: u64, prize_amount: Option<String>) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        require!(squad.status == SquadStatus::Bought, "not bought yet");
        let prize: u128 = prize_amount.map(|s| s.parse().expect("invalid prize")).unwrap_or(squad.pack_price * 2);
        squad.prize_total = prize;
        squad.status = SquadStatus::Opened;

        // Distribute pro-rata into claimable ledger pull-based, never push transfers
        for member in squad.members.iter() {
            let key = (squad_id, member.clone());
            let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
            if deposit > 0 && squad.deposited > 0 {
                let share = deposit * prize / squad.deposited;
                self.claimable.insert((squad_id, member.clone()), share);
            }
        }

        self.squads.insert(squad_id, squad);
        true
    }

    pub fn claim_share(&mut self, squad_id: u64) -> String {
        let claimer = env::predecessor_account_id();
        let key = (squad_id, claimer.clone());
        let amount = self.claimable.get(&key).cloned().unwrap_or(0);
        require!(amount > 0, "nothing to claim");
        self.claimable.insert(key, 0);
        amount.to_string()
    }

    pub fn refund_squad(&mut self, squad_id: u64) -> bool {
        let mut squad = self.squads.get(&squad_id).expect("squad not found").clone();
        if squad.status == SquadStatus::Bought || squad.status == SquadStatus::Opened || squad.status == SquadStatus::Refunded {
            env::panic_str("already bought, cannot refund");
        }
        require!(env::block_height() > squad.deadline_block, "deadline not passed yet");
        require!(
            squad.status == SquadStatus::Created || squad.status == SquadStatus::Funded,
            "already bought/opened/refunded - cannot refund"
        );
        squad.status = SquadStatus::Refunded;
        for member in squad.members.iter() {
            let key = (squad_id, member.clone());
            let deposit = self.member_deposits.get(&key).cloned().unwrap_or(0);
            if deposit > 0 {
                self.claimable.insert((squad_id, member.clone()), deposit);
            }
        }
        self.squads.insert(squad_id, squad);
        true
    }

    pub fn get_squad(&self, squad_id: u64) -> Option<SquadView> {
        self.squads.get(&squad_id).map(|s| SquadView {
            id: s.id,
            creator: s.creator.clone(),
            machine: s.machine.clone(),
            pack_price: s.pack_price.to_string(),
            target: s.target.to_string(),
            deposited: s.deposited.to_string(),
            members: s.members.clone(),
            deadline_block: s.deadline_block,
            status: s.status.clone(),
            prize_total: s.prize_total.to_string(),
        })
    }

    pub fn get_member_deposit(&self, squad_id: u64, account: AccountId) -> String {
        self.member_deposits.get(&(squad_id, account)).cloned().unwrap_or(0).to_string()
    }

    pub fn get_claimable(&self, squad_id: u64, account: AccountId) -> String {
        self.claimable.get(&(squad_id, account)).cloned().unwrap_or(0).to_string()
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
        Contract {
            owner: accounts(0),
            squads: LookupMap::new(b"s"),
            member_deposits: LookupMap::new(b"m"),
            next_id: 1,
            claimable: LookupMap::new(b"c"),
        }
    }

    #[test]
    fn create_and_join_and_buy_and_claim() {
        let mut c = setup();
        let ctx_creator = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx_creator);
        let id = c.create_squad(accounts(5), "1000".into(), Some(100), Some("2000".into()), Some("500".into()));
        assert_eq!(id, 1);

        let ctx_m1 = VMContextBuilder::new().predecessor_account_id(accounts(1)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_m1);
        c.join_squad(id, Some("600".into()));
        assert_eq!(c.get_member_deposit(id, accounts(1)), "600");

        let ctx_m2 = VMContextBuilder::new().predecessor_account_id(accounts(2)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_m2);
        c.join_squad(id, Some("400".into()));
        let squad = c.get_squad(id).unwrap();
        assert_eq!(squad.status, SquadStatus::Funded);
        assert_eq!(squad.deposited, "1000");

        let ctx_any = VMContextBuilder::new().predecessor_account_id(accounts(3)).build();
        testing_env!(ctx_any);
        assert!(c.buy_squad_pack(id));

        let ctx_open = VMContextBuilder::new().predecessor_account_id(accounts(3)).build();
        testing_env!(ctx_open);
        assert!(c.open_squad_pack(id, Some("2000".into())));

        let ctx_claim1 = VMContextBuilder::new().predecessor_account_id(accounts(1)).build();
        testing_env!(ctx_claim1);
        let claimed = c.claim_share(id);
        assert_eq!(claimed, "1200");

        let ctx_claim2 = VMContextBuilder::new().predecessor_account_id(accounts(2)).build();
        testing_env!(ctx_claim2);
        let claimed2 = c.claim_share(id);
        assert_eq!(claimed2, "800");
    }

    #[test]
    #[should_panic(expected = "already bought, cannot join")]
    fn cannot_join_after_bought() {
        let mut c = setup();
        let ctx_creator = VMContextBuilder::new().predecessor_account_id(accounts(0)).build();
        testing_env!(ctx_creator);
        let id = c.create_squad(accounts(5), "1000".into(), Some(100), None, None);

        let ctx_m1 = VMContextBuilder::new().predecessor_account_id(accounts(1)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_m1);
        c.join_squad(id, Some("1000".into()));

        let ctx_buy = VMContextBuilder::new().predecessor_account_id(accounts(2)).build();
        testing_env!(ctx_buy);
        c.buy_squad_pack(id);

        let ctx_join_late = VMContextBuilder::new().predecessor_account_id(accounts(3)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_join_late);
        c.join_squad(id, Some("100".into()));
    }

    #[test]
    fn refund_after_deadline() {
        let mut c = setup();
        let ctx_creator = VMContextBuilder::new().block_height(0).predecessor_account_id(accounts(0)).build();
        testing_env!(ctx_creator);
        let id = c.create_squad(accounts(5), "1000".into(), Some(10), None, None);

        let ctx_m1 = VMContextBuilder::new().block_height(1).predecessor_account_id(accounts(1)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_m1);
        c.join_squad(id, Some("600".into()));

        let ctx_refund = VMContextBuilder::new().block_height(11).predecessor_account_id(accounts(1)).build();
        testing_env!(ctx_refund);
        assert!(c.refund_squad(id));

        let ctx_claim = VMContextBuilder::new().block_height(11).predecessor_account_id(accounts(1)).build();
        testing_env!(ctx_claim);
        let claimed = c.claim_share(id);
        assert_eq!(claimed, "600");
    }

    #[test]
    #[should_panic(expected = "already bought, cannot refund")]
    fn refund_race_guarded_after_bought() {
        let mut c = setup();
        let ctx_creator = VMContextBuilder::new().block_height(0).predecessor_account_id(accounts(0)).build();
        testing_env!(ctx_creator);
        let id = c.create_squad(accounts(5), "1000".into(), Some(10), None, None);

        let ctx_m1 = VMContextBuilder::new().block_height(1).predecessor_account_id(accounts(1)).attached_deposit(near_sdk::NearToken::from_yoctonear(1)).build();
        testing_env!(ctx_m1);
        c.join_squad(id, Some("1000".into()));

        let ctx_buy = VMContextBuilder::new().block_height(2).predecessor_account_id(accounts(2)).build();
        testing_env!(ctx_buy);
        c.buy_squad_pack(id);

        let ctx_refund = VMContextBuilder::new().block_height(11).predecessor_account_id(accounts(1)).build();
        testing_env!(ctx_refund);
        c.refund_squad(id);
    }
}
