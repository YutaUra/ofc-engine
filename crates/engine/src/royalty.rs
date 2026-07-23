//! ロイヤリティ計算。表はデータとして保持し、ローカルルールの差し替えを許す(ADR 0003)。
//!
//! 内部表現は BTreeMap ではなく固定長配列(ランク/カテゴリの判別値で添字引き)。
//! 表引きは Joker 解決やソルバーの内側で大量に呼ばれるため、ポインタ追跡の
//! ない O(1) 参照にしている(ADR 0003 の「init 時に lookup 表へ焼く」方針)。

use crate::Rank;
use crate::hand::{Category, HandRank};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Top,
    Middle,
    Bottom,
}

const CATEGORY_COUNT: usize = 10;

/// ロイヤリティ表。top はペア/トリップスのランク別、middle/bottom はカテゴリ別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoyaltyTable {
    pub(crate) top_pair: [u32; 13],
    pub(crate) top_trips: [u32; 13],
    pub(crate) middle: [u32; CATEGORY_COUNT],
    pub(crate) bottom: [u32; CATEGORY_COUNT],
}

impl RoyaltyTable {
    /// アメリカン標準のロイヤリティ表。
    pub fn standard_american() -> Self {
        let mut top_pair = [0u32; 13];
        // 66=1 から AA=9 まで 1 点刻み
        for (offset, points) in (Rank::Six as usize..=Rank::Ace as usize).zip(1u32..) {
            top_pair[offset] = points;
        }
        let mut top_trips = [0u32; 13];
        // 222=10 から AAA=22 まで 1 点刻み
        for (rank, points) in (0..13).zip(10u32..) {
            top_trips[rank] = points;
        }

        let mut middle = [0u32; CATEGORY_COUNT];
        middle[Category::Trips as usize] = 2;
        middle[Category::Straight as usize] = 4;
        middle[Category::Flush as usize] = 8;
        middle[Category::FullHouse as usize] = 12;
        middle[Category::Quads as usize] = 20;
        middle[Category::StraightFlush as usize] = 30;
        middle[Category::RoyalFlush as usize] = 50;

        let mut bottom = [0u32; CATEGORY_COUNT];
        bottom[Category::Straight as usize] = 2;
        bottom[Category::Flush as usize] = 4;
        bottom[Category::FullHouse as usize] = 6;
        bottom[Category::Quads as usize] = 10;
        bottom[Category::StraightFlush as usize] = 15;
        bottom[Category::RoyalFlush as usize] = 25;

        Self {
            top_pair,
            top_trips,
            middle,
            bottom,
        }
    }

    /// middle / bottom のカテゴリ別点数を差し替える(ローカルルール対応)。
    /// top はランク別体系のため専用の setter を将来用意する。
    pub fn set_row_points(&mut self, row: Row, category: Category, points: u32) {
        let table = match row {
            Row::Middle => &mut self.middle,
            Row::Bottom => &mut self.bottom,
            Row::Top => unimplemented!("top はランク別体系のため専用 setter で扱う"),
        };
        table[category as usize] = points;
    }
}

pub fn royalty_points(row: Row, hand: &HandRank, table: &RoyaltyTable) -> u32 {
    match row {
        Row::Top => {
            let rank_table = match hand.category {
                Category::Pair => &table.top_pair,
                Category::Trips => &table.top_trips,
                _ => return 0,
            };
            hand.tiebreak
                .first()
                .map(|rank| rank_table[*rank as usize])
                .unwrap_or(0)
        }
        Row::Middle => table.middle[hand.category as usize],
        Row::Bottom => table.bottom[hand.category as usize],
    }
}
