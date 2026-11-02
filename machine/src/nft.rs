#![allow(dead_code)]
use crate::{PackRecord, PackStatus};
use near_sdk::AccountId;

pub fn make_pack_record(token_id: u64, owner: AccountId, draw_id: u64) -> PackRecord {
    PackRecord { token_id, owner, status: PackStatus::Unopened, draw_id }
}

pub fn is_unopened(pack: &PackRecord) -> bool {
    pack.status == PackStatus::Unopened
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::accounts;

    #[test]
    fn pack_record_is_unopened() {
        let r = make_pack_record(1, accounts(0), 1);
        assert!(is_unopened(&r));
        assert_eq!(r.token_id, 1);
    }
}
