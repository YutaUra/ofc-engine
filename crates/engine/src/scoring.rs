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
    let eval_a = BoardScore::new(a, royalty)?;
    let eval_b = BoardScore::new(b, royalty)?;
    let points_for_a = score_from(&eval_a, &eval_b, scoring);
    Ok((points_for_a, -points_for_a))
}

/// 採点に必要な盤面ごとの評価キャッシュ(行の役・ファウル・ロイヤリティ合計)。
struct BoardScore {
    rows: [crate::hand::HandRank; 3],
    foul: bool,
    royalty: i32,
}

impl BoardScore {
    fn new(board: &Board, table: &RoyaltyTable) -> Result<Self, FoulCheckError> {
        let rows = evaluate_rows(board)?;
        let foul = is_foul(&rows);
        let royalty = if foul { 0 } else { rows_royalty(&rows, table) };
        Ok(Self {
            rows,
            foul,
            royalty,
        })
    }
}

fn score_from(a: &BoardScore, b: &BoardScore, scoring: &ScoringRules) -> i32 {
    match (a.foul, b.foul) {
        (true, true) => 0,
        // ファウル側は全行負け + scoop を取られ、自分のロイヤリティは 0
        (true, false) => -(3 * scoring.row_point + scoring.scoop_bonus + b.royalty),
        (false, true) => 3 * scoring.row_point + scoring.scoop_bonus + a.royalty,
        (false, false) => {
            let mut wins_a = 0;
            let mut wins_b = 0;
            for (ra, rb) in a.rows.iter().zip(&b.rows) {
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
            points + a.royalty - b.royalty
        }
    }
}

/// 総当たり採点。各プレイヤーの得点は全ペア採点の和。
pub fn score_matchup(
    boards: &[Board],
    royalty: &RoyaltyTable,
    scoring: &ScoringRules,
) -> Result<Vec<i32>, FoulCheckError> {
    // 各盤面の評価は 1 回だけ行い、ペア採点はキャッシュ同士で行う
    // (素朴にペアごとに score_pair を呼ぶと各盤面が N-1 回評価される)。
    if boards.iter().any(|b| !b.is_complete()) {
        return Err(FoulCheckError::IncompleteBoard);
    }
    let evals: Vec<BoardScore> = boards
        .iter()
        .map(|b| BoardScore::new(b, royalty))
        .collect::<Result<_, _>>()?;
    let mut totals = vec![0i32; boards.len()];
    for i in 0..boards.len() {
        for j in (i + 1)..boards.len() {
            let p = score_from(&evals[i], &evals[j], scoring);
            totals[i] += p;
            totals[j] -= p;
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
