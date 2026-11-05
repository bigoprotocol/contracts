// basis points: 20 = 0.20 percent, 15 = 0.15 percent
pub fn fee_amount(amount: u128, bps: u32) -> u128 {
    amount.saturating_mul(bps as u128) / 10000
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_20bps_of_1000_is_2() {
        assert_eq!(fee_amount(1000, 20), 2);
    }

    #[test]
    fn fee_15bps_of_10000_is_15() {
        assert_eq!(fee_amount(10000, 15), 15);
    }
}
