//! 役評価。Joker 解決済みの手札を前提とする(解決は別モジュールの責務)。

use crate::{Card, Rank};

/// 役カテゴリ。wire では安定キー(`"straight_flush"` 等)として渡す(ADR 0003)。
/// RoyalFlush を独立カテゴリにする理由: ロイヤリティ表で
/// ストレートフラッシュと点が異なり、表引きのキーとして区別が必要なため。
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Category {
    HighCard,
    Pair,
    TwoPair,
    Trips,
    Straight,
    Flush,
    FullHouse,
    Quads,
    StraightFlush,
    RoyalFlush,
}

/// 役の強さ。カテゴリ→キッカー列の辞書式順序で比較できる。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct HandRank {
    pub category: Category,
    /// 同カテゴリ内の比較キー。強い順に並べたランク列
    /// (例: ツーペアなら [上ペア, 下ペア, キッカー])。
    pub tiebreak: Vec<Rank>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvalError {
    WrongCardCount { expected: usize, actual: usize },
    UnresolvedJoker,
}

/// ランク出現数(添字 = Rank の判別値)とランク集合のビットマスク。
/// BTreeMap やソートを使わないのは、この関数がソルバーの最内ループで
/// 呼ばれるため(ヒープ確保とキャッシュミスを避ける)。
struct RankProfile {
    counts: [u8; 13],
    mask: u16,
    is_flush: bool,
}

fn profile(cards: &[Card], expected: usize) -> Result<RankProfile, EvalError> {
    if cards.len() != expected {
        return Err(EvalError::WrongCardCount {
            expected,
            actual: cards.len(),
        });
    }
    let mut counts = [0u8; 13];
    let mut mask = 0u16;
    let mut suits = 0u8;
    for card in cards {
        match card {
            Card::Standard { rank, suit } => {
                let r = *rank as usize;
                counts[r] += 1;
                mask |= 1 << r;
                suits |= 1 << (*suit as u8);
            }
            Card::Joker => return Err(EvalError::UnresolvedJoker),
        }
    }
    Ok(RankProfile {
        counts,
        mask,
        is_flush: suits.count_ones() == 1,
    })
}

/// 出現数 `count` のランクを強い順に tiebreak へ追加する。
fn push_ranks(tiebreak: &mut Vec<Rank>, counts: &[u8; 13], count: u8) {
    for r in (0..13).rev() {
        if counts[r] == count {
            tiebreak.push(Rank::ALL[r]);
        }
    }
}

pub fn evaluate_five(cards: &[Card]) -> Result<HandRank, EvalError> {
    let p = profile(cards, 5)?;

    // ストレート判定: 5 種のランクが連続しているか、ホイール(A-2-3-4-5)か
    let straight_high = if p.mask.count_ones() == 5 {
        const WHEEL: u16 = 0b1_0000_0000_1111; // A + 2,3,4,5
        let low = p.mask.trailing_zeros() as usize;
        if p.mask >> low == 0b11111 {
            Some(Rank::ALL[low + 4])
        } else if p.mask == WHEEL {
            Some(Rank::Five) // ホイールは 5 ハイ扱い
        } else {
            None
        }
    } else {
        None
    };

    let distinct = p.mask.count_ones();
    let max_count = *p.counts.iter().max().expect("13 要素で空にならない");

    let mut tiebreak = Vec::with_capacity(5);
    let category = match (straight_high, p.is_flush) {
        (Some(Rank::Ace), true) => Category::RoyalFlush,
        (Some(high), true) => {
            tiebreak.push(high);
            Category::StraightFlush
        }
        (_, true) => {
            push_ranks(&mut tiebreak, &p.counts, 1);
            Category::Flush
        }
        (Some(high), false) => {
            tiebreak.push(high);
            Category::Straight
        }
        (None, false) => match (max_count, distinct) {
            (4, _) => {
                push_ranks(&mut tiebreak, &p.counts, 4);
                push_ranks(&mut tiebreak, &p.counts, 1);
                Category::Quads
            }
            (3, 2) => {
                push_ranks(&mut tiebreak, &p.counts, 3);
                push_ranks(&mut tiebreak, &p.counts, 2);
                Category::FullHouse
            }
            (3, _) => {
                push_ranks(&mut tiebreak, &p.counts, 3);
                push_ranks(&mut tiebreak, &p.counts, 1);
                Category::Trips
            }
            (2, 3) => {
                push_ranks(&mut tiebreak, &p.counts, 2);
                push_ranks(&mut tiebreak, &p.counts, 1);
                Category::TwoPair
            }
            (2, _) => {
                push_ranks(&mut tiebreak, &p.counts, 2);
                push_ranks(&mut tiebreak, &p.counts, 1);
                Category::Pair
            }
            _ => {
                push_ranks(&mut tiebreak, &p.counts, 1);
                Category::HighCard
            }
        },
    };
    Ok(HandRank { category, tiebreak })
}

pub fn evaluate_three(cards: &[Card]) -> Result<HandRank, EvalError> {
    // top はストレート/フラッシュを役として扱わないため、ランクの出現数のみで決まる
    let p = profile(cards, 3)?;
    let max_count = *p.counts.iter().max().expect("13 要素で空にならない");

    let mut tiebreak = Vec::with_capacity(3);
    let category = match max_count {
        3 => {
            push_ranks(&mut tiebreak, &p.counts, 3);
            Category::Trips
        }
        2 => {
            push_ranks(&mut tiebreak, &p.counts, 2);
            push_ranks(&mut tiebreak, &p.counts, 1);
            Category::Pair
        }
        _ => {
            push_ranks(&mut tiebreak, &p.counts, 1);
            Category::HighCard
        }
    };
    Ok(HandRank { category, tiebreak })
}
