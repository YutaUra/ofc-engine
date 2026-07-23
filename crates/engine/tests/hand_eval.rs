//! 役評価(5 枚・3 枚)のテスト。
//! カテゴリの安定キー(ADR 0003)と、同カテゴリ内のキッカー比較を仕様化する。
//! Joker 入りの評価は Joker 解決(別サイクル)の後段で扱う。

use ofc_engine::Card;
use ofc_engine::hand::{Category, HandRank, evaluate_five, evaluate_three};

fn hand5(wires: [&str; 5]) -> HandRank {
    let cards: Vec<Card> = wires.iter().map(|w| w.parse().unwrap()).collect();
    evaluate_five(&cards).unwrap()
}

fn hand3(wires: [&str; 3]) -> HandRank {
    let cards: Vec<Card> = wires.iter().map(|w| w.parse().unwrap()).collect();
    evaluate_three(&cards).unwrap()
}

// ---- 5 枚役のカテゴリ判定 ----

#[test]
fn ハイカードを判定できる() {
    assert_eq!(
        hand5(["As", "Kd", "9h", "5c", "2s"]).category,
        Category::HighCard
    );
}

#[test]
fn ワンペアを判定できる() {
    assert_eq!(
        hand5(["As", "Ad", "9h", "5c", "2s"]).category,
        Category::Pair
    );
}

#[test]
fn ツーペアを判定できる() {
    assert_eq!(
        hand5(["As", "Ad", "9h", "9c", "2s"]).category,
        Category::TwoPair
    );
}

#[test]
fn スリーカードを判定できる() {
    assert_eq!(
        hand5(["As", "Ad", "Ah", "9c", "2s"]).category,
        Category::Trips
    );
}

#[test]
fn ストレートを判定できる() {
    assert_eq!(
        hand5(["9s", "8d", "7h", "6c", "5s"]).category,
        Category::Straight
    );
}

#[test]
fn エースハイストレートを判定できる() {
    assert_eq!(
        hand5(["As", "Kd", "Qh", "Jc", "Ts"]).category,
        Category::Straight
    );
}

#[test]
fn ホイールはストレートとして判定される() {
    // A-2-3-4-5(ホイール)。A をローとして扱うエッジケース
    assert_eq!(
        hand5(["As", "2d", "3h", "4c", "5s"]).category,
        Category::Straight
    );
}

#[test]
fn フラッシュを判定できる() {
    assert_eq!(
        hand5(["As", "Ks", "9s", "5s", "2s"]).category,
        Category::Flush
    );
}

#[test]
fn フルハウスを判定できる() {
    assert_eq!(
        hand5(["As", "Ad", "Ah", "9c", "9s"]).category,
        Category::FullHouse
    );
}

#[test]
fn フォーカードを判定できる() {
    assert_eq!(
        hand5(["As", "Ad", "Ah", "Ac", "9s"]).category,
        Category::Quads
    );
}

#[test]
fn ストレートフラッシュを判定できる() {
    assert_eq!(
        hand5(["9s", "8s", "7s", "6s", "5s"]).category,
        Category::StraightFlush
    );
}

#[test]
fn ロイヤルフラッシュは独立カテゴリとして判定される() {
    // ロイヤリティ表でストレートフラッシュと点が異なるため別カテゴリにする
    assert_eq!(
        hand5(["As", "Ks", "Qs", "Js", "Ts"]).category,
        Category::RoyalFlush
    );
}

#[test]
fn スチールホイールはロイヤルではなくストレートフラッシュ() {
    assert_eq!(
        hand5(["As", "2s", "3s", "4s", "5s"]).category,
        Category::StraightFlush
    );
}

// ---- 同カテゴリ内の強さ比較(キッカー) ----

#[test]
fn 同カテゴリはキッカーで比較される() {
    // ペア同士: ペアランク優先
    assert!(hand5(["Ks", "Kd", "9h", "5c", "2s"]) > hand5(["Qs", "Qd", "Ah", "5c", "2s"]));
    // 同ペアならキッカー
    assert!(hand5(["Ks", "Kd", "Ah", "5c", "2s"]) > hand5(["Kh", "Kc", "Qh", "5d", "2d"]));
}

#[test]
fn ツーペアは上のペアから比較される() {
    assert!(hand5(["As", "Ad", "2h", "2c", "3s"]) > hand5(["Ks", "Kd", "Qh", "Qc", "As"]));
}

#[test]
fn ホイールは最弱のストレート() {
    // ホイールの強さは 5 ハイ。6 ハイストレートより弱い
    assert!(hand5(["6s", "5d", "4h", "3c", "2s"]) > hand5(["As", "2d", "3h", "4c", "5s"]));
}

#[test]
fn カテゴリが違えばカテゴリで決まる() {
    assert!(hand5(["2s", "2d", "3h", "3c", "4s"]) > hand5(["As", "Kd", "Qh", "Jc", "9s"]));
    assert!(hand5(["2s", "3s", "7s", "8s", "Ts"]) > hand5(["9s", "8d", "7h", "6c", "5c"]));
}

#[test]
fn フルハウスはトリップス部分から比較される() {
    assert!(hand5(["3s", "3d", "3h", "2c", "2s"]) > hand5(["2h", "2d", "2c", "As", "Ad"]));
}

// ---- 3 枚役(top) ----

#[test]
fn 三枚のハイカードとペアとトリップスを判定できる() {
    assert_eq!(hand3(["As", "Kd", "9h"]).category, Category::HighCard);
    assert_eq!(hand3(["Qs", "Qd", "9h"]).category, Category::Pair);
    assert_eq!(hand3(["Qs", "Qd", "Qh"]).category, Category::Trips);
}

#[test]
fn 三枚役にストレートとフラッシュは存在しない() {
    // OFC の top はストレート/フラッシュを役として扱わない
    assert_eq!(hand3(["9s", "8s", "7s"]).category, Category::HighCard);
    assert_eq!(hand3(["As", "Ks", "Qs"]).category, Category::HighCard);
}

#[test]
fn 三枚役同士もキッカーで比較される() {
    assert!(hand3(["Qs", "Qd", "9h"]) > hand3(["Qh", "Qc", "8h"]));
    assert!(hand3(["2s", "2d", "3h"]) > hand3(["As", "Kd", "Qh"]));
}

// ---- 入力バリデーション ----

#[test]
fn 枚数違いはエラーになる() {
    let four: Vec<Card> = ["As", "Kd", "Qh", "Jc"]
        .iter()
        .map(|w| w.parse().unwrap())
        .collect();
    assert!(evaluate_five(&four).is_err());
    assert!(evaluate_three(&four).is_err());
}

#[test]
fn 未解決のジョーカーが混ざっているとエラーになる() {
    // 役評価は Joker 解決済みの手札を前提とする(ADR 0003 の責務分離)
    let with_joker: Vec<Card> = ["As", "Kd", "Qh", "Jc", "Xj"]
        .iter()
        .map(|w| w.parse().unwrap())
        .collect();
    assert!(evaluate_five(&with_joker).is_err());
}
