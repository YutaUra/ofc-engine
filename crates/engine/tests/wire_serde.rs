//! JSON wire 表現(ADR 0003)のテスト。
//! Card は文字列、Board は {top, middle, bottom}、役カテゴリは安定キー。

use ofc_engine::hand::Category;
use ofc_engine::{Board, Card};

#[test]
fn cardは文字列としてシリアライズされる() {
    let card: Card = "As".parse().unwrap();
    assert_eq!(serde_json::to_string(&card).unwrap(), r#""As""#);
    let joker: Card = "Xj".parse().unwrap();
    assert_eq!(serde_json::to_string(&joker).unwrap(), r#""Xj""#);
}

#[test]
fn cardは文字列からデシリアライズされる() {
    let card: Card = serde_json::from_str(r#""Td""#).unwrap();
    assert_eq!(card, "Td".parse().unwrap());
}

#[test]
fn 不正なcard文字列はデシリアライズエラーになる() {
    assert!(serde_json::from_str::<Card>(r#""ZZ""#).is_err());
    assert!(serde_json::from_str::<Card>(r#""as""#).is_err());
}

#[test]
fn boardはrows形式でシリアライズされる() {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    let board = Board::new(
        parse(&["Qs", "Qh", "3d"]),
        parse(&["8c", "8d", "8h", "Kc", "2s"]),
        parse(&["Ah", "Kh", "Qd", "Jh", "Th"]),
    )
    .unwrap();

    let json = serde_json::to_value(&board).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "top": ["Qs", "Qh", "3d"],
            "middle": ["8c", "8d", "8h", "Kc", "2s"],
            "bottom": ["Ah", "Kh", "Qd", "Jh", "Th"],
        })
    );
}

#[test]
fn boardのデシリアライズは不変条件を検証する() {
    // 正常
    let board: Board = serde_json::from_value(serde_json::json!({
        "top": ["Qs"],
        "middle": [],
        "bottom": ["Ah"],
    }))
    .unwrap();
    assert_eq!(board.top().len(), 1);

    // 段サイズ超過は拒否(Board::new と同じ検証を通る)
    assert!(
        serde_json::from_value::<Board>(serde_json::json!({
            "top": ["Qs", "Qh", "3d", "4d"],
            "middle": [],
            "bottom": [],
        }))
        .is_err()
    );

    // 重複カードも拒否
    assert!(
        serde_json::from_value::<Board>(serde_json::json!({
            "top": ["Qs", "Qs", "3d"],
            "middle": [],
            "bottom": [],
        }))
        .is_err()
    );
}

#[test]
fn 役カテゴリは安定キーでシリアライズされる() {
    assert_eq!(
        serde_json::to_string(&Category::StraightFlush).unwrap(),
        r#""straight_flush""#
    );
    assert_eq!(
        serde_json::to_string(&Category::HighCard).unwrap(),
        r#""high_card""#
    );
    let category: Category = serde_json::from_str(r#""full_house""#).unwrap();
    assert_eq!(category, Category::FullHouse);
}
