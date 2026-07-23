//! RuleSet: JSON で完全データ化されたルール(ADR 0003)。
//! `compile()` で検証済みの CompiledRules に固め、評価系は CompiledRules のみを
//! 参照する(実行時にルール config を読み直さない)。

use std::collections::BTreeMap;

use crate::Rank;
use crate::fantasyland::FantasylandRules;
use crate::hand::Category;
use crate::royalty::RoyaltyTable;
use crate::scoring::ScoringRules;

/// デッキに入れられる Joker の上限。これを超えるルールは compile で拒否する
/// (Joker 解決の総当たりコストと Board の前提が 2 枚までを想定しているため)。
pub const MAX_JOKERS: u8 = 2;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RuleSet {
    pub variant: String,
    pub players: u8,
    pub deck: DeckRules,
    pub scoring: ScoringWire,
    pub royalties: RoyaltiesWire,
    pub fantasyland: FantasylandWire,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeckRules {
    pub jokers: u8,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ScoringWire {
    pub row_point: i32,
    pub scoop_bonus: i32,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RoyaltiesWire {
    pub top: TopRoyaltiesWire,
    pub middle: BTreeMap<Category, u32>,
    pub bottom: BTreeMap<Category, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TopRoyaltiesWire {
    pub pair: BTreeMap<Rank, u32>,
    pub trips: BTreeMap<Rank, u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FantasylandWire {
    pub pair_cards: BTreeMap<Rank, u8>,
    pub trips_cards: u8,
    pub stay_top_trips: bool,
    pub stay_bottom_quads_or_better: bool,
}

/// compile 済みルール。評価系はこの型のみを参照する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CompiledRules {
    pub players: u8,
    pub jokers: u8,
    pub scoring: ScoringRules,
    pub royalty: RoyaltyTable,
    pub fantasyland: FantasylandRules,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleSetError {
    TooManyJokers { max: u8, actual: u8 },
    TooFewPlayers { actual: u8 },
}

impl RuleSet {
    /// 標準 Pineapple(Joker なし・アメリカンロイヤリティ・progressive FL)。
    pub fn standard_pineapple() -> Self {
        let royalty = RoyaltyTable::standard_american();
        let fl = FantasylandRules::standard_progressive();
        Self {
            variant: "pineapple".to_string(),
            players: 2,
            deck: DeckRules { jokers: 0 },
            scoring: ScoringWire {
                row_point: 1,
                scoop_bonus: 3,
            },
            royalties: RoyaltiesWire {
                top: TopRoyaltiesWire {
                    pair: royalty.top_pair,
                    trips: royalty.top_trips,
                },
                middle: royalty.middle,
                bottom: royalty.bottom,
            },
            fantasyland: FantasylandWire {
                pair_cards: fl.pair_cards,
                trips_cards: fl.trips_cards,
                stay_top_trips: fl.stay_top_trips,
                stay_bottom_quads_or_better: fl.stay_bottom_quads_or_better,
            },
        }
    }

    pub fn compile(&self) -> Result<CompiledRules, RuleSetError> {
        if self.deck.jokers > MAX_JOKERS {
            return Err(RuleSetError::TooManyJokers {
                max: MAX_JOKERS,
                actual: self.deck.jokers,
            });
        }
        if self.players < 2 {
            return Err(RuleSetError::TooFewPlayers {
                actual: self.players,
            });
        }
        Ok(CompiledRules {
            players: self.players,
            jokers: self.deck.jokers,
            scoring: ScoringRules {
                row_point: self.scoring.row_point,
                scoop_bonus: self.scoring.scoop_bonus,
            },
            royalty: RoyaltyTable {
                top_pair: self.royalties.top.pair.clone(),
                top_trips: self.royalties.top.trips.clone(),
                middle: self.royalties.middle.clone(),
                bottom: self.royalties.bottom.clone(),
            },
            fantasyland: FantasylandRules {
                pair_cards: self.fantasyland.pair_cards.clone(),
                trips_cards: self.fantasyland.trips_cards,
                stay_top_trips: self.fantasyland.stay_top_trips,
                stay_bottom_quads_or_better: self.fantasyland.stay_bottom_quads_or_better,
            },
        })
    }
}
