// views are implemented directly in lib.rs #[near] impl for near-sdk 5 compatibility.
// This file documents the view surface and holds helpers.

use crate::{DrawView, PackView, MachineView, TopOpener, TopBuyer, MyStats, RecentEntry, MachineSummary, RecentTrade, Candle};
use near_sdk::near;

#[near(serializers = [json, borsh])]
#[derive(Clone)]
pub struct ViewsDoc {
    pub note: String,
}
