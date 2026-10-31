// fee math helpers used by pack buy flow

// basis points helpers: 20 = 0.20%, 95 = 0.95%, 15 = 0.15%
pub fn fee_amount(amount: u128, bps: u32) -> u128 {
    amount.saturating_mul(bps as u128) / 10000
}

pub fn max_multiplier_bps(prize_table: &[crate::PrizeTier]) -> u32 {
    prize_table.iter().map(|t| t.multiplier_bps).max().unwrap_or(0)
}

pub fn min_multiplier_bps(prize_table: &[crate::PrizeTier]) -> u32 {
    prize_table.iter().map(|t| t.multiplier_bps).min().unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_20bps_of_1000_is_2() {
        assert_eq!(fee_amount(1000, 20), 2);
    }

    #[test]
    fn fee_95bps_of_10000_is_95() {
        assert_eq!(fee_amount(10000, 95), 95);
    }
}
