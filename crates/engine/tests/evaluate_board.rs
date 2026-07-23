//! 盤面評価(evaluate_board)のテスト。
//! 役 + ロイヤリティ + ファウル + FL 判定 + Joker の盤面全体最適解決を一括で返す
//! wire 境界のエントリポイント(ADR 0003)。

use ofc_engine::evaluate::{EvaluateError, evaluate_board};
use ofc_engine::fantasyland::FantasylandRules;
use ofc_engine::hand::Category;
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::{Board, Card};

fn board(top: &[&str], middle: &[&str], bottom: &[&str]) -> Board {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    Board::new(parse(top), parse(middle), parse(bottom)).unwrap()
}

// ポーカー盤面の役評価を呼ぶだけのヘルパ(コード実行の eval とは無関係)。
fn eval(b: &Board) -> ofc_engine::evaluate::BoardEvaluation {
    evaluate_board(
        b,
        &[],
        &RoyaltyTable::standard_american(),
        &FantasylandRules::standard_progressive(),
    )
    .unwrap()
}

#[test]
fn ジョーカーなしの盤面の役とロイヤリティとflが返る() {
    let result = eval(&board(
        &["Qs", "Qh", "3d"],
        &["8c", "8d", "8h", "Kc", "2s"],
        &["Ah", "Kh", "Qd", "Jh", "Th"],
    ));
    assert!(!result.foul);
    assert_eq!(result.top.hand.category, Category::Pair);
    assert_eq!(result.middle.hand.category, Category::Trips);
    assert_eq!(result.bottom.hand.category, Category::Straight);
    assert_eq!(result.top.royalty, 7);
    assert_eq!(result.middle.royalty, 2);
    assert_eq!(result.bottom.royalty, 2);
    assert_eq!(result.royalty_total, 11);
    assert_eq!(result.fantasyland_cards, Some(14));
}

#[test]
fn ファウル盤面はロイヤリティ合計が0になる() {
    let result = eval(&board(
        &["As", "Ah", "3d"],
        &["Kc", "Qd", "9h", "5c", "2s"],
        &["Ad", "Kh", "Qh", "Jh", "Td"],
    ));
    assert!(result.foul);
    assert_eq!(result.royalty_total, 0);
    assert_eq!(result.fantasyland_cards, None);
}

#[test]
fn ジョーカーはロイヤリティ最大に解決される() {
    // top の Xj は QQ に解決するのが最適(ロイヤリティ 7 点 + FL 突入)
    let result = eval(&board(
        &["Qs", "Xj", "3d"],
        &["8c", "8d", "8h", "Kc", "2s"],
        &["Ah", "Kh", "Qd", "Jh", "Th"],
    ));
    assert!(!result.foul);
    assert_eq!(result.top.hand.category, Category::Pair);
    assert_eq!(result.top.royalty, 7);
    assert_eq!(result.fantasyland_cards, Some(14));
    // 解決後の盤面に Joker は残らない
    assert!(
        result
            .resolved
            .top()
            .iter()
            .all(|c| matches!(c, Card::Standard { .. }))
    );
}

#[test]
fn ジョーカーはファウル回避のため弱く解決されることがある() {
    // top を AA に解決すると middle の 88 ペアより強くなりファウル。
    // ファウル回避を優先し、AA より弱い解決を選ぶ(ADR 0003 の既定セマンティクス)
    let result = eval(&board(
        &["As", "Xj", "3d"],
        &["8c", "8d", "Kc", "Qd", "2s"],
        &["Ah", "Kh", "Qh", "Jh", "Th"],
    ));
    assert!(!result.foul);
    // 88 ペアより弱い範囲での最強(33 か 77 以下のペア等)に収まっている
    assert!(result.top.hand < result.middle.hand);
}

#[test]
fn どう解決してもファウルなら素直にファウルになる() {
    // top は既に AA 確定、middle はハイカードしか作れない
    let result = eval(&board(
        &["As", "Ah", "Xj"],
        &["Kc", "Qd", "9h", "5c", "2s"],
        &["Ad", "Kh", "Qh", "Jh", "Td"],
    ));
    assert!(result.foul);
    assert_eq!(result.royalty_total, 0);
}

#[test]
fn 未完成の盤面はエラーになる() {
    let incomplete = Board::new(vec!["Qs".parse().unwrap()], vec![], vec![]).unwrap();
    let result = evaluate_board(
        &incomplete,
        &[],
        &RoyaltyTable::standard_american(),
        &FantasylandRules::standard_progressive(),
    );
    assert_eq!(result.unwrap_err(), EvaluateError::IncompleteBoard);
}

#[test]
fn 使用済みカードはジョーカーの解決先から除外される() {
    // Q が全て使用済みなら top の Xj は QQ に解決できない
    let used: Vec<Card> = ["Qh", "Qd", "Qc"]
        .iter()
        .map(|w| w.parse().unwrap())
        .collect();
    let result = evaluate_board(
        &board(
            &["Qs", "Xj", "3d"],
            &["8c", "8d", "8h", "Kc", "2s"],
            &["5h", "6s", "7d", "8s", "9d"], // straight(middle の 888 より強い)
        ),
        &used,
        &RoyaltyTable::standard_american(),
        &FantasylandRules::standard_progressive(),
    )
    .unwrap();
    // QQ(7 点)は不可能。それでもファウルしない最善の解決になる
    assert!(!result.foul);
    assert_ne!(result.top.royalty, 7);
}
