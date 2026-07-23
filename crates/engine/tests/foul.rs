//! ファウル判定のテスト。
//! OFC の配置制約: bottom ≥ middle ≥ top(強さ順)。破ると foul。

use ofc_engine::foul::{FoulCheckError, check_foul};
use ofc_engine::{Board, Card};

fn board(top: [&str; 3], middle: [&str; 5], bottom: [&str; 5]) -> Board {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    Board::new(parse(&top), parse(&middle), parse(&bottom)).unwrap()
}

#[test]
fn 正しい強さ順の盤面はファウルではない() {
    let board = board(
        ["Qs", "Qh", "3d"],             // pair QQ
        ["8c", "8d", "8h", "Kc", "2s"], // trips
        ["Ah", "Kh", "Qd", "Jh", "Th"], // straight
    );
    assert_eq!(check_foul(&board), Ok(false));
}

#[test]
fn topがmiddleより強いとファウル() {
    let board = board(
        ["As", "Ah", "3d"],             // pair AA
        ["Kc", "Qd", "9h", "5c", "2s"], // K ハイ
        ["Ad", "Kh", "Qh", "Jh", "Td"], // straight
    );
    assert_eq!(check_foul(&board), Ok(true));
}

#[test]
fn middleがbottomより強いとファウル() {
    let board = board(
        ["3s", "4h", "5d"],             // ハイカード
        ["Ac", "Ad", "Ah", "Kc", "Ks"], // フルハウス
        ["Qd", "Qh", "9h", "5h", "2d"], // pair QQ
    );
    assert_eq!(check_foul(&board), Ok(true));
}

#[test]
fn 隣接する列が同じ強さならファウルではない() {
    // middle と bottom が完全に同格(同ランク構成のフラッシュなし手)
    let board = board(
        ["3s", "4h", "5d"],
        ["Ac", "Kd", "9h", "5c", "2s"], // A-K-9-5-2 ハイカード
        ["Ad", "Kh", "9c", "5s", "2d"], // 同構成のハイカード
    );
    assert_eq!(check_foul(&board), Ok(false));
}

#[test]
fn 同カテゴリはキッカーまで比較してファウル判定される() {
    // middle のペアが top のペアと同ランクでも、キッカーが弱ければファウル
    let board = board(
        ["Qs", "Qh", "9d"],             // QQ + kicker 9
        ["Qc", "Qd", "8h", "5c", "2s"], // QQ + kicker 8 → top より弱い
        ["Ad", "Ah", "Kc", "Ks", "9h"], // two pair AAKK
    );
    assert_eq!(check_foul(&board), Ok(true));
}

#[test]
fn 未完成の盤面はファウル判定できない() {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    let incomplete = Board::new(parse(&["Qs"]), vec![], vec![]).unwrap();
    assert_eq!(
        check_foul(&incomplete),
        Err(FoulCheckError::IncompleteBoard)
    );
}
