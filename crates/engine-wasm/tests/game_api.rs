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

#[test]
fn flゲームは一括配置で完了しviewにflフェーズが出る() {
    use ofc_engine_wasm::game_new_fl_json;
    let out = game_new_fl_json(2, 0, "7", "[14, 0]");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert_eq!(
        parsed["view"]["street"],
        serde_json::json!({ "phase": "fantasyland" })
    );
    let dealt: Vec<String> = parsed["view"]["dealtCards"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| c.as_str().unwrap().to_string())
        .collect();
    assert_eq!(dealt.len(), 14);

    // 先頭 13 枚を bottom5/middle5/top3 に配置し、残り 1 枚を配列で捨てる
    let mut placements = Vec::new();
    for (i, card) in dealt.iter().take(13).enumerate() {
        let row = if i < 5 {
            "bottom"
        } else if i < 10 {
            "middle"
        } else {
            "top"
        };
        placements.push(serde_json::json!({ "card": card, "row": row }));
    }
    let state = parsed["state"].as_str().unwrap();
    let out = ofc_engine_wasm::game_apply_json(
        state,
        &serde_json::Value::Array(placements).to_string(),
        &serde_json::json!([dealt[13]]).to_string(),
    );
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_null(), "{out}");
    // 次は通常プレイヤーの initial
    assert_eq!(parsed["view"]["currentPlayer"], 1);
    assert_eq!(
        parsed["view"]["street"],
        serde_json::json!({ "phase": "initial" })
    );
}

#[test]
fn cpuが通常手番とfl手番を打ってゲームを完走できる() {
    use ofc_engine_wasm::game_cpu_apply_json;
    // FL 込みのゲームを CPU 同士で最後まで
    let out = ofc_engine_wasm::game_new_fl_json(2, 2, "11", "[14, 0]");
    let mut parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let mut state = parsed["state"].as_str().unwrap().to_string();
    let mut seed = 1000u64;
    while parsed["view"]["street"]["phase"] != "finished" {
        let options = serde_json::json!({ "strength": 0.7, "seed": seed }).to_string();
        let out = game_cpu_apply_json(&state, &options);
        parsed = serde_json::from_str(&out).unwrap();
        assert!(parsed["error"].is_null(), "{out}");
        state = parsed["state"].as_str().unwrap().to_string();
        seed += 1;
    }
    for board in parsed["view"]["boards"].as_array().unwrap() {
        assert_eq!(board["bottom"].as_array().unwrap().len(), 5);
    }
}

#[test]
fn cpuの強さは範囲外でも丸められエラーにならない() {
    use ofc_engine_wasm::game_cpu_apply_json;
    let out = ofc_engine_wasm::game_new_json(2, 0, "5");
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    let state = parsed["state"].as_str().unwrap();
    let out = game_cpu_apply_json(state, r#"{"strength": 1.5, "seed": 1}"#);
    let parsed: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(parsed["error"].is_null(), "{out}");
}
