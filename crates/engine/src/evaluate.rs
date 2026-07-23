//! 盤面評価のエントリポイント。役 + ロイヤリティ + ファウル + FL 判定 +
//! Joker の盤面全体最適解決を一括で返す(ADR 0003 の wire 境界に対応する層)。

use crate::fantasyland::{FantasylandRules, fantasyland_entry};
use crate::hand::{EvalError, HandRank, evaluate_five, evaluate_three};
use crate::joker::available_cards;
use crate::royalty::{Row, RoyaltyTable, royalty_points};
use crate::{Board, Card};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RowEvaluation {
    pub hand: HandRank,
    pub royalty: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BoardEvaluation {
    pub foul: bool,
    pub top: RowEvaluation,
    pub middle: RowEvaluation,
    pub bottom: RowEvaluation,
    /// ファウル時は 0。
    pub royalty_total: u32,
    /// FL 突入時の配布枚数。突入しない/ファウル時は None。
    pub fantasyland_cards: Option<u8>,
    /// Joker 解決後の盤面。
    pub resolved: Board,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluateError {
    IncompleteBoard,
    Eval(EvalError),
}

/// 完成盤面を評価する。Joker は「ファウル回避 > ロイヤリティ合計 >
/// 行の強さ(bottom, middle, top)」の優先順で盤面全体最適に解決する
/// (ADR 0003 の既定セマンティクス。弱める解決によるファウル回避を含む)。
/// `used` は盤面外の使用済みカード(捨て札・相手の公開カードなど)。
pub fn evaluate_board(
    board: &Board,
    used: &[Card],
    royalty: &RoyaltyTable,
    fl: &FantasylandRules,
) -> Result<BoardEvaluation, EvaluateError> {
    if !board.is_complete() {
        return Err(EvaluateError::IncompleteBoard);
    }

    let joker_slots = joker_positions(board);
    let best_board = match joker_slots.as_slice() {
        [] => board.clone(),
        slots => {
            let board_cards: Vec<Card> = board
                .top()
                .iter()
                .chain(board.middle())
                .chain(board.bottom())
                .copied()
                .collect();
            let candidates = available_cards(&board_cards, used);
            resolve_best(board, slots, &candidates, royalty)?
        }
    };

    evaluation_of(&best_board, royalty, fl)
}

/// (行番号 0=top/1=middle/2=bottom, 行内位置) で Joker の場所を列挙する。
fn joker_positions(board: &Board) -> Vec<(usize, usize)> {
    [board.top(), board.middle(), board.bottom()]
        .into_iter()
        .enumerate()
        .flat_map(|(row, cards)| {
            cards
                .iter()
                .enumerate()
                .filter(|(_, c)| matches!(c, Card::Joker))
                .map(move |(pos, _)| (row, pos))
                .collect::<Vec<_>>()
        })
        .collect()
}

fn resolve_best(
    board: &Board,
    slots: &[(usize, usize)],
    candidates: &[Card],
    royalty: &RoyaltyTable,
) -> Result<Board, EvaluateError> {
    let mut best: Option<(SelectionKey, Board)> = None;
    let mut consider = |assignment: &[Card]| -> Result<(), EvaluateError> {
        let trial = with_assignment(board, slots, assignment);
        let key = selection_key(&trial, royalty)?;
        if best.as_ref().is_none_or(|(bk, _)| key > *bk) {
            best = Some((key, trial));
        }
        Ok(())
    };

    match slots.len() {
        1 => {
            for &c in candidates {
                consider(&[c])?;
            }
        }
        2 => {
            // Joker が別の行にあると割り当ての入れ替えで結果が変わるため、
            // 単一行の解決と違い順序付きペアを総当たりする。
            for (i, &c0) in candidates.iter().enumerate() {
                for (j, &c1) in candidates.iter().enumerate() {
                    if i != j {
                        consider(&[c0, c1])?;
                    }
                }
            }
        }
        n => {
            // デッキ設定の上限は Joker 2 枚(ADR 0003)。3 枚以上は未対応
            unimplemented!("Joker {n} 枚の盤面は未対応(デッキ上限は 2 枚)")
        }
    }

    Ok(best
        .expect("候補が空になることはない(52 枚 - 盤面 13 枚 > 0)")
        .1)
}

fn with_assignment(board: &Board, slots: &[(usize, usize)], assignment: &[Card]) -> Board {
    let mut rows = [
        board.top().to_vec(),
        board.middle().to_vec(),
        board.bottom().to_vec(),
    ];
    for ((row, pos), card) in slots.iter().zip(assignment) {
        rows[*row][*pos] = *card;
    }
    let [top, middle, bottom] = rows;
    Board::new(top, middle, bottom).expect("解決候補は盤面と重複しないため常に有効")
}

/// 解決候補の優先度: ファウル回避 > ロイヤリティ合計 > 行の強さ(bottom, middle, top)。
type SelectionKey = (bool, u32, [HandRank; 3]);

fn selection_key(board: &Board, royalty: &RoyaltyTable) -> Result<SelectionKey, EvaluateError> {
    let (top, middle, bottom) = row_hands(board)?;
    let foul = middle < top || bottom < middle;
    // 行の強さだけでなく実際の表のロイヤリティ合計をキーに含める。
    // 強さと点数はおおむね同調するが、ローカルルールの表では特定役だけ
    // 高得点になる逆転がありうるため、渡された表で測る。
    let total = if foul {
        0
    } else {
        royalty_points(Row::Top, &top, royalty)
            + royalty_points(Row::Middle, &middle, royalty)
            + royalty_points(Row::Bottom, &bottom, royalty)
    };
    Ok((!foul, total, [bottom, middle, top]))
}

fn row_hands(board: &Board) -> Result<(HandRank, HandRank, HandRank), EvaluateError> {
    Ok((
        evaluate_three(board.top()).map_err(EvaluateError::Eval)?,
        evaluate_five(board.middle()).map_err(EvaluateError::Eval)?,
        evaluate_five(board.bottom()).map_err(EvaluateError::Eval)?,
    ))
}

fn evaluation_of(
    board: &Board,
    royalty: &RoyaltyTable,
    fl: &FantasylandRules,
) -> Result<BoardEvaluation, EvaluateError> {
    let (top, middle, bottom) = row_hands(board)?;
    let foul = middle < top || bottom < middle;

    let top = RowEvaluation {
        royalty: royalty_points(Row::Top, &top, royalty),
        hand: top,
    };
    let middle = RowEvaluation {
        royalty: royalty_points(Row::Middle, &middle, royalty),
        hand: middle,
    };
    let bottom = RowEvaluation {
        royalty: royalty_points(Row::Bottom, &bottom, royalty),
        hand: bottom,
    };

    let royalty_total = if foul {
        0
    } else {
        top.royalty + middle.royalty + bottom.royalty
    };
    let fantasyland_cards = fantasyland_entry(board, fl).map_err(|e| match e {
        crate::foul::FoulCheckError::IncompleteBoard => EvaluateError::IncompleteBoard,
        crate::foul::FoulCheckError::Eval(e) => EvaluateError::Eval(e),
    })?;

    Ok(BoardEvaluation {
        foul,
        top,
        middle,
        bottom,
        royalty_total,
        fantasyland_cards,
        resolved: board.clone(),
    })
}
