//! Joker 解決(行単位)。行内の Joker を役が最強になる標準カードへ置き換える。
//!
//! 全候補の総当たり(1 枚: 最大 52 通り / 2 枚: 最大 1,326 通り)で実装している。
//! ビット表現による高速化はソルバー統合時の課題とし、まず正しさを単純な
//! 実装で固定する(このモジュールがホットループに入る場合は要最適化)。

use crate::hand::{EvalError, HandRank, evaluate_five, evaluate_three};
use crate::{Card, Rank, Suit};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RowArity {
    Three,
    Five,
}

/// Joker 解決後の行。cards に Joker は残らない。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedRow {
    pub cards: Vec<Card>,
    pub hand: HandRank,
}

/// 行内の Joker を最強の役になるよう解決する。
/// `used` にはこの行以外で使用済みのカード(他の行・捨て札など)を渡す。
/// 解決先の候補から、行内カードと `used` は除外される。
pub fn resolve_row(
    cards: &[Card],
    arity: RowArity,
    used: &[Card],
) -> Result<ResolvedRow, EvalError> {
    let expected = match arity {
        RowArity::Three => 3,
        RowArity::Five => 5,
    };
    if cards.len() != expected {
        return Err(EvalError::WrongCardCount {
            expected,
            actual: cards.len(),
        });
    }

    let evaluate = |cards: &[Card]| match arity {
        RowArity::Three => evaluate_three(cards),
        RowArity::Five => evaluate_five(cards),
    };

    let joker_positions: Vec<usize> = cards
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Card::Joker))
        .map(|(i, _)| i)
        .collect();

    if joker_positions.is_empty() {
        let hand = evaluate(cards)?;
        return Ok(ResolvedRow {
            cards: cards.to_vec(),
            hand,
        });
    }

    let candidates = available_cards(cards, used);
    let mut best: Option<ResolvedRow> = None;
    let mut trial = cards.to_vec();

    // Joker 数は最大 2(デッキ設定の上限)なので総当たりで十分
    match joker_positions.as_slice() {
        [p0] => {
            for &candidate in &candidates {
                trial[*p0] = candidate;
                consider(&mut best, &trial, &evaluate)?;
            }
        }
        [p0, p1] => {
            for (i, &c0) in candidates.iter().enumerate() {
                for &c1 in &candidates[(i + 1)..] {
                    trial[*p0] = c0;
                    trial[*p1] = c1;
                    consider(&mut best, &trial, &evaluate)?;
                    // 並び順で役は変わらないため逆順は試さない
                }
            }
        }
        _ => return Err(EvalError::UnresolvedJoker),
    }

    best.ok_or(EvalError::UnresolvedJoker)
}

fn consider(
    best: &mut Option<ResolvedRow>,
    trial: &[Card],
    evaluate: &impl Fn(&[Card]) -> Result<HandRank, EvalError>,
) -> Result<(), EvalError> {
    let hand = evaluate(trial)?;
    if best.as_ref().is_none_or(|b| hand > b.hand) {
        *best = Some(ResolvedRow {
            cards: trial.to_vec(),
            hand,
        });
    }
    Ok(())
}

/// 52 枚から行内カードと使用済みカードを除いた解決先候補。
fn available_cards(row: &[Card], used: &[Card]) -> Vec<Card> {
    let ranks = [
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
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    let taken: std::collections::HashSet<Card> = row.iter().chain(used).copied().collect();
    ranks
        .into_iter()
        .flat_map(|rank| suits.map(|suit| Card::Standard { rank, suit }))
        .filter(|card| !taken.contains(card))
        .collect()
}
