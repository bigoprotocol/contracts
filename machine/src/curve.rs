// Bonding curve math helpers
// Simple constant-product style with virtual reserves: price increases as more sold
// We use curve_quote_reserve and curve_coin_reserve to price trades
// For buy: user gives quote, receives coin: amount = coin_reserve * quote_in / (quote_reserve + quote_in)
// For sell: user gives coin, receives quote: quote_out = quote_reserve * coin_in / (coin_reserve + coin_in)
// Fee 1% taken from input before swap, split 0.5% creator, 0.5% protocol

pub fn fee_split(amount: u128) -> (u128, u128, u128) {
    // returns (fee_total, creator_fee, protocol_fee) where each 0.5%
    let fee_total = amount / 100; // 1%
    let creator = fee_total / 2;
    let protocol = fee_total - creator;
    (fee_total, creator, protocol)
}

pub fn net_after_fee(amount: u128) -> u128 {
    amount - amount / 100
}

pub fn buy_quote_to_coin(quote_reserve: u128, coin_reserve: u128, quote_in_net: u128) -> u128 {
    // coin_out = coin_reserve * quote_in_net / (quote_reserve + quote_in_net)
    if quote_reserve == 0 || coin_reserve == 0 || quote_in_net == 0 {
        return 0;
    }
    let numerator = coin_reserve.checked_mul(quote_in_net).unwrap_or(0);
    let denominator = quote_reserve + quote_in_net;
    numerator / denominator
}

pub fn sell_coin_to_quote(quote_reserve: u128, coin_reserve: u128, coin_in_net: u128) -> u128 {
    // quote_out = quote_reserve * coin_in_net / (coin_reserve + coin_in_net)
    if quote_reserve == 0 || coin_reserve == 0 || coin_in_net == 0 {
        return 0;
    }
    let numerator = quote_reserve.checked_mul(coin_in_net).unwrap_or(0);
    let denominator = coin_reserve + coin_in_net;
    numerator / denominator
}

pub fn price_per_coin(quote_reserve: u128, coin_reserve: u128) -> f64 {
    if coin_reserve == 0 {
        return 0.0;
    }
    quote_reserve as f64 / coin_reserve as f64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fee_split_10000() {
        let (total, creator, protocol) = fee_split(10000);
        assert_eq!(total, 100);
        assert_eq!(creator, 50);
        assert_eq!(protocol, 50);
    }

    #[test]
    fn buy_math_increases_price() {
        let mut q = 10000u128;
        let mut c = 1_000_000u128;
        let out1 = buy_quote_to_coin(q, c, 1000);
        q += 1000;
        c -= out1;
        let out2 = buy_quote_to_coin(q, c, 1000);
        // second buy for same quote should give fewer coins (price up)
        assert!(out2 < out1);
    }

    #[test]
    fn sell_math_decreases_price() {
        let mut q = 10000u128;
        let mut c = 1_000_000u128;
        // first buy to move price up
        let out = buy_quote_to_coin(q, c, 5000);
        q += 5000;
        c -= out;
        let quote_back = sell_coin_to_quote(q, c, out);
        // quote back less than original due to fee and slippage? Actually without fee should be ~5000 but less due to reserve shift
        // With constant product it returns slightly less
        assert!(quote_back <= 5000);
    }

    #[test]
    fn net_after_fee_10000_is_9900() {
        assert_eq!(net_after_fee(10000), 9900);
    }
}
