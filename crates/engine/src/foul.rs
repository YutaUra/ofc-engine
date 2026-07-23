//! ファウル判定。OFC の配置制約 bottom ≥ middle ≥ top を検査する。

use crate::Board;
use crate::hand::EvalError;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FoulCheckError {
    IncompleteBoard,
    Eval(EvalError),
}

pub fn check_foul(board: &Board) -> Result<bool, FoulCheckError> {
    if !board.is_complete() {
        return Err(FoulCheckError::IncompleteBoard);
    }
    let top = crate::hand::evaluate_three(board.top()).map_err(FoulCheckError::Eval)?;
    let middle = crate::hand::evaluate_five(board.middle()).map_err(FoulCheckError::Eval)?;
    let bottom = crate::hand::evaluate_five(board.bottom()).map_err(FoulCheckError::Eval)?;

    // HandRank の辞書式順序は 3 枚役と 5 枚役の間でも成立する
    // (カテゴリ→キッカー列の共通接頭辞比較。top にはストレート等が
    // 出現しないため、カテゴリ空間は共有できる)。
    Ok(middle < top || bottom < middle)
}
