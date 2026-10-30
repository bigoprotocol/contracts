use near_sdk::{env, near, PanicOnDefault, AccountId, require};

mod error;

#[near(serializers = [json, borsh])]
#[derive(Debug, Clone)]
pub struct PrizeTier {
    pub multiplier_bps: u32,
    pub odds_bps: u32,
    pub tier: String,
}

#[near(contract_state)]
#[derive(PanicOnDefault)]
pub struct Contract {
    owner: AccountId,
    coin: AccountId,
    table_hash: String,
    pack_art_url: String,
    prize_table: Vec<PrizeTier>,
    vrf_pubkey: String,
}

#[near]
impl Contract {
    #[init]
    pub fn new(
        owner: AccountId,
        coin: AccountId,
        prize_table: Vec<PrizeTier>,
        pack_art_url: String,
        vrf_pubkey: String,
    ) -> Self {
        assert!(!env::state_exists(), "already initialized");
        require!(
            pack_art_url.starts_with("ipfs://") || pack_art_url.starts_with("https://"),
            "invalid art url: must start with ipfs:// or https://"
        );
        require!(!pack_art_url.is_empty(), "empty art url");
        require!(!prize_table.is_empty(), "empty prize table");

        // EV cap: sum(odds * multiplier) <= 0.95. odds_bps/10000 * multiplier_bps/100.
        // Scaled EV*100 = sum(odds_bps * multiplier_bps /10000) must be <=95.
        let mut ev_scaled: u64 = 0;
        for t in &prize_table {
            ev_scaled += (t.odds_bps as u64 * t.multiplier_bps as u64) / 10000;
        }
        require!(ev_scaled <= 95, "EV exceeds cap 0.95");


        // simple hash from table json
        let table_str = prize_table.iter().map(|t| format!("{}:{}:{};", t.tier, t.multiplier_bps, t.odds_bps)).collect::<String>();
        let hash_bytes = env::sha256(table_str.as_bytes());
        let table_hash = hash_bytes.iter().map(|b| format!("{:02x}", b)).collect::<String>();

        Self { owner, coin, table_hash, pack_art_url, prize_table, vrf_pubkey }
    }

    pub fn get_machine(&self) -> MachineView {
        MachineView {
            owner: self.owner.clone(),
            coin: self.coin.clone(),
            table_hash: self.table_hash.clone(),
            pack_art_url: self.pack_art_url.clone(),
            prize_table: self.prize_table.clone(),
        }
    }

    pub fn get_table_hash(&self) -> String {
        self.table_hash.clone()
    }
}

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct MachineView {
    pub owner: AccountId,
    pub coin: AccountId,
    pub table_hash: String,
    pub pack_art_url: String,
    pub prize_table: Vec<PrizeTier>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::{accounts, VMContextBuilder};
    use near_sdk::testing_env;

    fn good_table() -> Vec<PrizeTier> {
        vec![
            PrizeTier { multiplier_bps: 25, odds_bps: 6000, tier: "Common".into() },
            PrizeTier { multiplier_bps: 100, odds_bps: 2500, tier: "Common".into() },
            PrizeTier { multiplier_bps: 200, odds_bps: 1000, tier: "Rare".into() },
            PrizeTier { multiplier_bps: 400, odds_bps: 400, tier: "Epic".into() },
            PrizeTier { multiplier_bps: 800, odds_bps: 100, tier: "Mythic".into() },
        ]
    }

    #[test]
    fn init_ok() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let c = Contract::new(accounts(0), accounts(1), good_table(), "ipfs://QmTest".into(), "pubkey".into());
        assert!(!c.table_hash.is_empty());
    }

    #[test]
    #[should_panic(expected = "EV exceeds cap")]
    fn ev_cap_rejects() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        let bad = vec![PrizeTier { multiplier_bps: 10000, odds_bps: 10000, tier: "Mythic".into() }];
        Contract::new(accounts(0), accounts(1), bad, "ipfs://QmTest".into(), "pubkey".into());
    }



    #[test]
    #[should_panic(expected = "invalid art url")]
    fn art_url_rejects() {
        let ctx = VMContextBuilder::new().build();
        testing_env!(ctx);
        Contract::new(accounts(0), accounts(1), good_table(), "".into(), "pubkey".into());
    }
}
