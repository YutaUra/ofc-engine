//! 対戦採点(1-6 方式)。行勝敗 + scoop + ロイヤリティ差分 + ファウルペナルティ。

use crate::Board;
use crate::foul::FoulCheckError;
use crate::royalty::RoyaltyTable;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScoringRules {
    pub row_point: i32,
    pub scoop_bonus: i32,
}

impl ScoringRules {
    /// 1-6 方式の標準(行 1 点 + scoop 3 点)。
    pub fn standard() -> Self {
        Self {
            row_point: 1,
            scoop_bonus: 3,
        }
    }
}

/// 2 者間の採点。戻り値は (a の得点, b の得点) でゼロサム。
pub fn score_pair(
    a: &Board,
    b: &Board,
    royalty: &RoyaltyTable,
    scoring: &ScoringRules,
) -> Result<(i32, i32), FoulCheckError> {
    if !a.is_complete() || !b.is_complete() {
        return Err(FoulCheckError::IncompleteBoard);
    }
    // 行評価は 1 盤面につき 1 回だけ行い、ファウル判定・行勝敗・ロイヤリティを
    // すべてその結果から導く(評価の重複はホットループで効くため)。
    let rows_a = evaluate_rows(a)?;
    let rows_b = evaluate_rows(b)?;
    let a_foul = is_foul(&rows_a);
    let b_foul = is_foul(&rows_b);

    let points_for_a = match (a_foul, b_foul) {
        (true, true) => 0,
        // ファウル側は全行負け + scoop を取られ、自分のロイヤリティは 0
        (true, false) => {
            -(3 * scoring.row_point + scoring.scoop_bonus + rows_royalty(&rows_b, royalty))
        }
        (false, true) => {
            3 * scoring.row_point + scoring.scoop_bonus + rows_royalty(&rows_a, royalty)
        }
        (false, false) => {
            let mut wins_a = 0;
            let mut wins_b = 0;
            for (ra, rb) in rows_a.iter().zip(&rows_b) {
                match ra.cmp(rb) {
                    std::cmp::Ordering::Greater => wins_a += 1,
                    std::cmp::Ordering::Less => wins_b += 1,
                    std::cmp::Ordering::Equal => {}
                }
            }
            let mut points = (wins_a - wins_b) * scoring.row_point;
            if wins_a == 3 {
                points += scoring.scoop_bonus;
            }
            if wins_b == 3 {
                points -= scoring.scoop_bonus;
            }
            points + rows_royalty(&rows_a, royalty) - rows_royalty(&rows_b, royalty)
        }
    };
    Ok((points_for_a, -points_for_a))
}

/// 総当たり採点。各プレイヤーの得点は全ペア採点の和。
pub fn score_matchup(
    boards: &[Board],
    royalty: &RoyaltyTable,
    scoring: &ScoringRules,
) -> Result<Vec<i32>, FoulCheckError> {
    let mut totals = vec![0i32; boards.len()];
    for i in 0..boards.len() {
        for j in (i + 1)..boards.len() {
            let (pi, pj) = score_pair(&boards[i], &boards[j], royalty, scoring)?;
            totals[i] += pi;
            totals[j] += pj;
        }
    }
    Ok(totals)
}

fn evaluate_rows(board: &Board) -> Result<[crate::hand::HandRank; 3], FoulCheckError> {
    Ok([
        crate::hand::evaluate_three(board.top()).map_err(FoulCheckError::Eval)?,
        crate::hand::evaluate_five(board.middle()).map_err(FoulCheckError::Eval)?,
        crate::hand::evaluate_five(board.bottom()).map_err(FoulCheckError::Eval)?,
    ])
}

/// 行評価は [top, middle, bottom] の順(evaluate_rows と一致させること)。
fn is_foul(rows: &[crate::hand::HandRank; 3]) -> bool {
    let [top, middle, bottom] = rows;
    middle < top || bottom < middle
}

fn rows_royalty(rows: &[crate::hand::HandRank; 3], table: &RoyaltyTable) -> i32 {
    use crate::royalty::{Row, royalty_points};
    let [top, middle, bottom] = rows;
    (royalty_points(Row::Top, top, table)
        + royalty_points(Row::Middle, middle, table)
        + royalty_points(Row::Bottom, bottom, table)) as i32
}
