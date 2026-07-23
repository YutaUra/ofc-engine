//! Fantasyland 突入・継続判定のテスト。
//! progressive FL(QQ=14 / KK=15 / AA=16 / トリップス=17 枚)を
//! データ(FantasylandRules)で表現する(ADR 0003)。

use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::foul::FoulCheckError;
use ofc_engine::{Board, Card};

fn board(top: [&str; 3], middle: [&str; 5], bottom: [&str; 5]) -> Board {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    Board::new(parse(&top), parse(&middle), parse(&bottom)).unwrap()
}

fn rules() -> FantasylandRules {
    FantasylandRules::standard_progressive()
}

// ---- 突入判定 ----

#[test]
fn top_qqでファウルなしなら14枚のflに突入する() {
    let board = board(
        ["Qs", "Qh", "3d"],
        ["8c", "8d", "8h", "Kc", "2s"],
        ["Ah", "Kh", "Qd", "Jh", "Th"],
    );
    assert_eq!(fantasyland_entry(&board, &rules()), Ok(Some(14)));
}

#[test]
fn progressive_flはペアのランクで配布枚数が変わる() {
    let kk = board(
        ["Ks", "Kh", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["Ah", "Kd", "Qd", "Jh", "Th"],
    );
    assert_eq!(fantasyland_entry(&kk, &rules()), Ok(Some(15)));

    let aa = board(
        ["As", "Ah", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["Ad", "Kd", "Qd", "Jh", "Td"],
    );
    assert_eq!(fantasyland_entry(&aa, &rules()), Ok(Some(16)));
}

#[test]
fn topトリップスは17枚のflに突入する() {
    let board = board(
        ["2s", "2h", "2d"],
        ["8c", "8d", "8h", "Qc", "3s"],
        ["Ah", "Kh", "Qd", "Jh", "Th"],
    );
    assert_eq!(fantasyland_entry(&board, &rules()), Ok(Some(17)));
}

#[test]
fn top_jj以下ではflに突入しない() {
    let board = board(
        ["Js", "Jh", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["Ah", "Kh", "Qd", "Th", "9h"],
    );
    assert_eq!(fantasyland_entry(&board, &rules()), Ok(None));
}

#[test]
fn ファウルした盤面はqq以上でも突入しない() {
    // top AA が middle のハイカードより強い → ファウル
    let board = board(
        ["As", "Ah", "3d"],
        ["Kc", "Qd", "9h", "5c", "2s"],
        ["Ad", "Kh", "Qh", "Jh", "Td"],
    );
    assert_eq!(fantasyland_entry(&board, &rules()), Ok(None));
}

#[test]
fn 未完成の盤面は突入判定できない() {
    let incomplete = Board::new(vec!["Qs".parse().unwrap()], vec![], vec![]).unwrap();
    assert_eq!(
        fantasyland_entry(&incomplete, &rules()),
        Err(FoulCheckError::IncompleteBoard)
    );
}

// ---- 継続(stay)判定 ----

#[test]
fn topトリップスで継続できる() {
    let board = board(
        ["5s", "5h", "5d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["Ah", "Kh", "Qd", "Jh", "Th"],
    );
    assert_eq!(fantasyland_stay(&board, &rules()), Ok(true));
}

#[test]
fn bottomクワッズ以上で継続できる() {
    let quads = board(
        ["As", "Kh", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["9s", "9d", "9h", "9c", "Th"],
    );
    assert_eq!(fantasyland_stay(&quads, &rules()), Ok(true));

    let straight_flush = board(
        ["As", "Kh", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["9s", "Ts", "Js", "Qs", "Ks"],
    );
    assert_eq!(fantasyland_stay(&straight_flush, &rules()), Ok(true));
}

#[test]
fn 条件を満たさなければ継続できない() {
    let board = board(
        ["Qs", "Qh", "3d"], // QQ は突入条件であって継続条件ではない
        ["8c", "8d", "8h", "Kc", "2s"],
        ["Ah", "Kh", "Qd", "Jh", "Th"],
    );
    assert_eq!(fantasyland_stay(&board, &rules()), Ok(false));
}

#[test]
fn ファウルした盤面は継続できない() {
    let board = board(
        ["5s", "5h", "5d"],             // top トリップスだが…
        ["Kc", "Qd", "9h", "4c", "2s"], // middle がハイカードでファウル
        ["Ah", "Kh", "Qh", "Jh", "Th"],
    );
    assert_eq!(fantasyland_stay(&board, &rules()), Ok(false));
}

// ---- ローカルルール対応 ----

#[test]
fn 非progressiveルールでは配布枚数を揃えられる() {
    // ローカルルール例: QQ でも AA でも一律 14 枚
    let mut rules = FantasylandRules::standard_progressive();
    rules.set_uniform_cards(14);

    let aa = board(
        ["As", "Ah", "3d"],
        ["8c", "8d", "8h", "Qc", "2s"],
        ["Ad", "Kd", "Qd", "Jh", "Td"],
    );
    assert_eq!(fantasyland_entry(&aa, &rules), Ok(Some(14)));
}
