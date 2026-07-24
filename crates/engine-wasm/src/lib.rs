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

// ---- ゲーム進行(GameState)の JSON 文字列 API ----
// state はオペークな JSON 文字列として返し、利用側はそのまま保存・再投入する
// (中断復帰用途。deck = 未公開のカード順を含むため相手には渡さないこと)。
// UI 向けの読み取りは view として毎回添える。

use ofc_engine::game::{GameState, Placement};

#[derive(Serialize)]
struct GameOut {
    state: String,
    view: GameView,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct GameView {
    current_player: usize,
    street: StreetView,
    dealt_cards: Vec<Card>,
    boards: Vec<Board>,
}

#[derive(Serialize)]
struct StreetView {
    phase: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    number: Option<u8>,
}

/// ゲームを開始する。seed は文字列で受ける(JS の number は u64 を安全に
/// 表現できないため。BigInt でも数値でも toString して渡せばよい)。
#[wasm_bindgen]
pub fn game_new_json(players: u8, jokers: u8, seed: &str) -> String {
    to_json(game_new_impl(players, jokers, seed, None))
}

/// FL ハンドを含むゲームを開始する。fl_cards はプレイヤーごとの FL 配布枚数
/// の JSON 配列(0 = 通常。例: [14, 0])。
#[wasm_bindgen]
pub fn game_new_fl_json(players: u8, jokers: u8, seed: &str, fl_cards_json: &str) -> String {
    to_json(game_new_impl(players, jokers, seed, Some(fl_cards_json)))
}

/// 着手を適用する。placements は [{card, row}] の配列、discard は Card か null。
#[wasm_bindgen]
pub fn game_apply_json(state_json: &str, placements_json: &str, discard_json: &str) -> String {
    to_json(game_apply_impl(state_json, placements_json, discard_json))
}

/// 保存済み state から view を再構築する(中断復帰)。
#[wasm_bindgen]
pub fn game_view_json(state_json: &str) -> String {
    to_json(parse_state(state_json).map(|state| view_of(&state)))
}

fn game_new_impl(
    players: u8,
    jokers: u8,
    seed: &str,
    fl_cards_json: Option<&str>,
) -> Result<GameOut, String> {
    let seed: u64 = seed
        .trim()
        .parse()
        .map_err(|_| format!("seed: 数値文字列ではありません: {seed:?}"))?;
    let state = match fl_cards_json {
        None => GameState::new(players, jokers, seed),
        Some(json) => {
            let fl_cards: Vec<u8> =
                serde_json::from_str(json).map_err(|e| format!("flCards: {e}"))?;
            GameState::new_with_fantasyland(players, jokers, seed, &fl_cards)
        }
    }
    .map_err(|e| format!("{e:?}"))?;
    game_out(&state)
}

/// discard は後方互換のため Card 単体・null・Card 配列のすべてを受ける。
#[derive(serde::Deserialize)]
#[serde(untagged)]
enum DiscardWire {
    One(Option<Card>),
    Many(Vec<Card>),
}

fn game_apply_impl(
    state_json: &str,
    placements_json: &str,
    discard_json: &str,
) -> Result<GameOut, String> {
    let mut state = parse_state(state_json)?;
    let placements: Vec<Placement> =
        serde_json::from_str(placements_json).map_err(|e| format!("placements: {e}"))?;
    let discard: DiscardWire =
        serde_json::from_str(discard_json).map_err(|e| format!("discard: {e}"))?;
    let discards: Vec<Card> = match discard {
        DiscardWire::One(None) => vec![],
        DiscardWire::One(Some(card)) => vec![card],
        DiscardWire::Many(cards) => cards,
    };
    use ofc_engine::game::Street;
    if state.street() == Street::Fantasyland {
        state
            .apply_fantasyland(&placements, &discards)
            .map_err(|e| format!("{e:?}"))?;
    } else {
        state
            .apply(&placements, discards.first().copied())
            .map_err(|e| format!("{e:?}"))?;
    }
    game_out(&state)
}

fn parse_state(state_json: &str) -> Result<GameState, String> {
    serde_json::from_str(state_json).map_err(|e| format!("state: {e}"))
}

fn game_out(state: &GameState) -> Result<GameOut, String> {
    Ok(GameOut {
        state: serde_json::to_string(state).map_err(|e| format!("state: {e}"))?,
        view: view_of(state),
    })
}

fn view_of(state: &GameState) -> GameView {
    use ofc_engine::game::Street;
    let street = match state.street() {
        Street::Initial => StreetView {
            phase: "initial",
            number: None,
        },
        Street::Fantasyland => StreetView {
            phase: "fantasyland",
            number: None,
        },
        Street::Draw(n) => StreetView {
            phase: "draw",
            number: Some(n),
        },
        Street::Finished => StreetView {
            phase: "finished",
            number: None,
        },
    };
    GameView {
        current_player: state.current_player(),
        street,
        dealt_cards: state.dealt_cards().to_vec(),
        boards: state.boards().to_vec(),
    }
}
