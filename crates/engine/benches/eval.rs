//! 役評価・Joker 解決・採点のベンチマーク。
//! 高速化の各ステップでここの数値を比較する(結果の厳密性はテストが担保)。

use criterion::{Criterion, black_box, criterion_group, criterion_main};
use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::FantasylandRules;
use ofc_engine::hand::{evaluate_five, evaluate_three};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine::{Board, Card};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

/// カテゴリが偏らないよう代表的な 8 種の 5 枚手。
fn five_card_hands() -> Vec<Vec<Card>> {
    [
        ["As", "Kd", "9h", "5c", "2s"], // high card
        ["As", "Ad", "9h", "5c", "2s"], // pair
        ["As", "Ad", "9h", "9c", "2s"], // two pair
        ["As", "Ad", "Ah", "9c", "2s"], // trips
        ["9s", "8d", "7h", "6c", "5s"], // straight
        ["As", "Ks", "9s", "5s", "2s"], // flush
        ["As", "Ad", "Ah", "9c", "9s"], // full house
        ["9s", "8s", "7s", "6s", "5s"], // straight flush
    ]
    .iter()
    .map(|h| cards(h))
    .collect()
}

fn bench_evaluate_five(c: &mut Criterion) {
    let hands = five_card_hands();
    c.bench_function("evaluate_five(8手)", |b| {
        b.iter(|| {
            for hand in &hands {
                black_box(evaluate_five(black_box(hand)).unwrap());
            }
        })
    });
}

fn bench_evaluate_three(c: &mut Criterion) {
    let hands: Vec<Vec<Card>> = [["As", "Kd", "9h"], ["Qs", "Qd", "9h"], ["Qs", "Qd", "Qh"]]
        .iter()
        .map(|h| cards(h))
        .collect();
    c.bench_function("evaluate_three(3手)", |b| {
        b.iter(|| {
            for hand in &hands {
                black_box(evaluate_three(black_box(hand)).unwrap());
            }
        })
    });
}

fn bench_score_pair(c: &mut Criterion) {
    let a = Board::new(
        cards(&["Qs", "Qh", "3d"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh", "Th"]),
    )
    .unwrap();
    let b_board = Board::new(
        cards(&["2c", "4h", "6d"]),
        cards(&["Ks", "Qc", "Tc", "6c", "3s"]),
        cards(&["Ad", "Kd", "Td", "6s", "3d"]),
    )
    .unwrap();
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    c.bench_function("score_pair", |b| {
        b.iter(|| black_box(score_pair(&a, &b_board, &royalty, &scoring).unwrap()))
    });
}

fn bench_joker_board(c: &mut Criterion) {
    let royalty = RoyaltyTable::standard_american();
    let fl = FantasylandRules::standard_progressive();

    let no_joker = Board::new(
        cards(&["Qs", "Qh", "3d"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh", "Th"]),
    )
    .unwrap();
    c.bench_function("evaluate_board(jokerなし)", |b| {
        b.iter(|| black_box(evaluate_board(&no_joker, &[], &royalty, &fl).unwrap()))
    });

    let one_joker = Board::new(
        cards(&["Qs", "Xj", "3d"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh", "Th"]),
    )
    .unwrap();
    c.bench_function("evaluate_board(joker1枚)", |b| {
        b.iter(|| black_box(evaluate_board(&one_joker, &[], &royalty, &fl).unwrap()))
    });

    let two_jokers = Board::new(
        cards(&["Qs", "Xj", "3d"]),
        cards(&["8c", "8d", "Xj", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh", "Th"]),
    )
    .unwrap();
    c.bench_function("evaluate_board(joker2枚)", |b| {
        b.iter(|| black_box(evaluate_board(&two_jokers, &[], &royalty, &fl).unwrap()))
    });
}

criterion_group!(
    benches,
    bench_evaluate_five,
    bench_evaluate_three,
    bench_score_pair,
    bench_joker_board
);
criterion_main!(benches);
