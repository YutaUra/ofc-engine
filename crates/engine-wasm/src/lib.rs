//! wasm 配布層。JSON 文字列 API のみを公開する(ADR 0003)。
//! 失敗は panic ではなく {"error": "..."} の JSON で返す —
//! wasm 境界を越える panic は利用側で原因追跡が困難になるため。

use serde::Serialize;
use wasm_bindgen::prelude::*;

use ofc_engine::evaluate::{BoardEvaluation, evaluate_board};
use ofc_engine::hand::{Category, HandRank};
use ofc_engine::ruleset::RuleSet;
use ofc_engine::scoring::score_matchup;
use ofc_engine::{Board, Card, Rank};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EvalOut {
    foul: bool,
    top: RowOut,
    middle: RowOut,
    bottom: RowOut,
    royalty_total: u32,
    fantasyland_cards: Option<u8>,
    resolved: Board,
}

#[derive(Serialize)]
struct RowOut {
    hand: HandOut,
    royalty: u32,
}

#[derive(Serialize)]
struct HandOut {
    category: Category,
    tiebreak: Vec<Rank>,
}

#[derive(Serialize)]
struct ErrorOut {
    error: String,
}

#[derive(Serialize)]
struct TotalsOut {
    totals: Vec<i32>,
}

/// 標準 Pineapple ルールセットの JSON。利用側はこれを部分編集して
/// ローカルルールを作れる。
#[wasm_bindgen]
pub fn standard_ruleset_json() -> String {
    serde_json::to_string(&RuleSet::standard_pineapple()).expect("プリセットは常に直列化可能")
}

/// 盤面評価。入力はすべて JSON 文字列(board / 使用済みカード配列 / RuleSet)。
#[wasm_bindgen]
pub fn evaluate_board_json(board_json: &str, used_json: &str, ruleset_json: &str) -> String {
    to_json(evaluate_impl(board_json, used_json, ruleset_json))
}

/// 総当たり採点。boards は Board の JSON 配列。
#[wasm_bindgen]
pub fn score_matchup_json(boards_json: &str, ruleset_json: &str) -> String {
    to_json(matchup_impl(boards_json, ruleset_json))
}

fn evaluate_impl(board_json: &str, used_json: &str, ruleset_json: &str) -> Result<EvalOut, String> {
    let board: Board = serde_json::from_str(board_json).map_err(|e| format!("board: {e}"))?;
    let used: Vec<Card> = serde_json::from_str(used_json).map_err(|e| format!("used: {e}"))?;
    let ruleset: RuleSet =
        serde_json::from_str(ruleset_json).map_err(|e| format!("ruleset: {e}"))?;
    let compiled = ruleset.compile().map_err(|e| format!("ruleset: {e:?}"))?;

    let result = evaluate_board(&board, &used, &compiled.royalty, &compiled.fantasyland)
        .map_err(|e| format!("{e:?}"))?;
    Ok(eval_out(result))
}

fn matchup_impl(boards_json: &str, ruleset_json: &str) -> Result<TotalsOut, String> {
    let boards: Vec<Board> =
        serde_json::from_str(boards_json).map_err(|e| format!("boards: {e}"))?;
    let ruleset: RuleSet =
        serde_json::from_str(ruleset_json).map_err(|e| format!("ruleset: {e}"))?;
    let compiled = ruleset.compile().map_err(|e| format!("ruleset: {e:?}"))?;

    let totals = score_matchup(&boards, &compiled.royalty, &compiled.scoring)
        .map_err(|e| format!("{e:?}"))?;
    Ok(TotalsOut { totals })
}

fn eval_out(result: BoardEvaluation) -> EvalOut {
    let row = |hand: HandRank, royalty: u32| RowOut {
        hand: HandOut {
            category: hand.category,
            tiebreak: hand.tiebreak,
        },
        royalty,
    };
    EvalOut {
        foul: result.foul,
        top: row(result.top.hand, result.top.royalty),
        middle: row(result.middle.hand, result.middle.royalty),
        bottom: row(result.bottom.hand, result.bottom.royalty),
        royalty_total: result.royalty_total,
        fantasyland_cards: result.fantasyland_cards,
        resolved: result.resolved,
    }
}

fn to_json<T: Serialize>(result: Result<T, String>) -> String {
    let json = match result {
        Ok(value) => serde_json::to_string(&value),
        Err(error) => serde_json::to_string(&ErrorOut { error }),
    };
    json.unwrap_or_else(|e| format!(r#"{{"error":"serialization failed: {e}"}}"#))
}
