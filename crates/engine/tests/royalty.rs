//! ロイヤリティ計算のテスト。
//! 表はデータ(RoyaltyTable)であり、標準プリセットはアメリカン方式(ADR 0003)。

use ofc_engine::Card;
use ofc_engine::hand::{evaluate_five, evaluate_three};
use ofc_engine::royalty::{Row, RoyaltyTable, royalty_points};

fn points5(row: Row, wires: [&str; 5]) -> u32 {
    let cards: Vec<Card> = wires.iter().map(|w| w.parse().unwrap()).collect();
    royalty_points(
        row,
        &evaluate_five(&cards).unwrap(),
        &RoyaltyTable::standard_american(),
    )
}

fn points3(wires: [&str; 3]) -> u32 {
    let cards: Vec<Card> = wires.iter().map(|w| w.parse().unwrap()).collect();
    royalty_points(
        Row::Top,
        &evaluate_three(&cards).unwrap(),
        &RoyaltyTable::standard_american(),
    )
}

// ---- top(ペアは 66 から、トリップスはランク別) ----

#[test]
fn topの66ペアは1点で55以下は0点() {
    assert_eq!(points3(["6s", "6d", "2h"]), 1);
    assert_eq!(points3(["5s", "5d", "2h"]), 0);
    assert_eq!(points3(["2s", "2d", "3h"]), 0);
}

#[test]
fn topのペアはランクに応じて加点される() {
    assert_eq!(points3(["Ts", "Td", "2h"]), 5);
    assert_eq!(points3(["Qs", "Qd", "2h"]), 7);
    assert_eq!(points3(["As", "Ad", "2h"]), 9);
}

#[test]
fn topのトリップスはランクに応じて10点から22点() {
    assert_eq!(points3(["2s", "2d", "2h"]), 10);
    assert_eq!(points3(["Qs", "Qd", "Qh"]), 20);
    assert_eq!(points3(["As", "Ad", "Ah"]), 22);
}

#[test]
fn topのハイカードは0点() {
    assert_eq!(points3(["As", "Kd", "Qh"]), 0);
}

// ---- middle(トリップスから加点) ----

#[test]
fn middleのトリップスは2点() {
    assert_eq!(points5(Row::Middle, ["8c", "8d", "8h", "Kc", "2s"]), 2);
}

#[test]
fn middleのペア以下は0点() {
    assert_eq!(points5(Row::Middle, ["As", "Ad", "9h", "9c", "2s"]), 0); // two pair
    assert_eq!(points5(Row::Middle, ["As", "Ad", "9h", "5c", "2s"]), 0); // pair
}

#[test]
fn middleの上位役の点数() {
    assert_eq!(points5(Row::Middle, ["9s", "8d", "7h", "6c", "5s"]), 4); // straight
    assert_eq!(points5(Row::Middle, ["As", "Ks", "9s", "5s", "2s"]), 8); // flush
    assert_eq!(points5(Row::Middle, ["As", "Ad", "Ah", "9c", "9s"]), 12); // full house
    assert_eq!(points5(Row::Middle, ["As", "Ad", "Ah", "Ac", "9s"]), 20); // quads
    assert_eq!(points5(Row::Middle, ["9s", "8s", "7s", "6s", "5s"]), 30); // straight flush
    assert_eq!(points5(Row::Middle, ["As", "Ks", "Qs", "Js", "Ts"]), 50); // royal
}

// ---- bottom(ストレートから加点。middle の半分相当の表) ----

#[test]
fn bottomはストレートから加点される() {
    assert_eq!(points5(Row::Bottom, ["9s", "8d", "7h", "6c", "5s"]), 2); // straight
    assert_eq!(points5(Row::Bottom, ["As", "Ks", "9s", "5s", "2s"]), 4); // flush
    assert_eq!(points5(Row::Bottom, ["As", "Ad", "Ah", "9c", "9s"]), 6); // full house
    assert_eq!(points5(Row::Bottom, ["As", "Ad", "Ah", "Ac", "9s"]), 10); // quads
    assert_eq!(points5(Row::Bottom, ["9s", "8s", "7s", "6s", "5s"]), 15); // straight flush
    assert_eq!(points5(Row::Bottom, ["As", "Ks", "Qs", "Js", "Ts"]), 25); // royal
}

#[test]
fn bottomのトリップス以下は0点() {
    assert_eq!(points5(Row::Bottom, ["8c", "8d", "8h", "Kc", "2s"]), 0);
}

// ---- 表のカスタマイズ(ローカルルール対応) ----

#[test]
fn ロイヤリティ表は差し替えられる() {
    // ローカルルール例: bottom のトリップスにも 1 点つける
    let mut table = RoyaltyTable::standard_american();
    table.set_row_points(Row::Bottom, ofc_engine::hand::Category::Trips, 1);

    let cards: Vec<Card> = ["8c", "8d", "8h", "Kc", "2s"]
        .iter()
        .map(|w| w.parse().unwrap())
        .collect();
    let hand = evaluate_five(&cards).unwrap();
    assert_eq!(royalty_points(Row::Bottom, &hand, &table), 1);
    // 他の行・役には影響しない
    assert_eq!(royalty_points(Row::Middle, &hand, &table), 2);
}
