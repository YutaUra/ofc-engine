//! Fantasyland 突入・継続判定。条件と配布枚数はデータで表現する(ADR 0003)。
//! FL の EV 評価はエンジンの責務外(ソルバー側が FantasylandValues を注入する)。

use std::collections::BTreeMap;

use crate::foul::{FoulCheckError, check_foul};
use crate::hand::{Category, evaluate_five, evaluate_three};
use crate::{Board, Rank};

/// FL のルール。突入条件は「pair_cards にランクが載っているか」で表現するため、
/// 条件と配布枚数が別々に矛盾することがない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FantasylandRules {
    /// top のペアランク → 配布枚数。マップにないペアは突入なし。
    pair_cards: BTreeMap<Rank, u8>,
    /// top トリップスでの配布枚数(ランク不問)。
    trips_cards: u8,
    /// 継続条件: top トリップス。
    stay_top_trips: bool,
    /// 継続条件: bottom クワッズ以上。
    stay_bottom_quads_or_better: bool,
}

impl FantasylandRules {
    /// progressive FL の標準: QQ=14 / KK=15 / AA=16 / トリップス=17。
    pub fn standard_progressive() -> Self {
        Self {
            pair_cards: BTreeMap::from([(Rank::Queen, 14), (Rank::King, 15), (Rank::Ace, 16)]),
            trips_cards: 17,
            stay_top_trips: true,
            stay_bottom_quads_or_better: true,
        }
    }

    /// 全突入条件の配布枚数を一律にする(非 progressive のローカルルール)。
    pub fn set_uniform_cards(&mut self, cards: u8) {
        for value in self.pair_cards.values_mut() {
            *value = cards;
        }
        self.trips_cards = cards;
    }
}

/// FL 突入判定。突入するなら配布枚数を返す。ファウル盤面は突入しない。
pub fn fantasyland_entry(
    board: &Board,
    rules: &FantasylandRules,
) -> Result<Option<u8>, FoulCheckError> {
    if check_foul(board)? {
        return Ok(None);
    }
    let top = evaluate_three(board.top()).map_err(FoulCheckError::Eval)?;
    let cards = match top.category {
        Category::Trips => Some(rules.trips_cards),
        Category::Pair => top
            .tiebreak
            .first()
            .and_then(|rank| rules.pair_cards.get(rank))
            .copied(),
        _ => None,
    };
    Ok(cards)
}

/// FL 継続判定。ファウル盤面は継続しない。
pub fn fantasyland_stay(board: &Board, rules: &FantasylandRules) -> Result<bool, FoulCheckError> {
    if check_foul(board)? {
        return Ok(false);
    }
    if rules.stay_top_trips {
        let top = evaluate_three(board.top()).map_err(FoulCheckError::Eval)?;
        if top.category == Category::Trips {
            return Ok(true);
        }
    }
    if rules.stay_bottom_quads_or_better {
        let bottom = evaluate_five(board.bottom()).map_err(FoulCheckError::Eval)?;
        if bottom.category >= Category::Quads {
            return Ok(true);
        }
    }
    Ok(false)
}
