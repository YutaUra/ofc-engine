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

/// 行単位の評価キャッシュを使った Joker 解決。
/// 盤面全体を毎回評価し直すのではなく、Joker を含む行だけを再評価する。
/// 特に Joker 2 枚が別々の行にある場合、全割り当て(≤2,652 通り)の評価を
/// 「行ごとの候補評価(≤ 候補数 × 2 回)+ キャッシュ済みキーの組み合わせ比較」
/// に落とす。オラクルテスト(joker_oracle)で総当たり参照実装との一致を保証。
/// (役, ロイヤリティ) の行評価キャッシュ。
type RowEval = (HandRank, u32);
/// 解決候補の優先度: ファウル回避 > ロイヤリティ合計 > 行の強さ(bottom, middle, top)。
type SelectionKey = (bool, u32, [HandRank; 3]);

fn resolve_best(
    board: &Board,
    slots: &[(usize, usize)],
    candidates: &[Card],
    royalty: &RoyaltyTable,
) -> Result<Board, EvaluateError> {
    let rows: [&[Card]; 3] = [board.top(), board.middle(), board.bottom()];

    let eval_row = |row_idx: usize, cards: &[Card]| -> Result<RowEval, EvaluateError> {
        let hand = match row_idx {
            0 => evaluate_three(cards),
            _ => evaluate_five(cards),
        }
        .map_err(EvaluateError::Eval)?;
        let row = [Row::Top, Row::Middle, Row::Bottom][row_idx];
        let points = royalty_points(row, &hand, royalty);
        Ok((hand, points))
    };

    // Joker を含まない行は 1 回だけ評価して固定する
    let joker_rows: Vec<usize> = slots.iter().map(|(row, _)| *row).collect();
    let mut fixed: [Option<RowEval>; 3] = [None, None, None];
    for (idx, row) in rows.iter().enumerate() {
        if !joker_rows.contains(&idx) {
            fixed[idx] = Some(eval_row(idx, row)?);
        }
    }

    // (foul 回避, royalty 合計, 行の強さ) の優先度キーで最良の割り当てを選ぶ。
    // HandRank の clone(ヒープ確保)は「最良を更新したときだけ」に限定する。
    // 大半の候補は (foul, royalty) か最初の行比較で負けるため、比較は参照で行う。
    let mut best: Option<(SelectionKey, Vec<Card>)> = None;
    let mut consider = |top: &RowEval, middle: &RowEval, bottom: &RowEval, assignment: &[Card]| {
        let foul = middle.0 < top.0 || bottom.0 < middle.0;
        let total = if foul { 0 } else { top.1 + middle.1 + bottom.1 };
        let improved = match &best {
            None => true,
            Some(((best_not_foul, best_total, best_hands), _)) => {
                (!foul, total, [&bottom.0, &middle.0, &top.0])
                    > (
                        *best_not_foul,
                        *best_total,
                        [&best_hands[0], &best_hands[1], &best_hands[2]],
                    )
            }
        };
        if improved {
            let key = (
                !foul,
                total,
                [bottom.0.clone(), middle.0.clone(), top.0.clone()],
            );
            best = Some((key, assignment.to_vec()));
        }
    };

    match slots {
        [(r0, p0)] => {
            let mut row = rows[*r0].to_vec();
            for &c in candidates {
                row[*p0] = c;
                let evaluated = eval_row(*r0, &row)?;
                let get = |idx: usize| -> &RowEval {
                    if idx == *r0 {
                        &evaluated
                    } else {
                        fixed[idx].as_ref().expect("Joker なし行は評価済み")
                    }
                };
                consider(get(0), get(1), get(2), &[c]);
            }
        }
        [(r0, p0), (r1, p1)] if r0 == r1 => {
            // 同一行内の 2 枚: 行内の並びは役に影響しないため非順序ペアで足りる
            let mut row = rows[*r0].to_vec();
            for (i, &c0) in candidates.iter().enumerate() {
                for &c1 in &candidates[(i + 1)..] {
                    row[*p0] = c0;
                    row[*p1] = c1;
                    let evaluated = eval_row(*r0, &row)?;
                    let get = |idx: usize| -> &RowEval {
                        if idx == *r0 {
                            &evaluated
                        } else {
                            fixed[idx].as_ref().expect("Joker なし行は評価済み")
                        }
                    };
                    consider(get(0), get(1), get(2), &[c0, c1]);
                }
            }
        }
        [(r0, p0), (r1, p1)] => {
            // 別々の行: 行ごとに候補別評価を先に作り、組み合わせはキャッシュ参照のみ
            let per_row = |row_idx: usize, pos: usize| -> Result<Vec<RowEval>, EvaluateError> {
                let mut row = rows[row_idx].to_vec();
                candidates
                    .iter()
                    .map(|&c| {
                        row[pos] = c;
                        eval_row(row_idx, &row)
                    })
                    .collect()
            };
            let evals0 = per_row(*r0, *p0)?;
            let evals1 = per_row(*r1, *p1)?;
            for (i, e0) in evals0.iter().enumerate() {
                for (j, e1) in evals1.iter().enumerate() {
                    if i == j {
                        continue; // 同一カードを 2 枚の Joker に割り当てることはできない
                    }
                    let get = |idx: usize| -> &RowEval {
                        if idx == *r0 {
                            e0
                        } else if idx == *r1 {
                            e1
                        } else {
                            fixed[idx].as_ref().expect("Joker なし行は評価済み")
                        }
                    };
                    consider(get(0), get(1), get(2), &[candidates[i], candidates[j]]);
                }
            }
        }
        _ => {
            // デッキ設定の上限は Joker 2 枚(ADR 0003)。3 枚以上は未対応
            unimplemented!("Joker {} 枚の盤面は未対応(デッキ上限は 2 枚)", slots.len())
        }
    }

    let (_, assignment) = best.expect("候補が空になることはない(52 枚 - 盤面 13 枚 > 0)");
    Ok(with_assignment(board, slots, &assignment))
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
