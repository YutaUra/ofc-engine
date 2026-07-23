//! 役評価。Joker 解決済みの手札を前提とする(解決は別モジュールの責務)。

use crate::{Card, Rank};

/// 役カテゴリ。wire では安定キー(`"straight_flush"` 等)として渡す(ADR 0003)。
/// RoyalFlush を独立カテゴリにする理由: ロイヤリティ表で
/// ストレートフラッシュと点が異なり、表引きのキーとして区別が必要なため。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
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

pub fn evaluate_five(cards: &[Card]) -> Result<HandRank, EvalError> {
    let (ranks, suits) = resolved_ranks(cards, 5)?;

    let groups = rank_groups(&ranks);
    let is_flush = suits.windows(2).all(|w| w[0] == w[1]);
    let straight_high = straight_high(&ranks);

    let hand = match (straight_high, is_flush, groups.as_slice()) {
        (Some(Rank::Ace), true, _) => HandRank {
            category: Category::RoyalFlush,
            tiebreak: vec![],
        },
        (Some(high), true, _) => HandRank {
            category: Category::StraightFlush,
            tiebreak: vec![high],
        },
        (_, _, [(4, quad), (1, kicker)]) => HandRank {
            category: Category::Quads,
            tiebreak: vec![*quad, *kicker],
        },
        (_, _, [(3, trips), (2, pair)]) => HandRank {
            category: Category::FullHouse,
            tiebreak: vec![*trips, *pair],
        },
        (_, true, _) => HandRank {
            category: Category::Flush,
            tiebreak: ranks_desc(&ranks),
        },
        (Some(high), false, _) => HandRank {
            category: Category::Straight,
            tiebreak: vec![high],
        },
        (_, _, [(3, trips), (1, k1), (1, k2)]) => HandRank {
            category: Category::Trips,
            tiebreak: vec![*trips, *k1, *k2],
        },
        (_, _, [(2, high_pair), (2, low_pair), (1, kicker)]) => HandRank {
            category: Category::TwoPair,
            tiebreak: vec![*high_pair, *low_pair, *kicker],
        },
        (_, _, [(2, pair), rest @ ..]) => HandRank {
            category: Category::Pair,
            tiebreak: std::iter::once(*pair)
                .chain(rest.iter().map(|(_, r)| *r))
                .collect(),
        },
        _ => HandRank {
            category: Category::HighCard,
            tiebreak: ranks_desc(&ranks),
        },
    };
    Ok(hand)
}

pub fn evaluate_three(cards: &[Card]) -> Result<HandRank, EvalError> {
    // top はストレート/フラッシュを役として扱わないため、ランクのグループ化のみで決まる
    let (ranks, _) = resolved_ranks(cards, 3)?;
    let groups = rank_groups(&ranks);
    let hand = match groups.as_slice() {
        [(3, trips)] => HandRank {
            category: Category::Trips,
            tiebreak: vec![*trips],
        },
        [(2, pair), (1, kicker)] => HandRank {
            category: Category::Pair,
            tiebreak: vec![*pair, *kicker],
        },
        _ => HandRank {
            category: Category::HighCard,
            tiebreak: ranks_desc(&ranks),
        },
    };
    Ok(hand)
}

/// Joker が残っていないことを検証しつつランクとスートに分解する。
fn resolved_ranks(
    cards: &[Card],
    expected: usize,
) -> Result<(Vec<Rank>, Vec<crate::Suit>), EvalError> {
    if cards.len() != expected {
        return Err(EvalError::WrongCardCount {
            expected,
            actual: cards.len(),
        });
    }
    cards
        .iter()
        .map(|card| match card {
            Card::Standard { rank, suit } => Ok((*rank, *suit)),
            Card::Joker => Err(EvalError::UnresolvedJoker),
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|pairs| pairs.into_iter().unzip())
}

/// (出現数, ランク) を出現数→ランクの降順で返す。役カテゴリはこの形で判別できる。
fn rank_groups(ranks: &[Rank]) -> Vec<(usize, Rank)> {
    let mut counts = std::collections::BTreeMap::new();
    for rank in ranks {
        *counts.entry(*rank).or_insert(0usize) += 1;
    }
    let mut groups: Vec<(usize, Rank)> = counts.into_iter().map(|(r, c)| (c, r)).collect();
    groups.sort_unstable_by(|a, b| b.cmp(a));
    groups
}

fn ranks_desc(ranks: &[Rank]) -> Vec<Rank> {
    let mut sorted = ranks.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    sorted
}

/// ストレートなら最高ランクを返す。ホイール(A-5)は 5 ハイとして扱う。
fn straight_high(ranks: &[Rank]) -> Option<Rank> {
    let mut sorted = ranks.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != 5 {
        return None;
    }
    let consecutive = sorted.windows(2).all(|w| w[1] as u8 - w[0] as u8 == 1);
    if consecutive {
        return Some(sorted[4]);
    }
    // ホイール: A,2,3,4,5。A を除いた 2..5 が連続し、最高位は 5
    if sorted == [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Ace] {
        return Some(Rank::Five);
    }
    None
}
