//! GameState の JSON 文字列 API のテスト。
//! 状態(state)はオペークな JSON 文字列として往復させ、UI 向けの
//! 読み取りは view(currentPlayer / street / dealtCards / boards)で返す。

use ofc_engine_wasm::{game_apply_json, game_new_json, game_view_json};

fn new_game() -> (String, serde_json::Value) {
    let out = game_new_json(2, 0, "42");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let state = parsed["state"].as_str().unwrap().to_string();
    (state, parsed["view"].clone())
}

#[test]
fn ゲームを生成すると初手5枚のviewが返る() {
    let (_, view) = new_game();
    assert_eq!(view["currentPlayer"], 0);
    assert_eq!(view["street"], serde_json::json!({ "phase": "initial" }));
    assert_eq!(view["dealtCards"].as_array().unwrap().len(), 5);
    assert_eq!(view["boards"].as_array().unwrap().len(), 2);
}

#[test]
fn 着手を適用すると次の手番に進む() {
    let (state, view) = new_game();
    let dealt: Vec<String> = view["dealtCards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    let placements = serde_json::json!([
        { "card": dealt[0], "row": "bottom" },
        { "card": dealt[1], "row": "bottom" },
        { "card": dealt[2], "row": "middle" },
        { "card": dealt[3], "row": "middle" },
        { "card": dealt[4], "row": "top" },
    ]);
    let out = game_apply_json(&state, &placements.to_string(), "null");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_null(), "{out}");
    assert_eq!(parsed["view"]["currentPlayer"], 1);
    // player 0 の盤面に反映されている
    assert_eq!(
        parsed["view"]["boards"][0]["bottom"],
        serde_json::json!([dealt[0], dealt[1]])
    );
}

#[test]
fn stateはオペークに保存して復元できる() {
    let (state, _) = new_game();
    // 保存しておいた state をそのまま view に渡せる(= 中断復帰)
    let out = game_view_json(&state);
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(parsed["street"], serde_json::json!({ "phase": "initial" }));
}

#[test]
fn 不正な入力はerrorを持つjsonで返る() {
    let out = game_new_json(9, 0, "1"); // デッキ不足
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_string());

    let out = game_apply_json("{", "[]", "null");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_string());

    let out = game_new_json(2, 0, "abc"); // seed が数値でない
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_string());
}

#[test]
fn 街では捨て札が必要でviewのstreetが進む() {
    let (mut state, mut view) = new_game();
    // 2 人分の初手を適用
    for _ in 0..2 {
        let dealt: Vec<String> = view["dealtCards"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c.as_str().unwrap().to_string())
            .collect();
        let placements = serde_json::json!([
            { "card": dealt[0], "row": "bottom" },
            { "card": dealt[1], "row": "bottom" },
            { "card": dealt[2], "row": "middle" },
            { "card": dealt[3], "row": "middle" },
            { "card": dealt[4], "row": "top" },
        ]);
        let out = game_apply_json(&state, &placements.to_string(), "null");
        let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
        state = parsed["state"].as_str().unwrap().to_string();
        view = parsed["view"].clone();
    }
    assert_eq!(
        view["street"],
        serde_json::json!({ "phase": "draw", "number": 1 })
    );
    assert_eq!(view["dealtCards"].as_array().unwrap().len(), 3);

    // 捨て札つきで 2 枚配置
    let dealt: Vec<String> = view["dealtCards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    let placements = serde_json::json!([
        { "card": dealt[0], "row": "bottom" },
        { "card": dealt[1], "row": "bottom" },
    ]);
    let out = game_apply_json(
        &state,
        &placements.to_string(),
        &serde_json::json!(dealt[2]).to_string(),
    );
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_null(), "{out}");
    assert_eq!(parsed["view"]["currentPlayer"], 1);
}
