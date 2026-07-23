//! ロイヤリティ計算。表はデータとして保持し、ローカルルールの差し替えを許す(ADR 0003)。

use std::collections::BTreeMap;

use crate::Rank;
use crate::hand::{Category, HandRank};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Top,
    Middle,
    Bottom,
}

/// ロイヤリティ表。top はペア/トリップスのランク別、middle/bottom はカテゴリ別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RoyaltyTable {
    pub(crate) top_pair: BTreeMap<Rank, u32>,
    pub(crate) top_trips: BTreeMap<Rank, u32>,
    pub(crate) middle: BTreeMap<Category, u32>,
    pub(crate) bottom: BTreeMap<Category, u32>,
}

impl RoyaltyTable {
    /// アメリカン標準のロイヤリティ表。
    pub fn standard_american() -> Self {
        // top ペア: 66=1 から AA=9 まで 1 点刻み
        let pair_ranks = [
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        let top_pair = pair_ranks.into_iter().zip(1u32..).collect();

        // top トリップス: 222=10 から AAA=22 まで 1 点刻み
        let trips_ranks = [
            Rank::Two,
            Rank::Three,
            Rank::Four,
            Rank::Five,
            Rank::Six,
            Rank::Seven,
            Rank::Eight,
            Rank::Nine,
            Rank::Ten,
            Rank::Jack,
            Rank::Queen,
            Rank::King,
            Rank::Ace,
        ];
        let top_trips = trips_ranks.into_iter().zip(10u32..).collect();

        let middle = BTreeMap::from([
            (Category::Trips, 2),
            (Category::Straight, 4),
            (Category::Flush, 8),
            (Category::FullHouse, 12),
            (Category::Quads, 20),
            (Category::StraightFlush, 30),
            (Category::RoyalFlush, 50),
        ]);
        let bottom = BTreeMap::from([
            (Category::Straight, 2),
            (Category::Flush, 4),
            (Category::FullHouse, 6),
            (Category::Quads, 10),
            (Category::StraightFlush, 15),
            (Category::RoyalFlush, 25),
        ]);

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
        table.insert(category, points);
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
                .and_then(|rank| rank_table.get(rank))
                .copied()
                .unwrap_or(0)
        }
        Row::Middle => table.middle.get(&hand.category).copied().unwrap_or(0),
        Row::Bottom => table.bottom.get(&hand.category).copied().unwrap_or(0),
    }
}
