//! JSON 文字列 API のテスト。wasm 境界の glue はホストターゲットでも
//! 同じ関数が動くため、ここで振る舞いを仕様化する。

use ofc_engine_wasm::{evaluate_board_json, score_matchup_json, standard_ruleset_json};

#[test]
fn 標準ルールセットのjsonが取得できcompile可能な形をしている() {
    let json = standard_ruleset_json();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["variant"], "pineapple");
    assert_eq!(parsed["scoring"]["rowPoint"], 1);
}

#[test]
fn 盤面評価がjson文字列で往復できる() {
    let board = serde_json::json!({
        "top": ["Qs", "Qh", "3d"],
        "middle": ["8c", "8d", "8h", "Kc", "2s"],
        "bottom": ["Ah", "Kh", "Qd", "Jh", "Th"],
    })
    .to_string();

    let result = evaluate_board_json(&board, "[]", &standard_ruleset_json());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();

    assert_eq!(parsed["foul"], false);
    assert_eq!(parsed["top"]["hand"]["category"], "pair");
    assert_eq!(parsed["royaltyTotal"], 11);
    assert_eq!(parsed["fantasylandCards"], 14);
    assert_eq!(parsed["resolved"]["top"][0], "Qs");
}

#[test]
fn 不正な入力はerrorキーを持つjsonで返る() {
    let result = evaluate_board_json("{", "[]", &standard_ruleset_json());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["error"].is_string());

    // 未完成盤面もエラー(panic ではなく error JSON)
    let incomplete = serde_json::json!({ "top": ["Qs"], "middle": [], "bottom": [] }).to_string();
    let result = evaluate_board_json(&incomplete, "[]", &standard_ruleset_json());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert!(parsed["error"].as_str().unwrap().contains("Incomplete"));
}

#[test]
fn 対戦採点がjson文字列でできる() {
    let boards = serde_json::json!([
        {
            "top": ["2c", "4h", "6d"],
            "middle": ["Ks", "Qh", "Th", "6c", "3s"],
            "bottom": ["Ad", "Kd", "Tc", "6s", "3d"],
        },
        {
            "top": ["2s", "3h", "5d"],
            "middle": ["Kc", "Qd", "9h", "5c", "2h"],
            "bottom": ["Ah", "Kh", "9c", "5s", "2d"],
        }
    ])
    .to_string();

    let result = score_matchup_json(&boards, &standard_ruleset_json());
    let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
    assert_eq!(parsed["totals"], serde_json::json!([6, -6]));
}
