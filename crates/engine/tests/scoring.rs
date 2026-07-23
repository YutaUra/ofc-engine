//! 対戦採点(1-6 方式)のテスト。
//! 行ごとの勝敗 + scoop ボーナス + ロイヤリティ差分 + ファウルペナルティを仕様化する。

use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_matchup, score_pair};
use ofc_engine::{Board, Card};

fn board(top: [&str; 3], middle: [&str; 5], bottom: [&str; 5]) -> Board {
    let parse =
        |wires: &[&str]| -> Vec<Card> { wires.iter().map(|w| w.parse().unwrap()).collect() };
    Board::new(parse(&top), parse(&middle), parse(&bottom)).unwrap()
}

fn rules() -> (RoyaltyTable, ScoringRules) {
    (RoyaltyTable::standard_american(), ScoringRules::standard())
}

/// ロイヤリティが発生しない弱い盤面(A ハイ系)。
fn weak_board() -> Board {
    board(
        ["2s", "3h", "5d"],
        ["Kc", "Qd", "9h", "5c", "2h"],
        ["Ah", "Kh", "9c", "5s", "2d"],
    )
}

/// weak_board に 3 行すべて勝つがロイヤリティはない盤面。
fn stronger_board() -> Board {
    board(
        ["2c", "4h", "6d"],
        ["Ks", "Qh", "Th", "6c", "3s"],
        ["Ad", "Kd", "Tc", "6s", "3d"],
    )
}

#[test]
fn 全行勝ちはスクープで6点になる() {
    let (royalty, scoring) = rules();
    // 3 行 × 1 点 + scoop 3 点 = 6 点(ゼロサム)
    let (a, b) = score_pair(&stronger_board(), &weak_board(), &royalty, &scoring).unwrap();
    assert_eq!((a, b), (6, -6));
}

#[test]
fn 行の勝ち越し分だけ加点される() {
    let (royalty, scoring) = rules();
    // top と middle は stronger、bottom だけ weak が勝つ盤面
    let two_rows = board(
        ["2c", "4h", "6d"],
        ["Ks", "Qh", "Th", "6c", "3s"], // K-Q-T > weak の K-Q-9
        ["Kd", "Qc", "Jh", "6s", "4d"], // middle 以上だが weak の A ハイには負ける
    );
    let (a, b) = score_pair(&two_rows, &weak_board(), &royalty, &scoring).unwrap();
    assert_eq!((a, b), (1, -1)); // +1 +1 -1
}

#[test]
fn 同格の行は点が動かない() {
    let (royalty, scoring) = rules();
    // 同一構成(スート違い)の盤面同士は全行タイで 0-0
    let mirror = board(
        ["2c", "3d", "5h"],
        ["Kd", "Qc", "9d", "5d", "2h"],
        ["Ac", "Kc", "9s", "5c", "2s"],
    );
    let mirror2 = board(
        ["2d", "3s", "5c"],
        ["Kh", "Qs", "9c", "5s", "2s"],
        ["As", "Ks", "9h", "5d", "2h"],
    );
    let (a, b) = score_pair(&mirror, &mirror2, &royalty, &scoring).unwrap();
    assert_eq!((a, b), (0, 0));
}

#[test]
fn ロイヤリティ差分が加算される() {
    let (royalty, scoring) = rules();
    // trips middle(2 点) + straight bottom(2 点) + QQ top(7 点) = 11 点のロイヤリティ
    let royal_board = board(
        ["Qs", "Qh", "3d"],
        ["8c", "8d", "8h", "Kc", "2s"],
        ["Ah", "Kh", "Qd", "Jh", "Th"],
    );
    let (a, b) = score_pair(&royal_board, &weak_board(), &royalty, &scoring).unwrap();
    // 行 6 点 + ロイヤリティ差 11 点 = 17 点
    assert_eq!((a, b), (17, -17));
}

#[test]
fn ファウルすると全行負け扱いで相手のロイヤリティだけ加算される() {
    let (royalty, scoring) = rules();
    // top AA > middle K ハイでファウル
    let fouled = board(
        ["As", "Ah", "3d"],
        ["Kc", "Qd", "9h", "5c", "2h"],
        ["Ad", "Kh", "Qh", "Jh", "Td"],
    );
    // 相手は QQ top(7 点)のロイヤリティ持ち
    let opponent = board(
        ["Qs", "Qh", "3s"],
        ["8c", "8d", "8h", "Ks", "2s"],
        ["Ac", "Kd", "Qc", "Jc", "Tc"],
    );
    let (a, b) = score_pair(&fouled, &opponent, &royalty, &scoring).unwrap();
    // ファウル側: -(6 + 11)。ファウル側のロイヤリティは 0 になる
    assert_eq!((a, b), (-17, 17));
}

#[test]
fn 両者ファウルなら0点() {
    let (royalty, scoring) = rules();
    let foul_a = board(
        ["As", "Ah", "3d"],
        ["Kc", "Qd", "9h", "5c", "2h"],
        ["Ad", "Kh", "Qh", "Jh", "Td"],
    );
    let foul_b = board(
        ["Ac", "Ad", "3s"],
        ["Ks", "Qc", "9c", "5s", "2s"],
        ["Kd", "Qs", "Jc", "Tc", "9d"],
    );
    let (a, b) = score_pair(&foul_a, &foul_b, &royalty, &scoring).unwrap();
    assert_eq!((a, b), (0, 0));
}

#[test]
fn scoop点はルールで変えられる() {
    let royalty = RoyaltyTable::standard_american();
    // ローカルルール例: scoop なし
    let scoring = ScoringRules {
        row_point: 1,
        scoop_bonus: 0,
    };
    let (a, _) = score_pair(&stronger_board(), &weak_board(), &royalty, &scoring).unwrap();
    assert_eq!(a, 3);
}

#[test]
fn 三人対戦は全ペアの合計になる() {
    let (royalty, scoring) = rules();
    let third = board(
        ["3c", "4d", "7h"],
        ["Js", "Th", "8s", "4c", "3s"],
        ["Jc", "Td", "7c", "4s", "3d"],
    );
    let boards = [stronger_board(), weak_board(), third];
    let totals = score_matchup(&boards, &royalty, &scoring).unwrap();

    // 総当たりの合計は常にゼロサム
    assert_eq!(totals.iter().sum::<i32>(), 0);

    // 各プレイヤーの合計は「全ペア採点の和」に一致する(総当たりの定義)
    let (ab, _) = score_pair(&boards[0], &boards[1], &royalty, &scoring).unwrap();
    let (ac, _) = score_pair(&boards[0], &boards[2], &royalty, &scoring).unwrap();
    let (bc, _) = score_pair(&boards[1], &boards[2], &royalty, &scoring).unwrap();
    assert_eq!(totals, vec![ab + ac, -ab + bc, -ac - bc]);
}
