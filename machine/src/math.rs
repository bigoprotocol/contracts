use near_sdk::require;

pub fn checked_add(a: u128, b: u128) -> u128 {
    a.checked_add(b).expect("addition overflow")
}

pub fn checked_sub(a: u128, b: u128) -> u128 {
    a.checked_sub(b).expect("subtraction underflow")
}

// ev_scaled = sum(odds_bps * multiplier_bps / 10000)
// multiplier_bps is *100 (100 = 1x, 800 = 8x), odds_bps is *10000
// so ev in 1/100 units: 25 => 0.25x, 100 => 1x, etc scaled already
// ev_scaled = sum(odds_bps * multiplier_bps /10000) must be <=95 for 0.95 cap
pub fn compute_ev_scaled(tiers: &[(u32, u32)]) -> u64 {
    let mut ev: u64 = 0;
    for (mult_bps, odds_bps) in tiers {
        ev += (*odds_bps as u64 * *mult_bps as u64) / 10000;
    }
    ev
}

pub fn assert_solvency(pool_balance: u128, liabilities_max: u128, claimable_total: u128) {
    require!(
        liabilities_max + claimable_total <= pool_balance,
        format!(
            "solvency breach: liabilities {} + claimable {} > balance {}",
            liabilities_max, claimable_total, pool_balance
        )
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ev_default_table_is_84() {
        // default table: 0.25*60% +1*25%+2*10%+4*4%+8*1% = 0.84
        let tiers = vec![(25u32, 6000u32), (100, 2500), (200, 1000), (400, 400), (800, 100)];
        // ev_scaled = 25*6000/10000 + ... = 15 +25+20+16+8 =84
        assert_eq!(compute_ev_scaled(&tiers), 84);
    }
}
