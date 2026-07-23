//! 役評価の厳密性オラクル。
//! 最適化前の素朴な実装を参照実装としてここに固定し、本体の実装
//! (今後高速化される)と結果が完全一致することをランダム手で検証する。
//! 参照実装は「読んで正しいと確信できること」を優先し、一切最適化しない。

use ofc_engine::hand::{Category, HandRank, evaluate_five, evaluate_three};
use ofc_engine::{Card, Rank, Suit};

// ---- 参照実装(最適化禁止。本体の旧実装のコピー) ----

fn ref_five(cards: &[Card]) -> HandRank {
    let (ranks, suits) = split(cards);
    let groups = rank_groups(&ranks);
    let is_flush = suits.windows(2).all(|w| w[0] == w[1]);
    let straight_high = straight_high(&ranks);

    match (straight_high, is_flush, groups.as_slice()) {
        (Some(Rank::Ace), true, _) => hand(Category::RoyalFlush, vec![]),
        (Some(high), true, _) => hand(Category::StraightFlush, vec![high]),
        (_, _, [(4, quad), (1, kicker)]) => hand(Category::Quads, vec![*quad, *kicker]),
        (_, _, [(3, trips), (2, pair)]) => hand(Category::FullHouse, vec![*trips, *pair]),
        (_, true, _) => hand(Category::Flush, ranks_desc(&ranks)),
        (Some(high), false, _) => hand(Category::Straight, vec![high]),
        (_, _, [(3, trips), (1, k1), (1, k2)]) => hand(Category::Trips, vec![*trips, *k1, *k2]),
        (_, _, [(2, hp), (2, lp), (1, k)]) => hand(Category::TwoPair, vec![*hp, *lp, *k]),
        (_, _, [(2, pair), rest @ ..]) => hand(
            Category::Pair,
            std::iter::once(*pair)
                .chain(rest.iter().map(|(_, r)| *r))
                .collect(),
        ),
        _ => hand(Category::HighCard, ranks_desc(&ranks)),
    }
}

fn ref_three(cards: &[Card]) -> HandRank {
    let (ranks, _) = split(cards);
    let groups = rank_groups(&ranks);
    match groups.as_slice() {
        [(3, trips)] => hand(Category::Trips, vec![*trips]),
        [(2, pair), (1, kicker)] => hand(Category::Pair, vec![*pair, *kicker]),
        _ => hand(Category::HighCard, ranks_desc(&ranks)),
    }
}

fn hand(category: Category, tiebreak: Vec<Rank>) -> HandRank {
    HandRank { category, tiebreak }
}

fn split(cards: &[Card]) -> (Vec<Rank>, Vec<Suit>) {
    cards
        .iter()
        .map(|c| match c {
            Card::Standard { rank, suit } => (*rank, *suit),
            Card::Joker => unreachable!("オラクルは標準カードのみ扱う"),
        })
        .unzip()
}

fn rank_groups(ranks: &[Rank]) -> Vec<(usize, Rank)> {
    let mut counts = std::collections::BTreeMap::new();
    for rank in ranks {
        *counts.entry(*rank).or_insert(0usize) += 1;
    }
    let mut groups: Vec<(usize, Rank)> = counts.into_iter().map(|(r, c)| (c, r)).collect();
    groups.sort_unstable_by(|a, b| b.cmp(a));
    groups
}

fn ranks_desc(ranks: &[Rank]) -> Vec<Rank> {
    let mut sorted = ranks.to_vec();
    sorted.sort_unstable_by(|a, b| b.cmp(a));
    sorted
}

fn straight_high(ranks: &[Rank]) -> Option<Rank> {
    let mut sorted = ranks.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    if sorted.len() != 5 {
        return None;
    }
    if sorted.windows(2).all(|w| w[1] as u8 - w[0] as u8 == 1) {
        return Some(sorted[4]);
    }
    if sorted == [Rank::Two, Rank::Three, Rank::Four, Rank::Five, Rank::Ace] {
        return Some(Rank::Five);
    }
    None
}

// ---- 検証 ----

fn full_deck() -> Vec<Card> {
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    ranks
        .into_iter()
        .flat_map(|rank| suits.map(|suit| Card::Standard { rank, suit }))
        .collect()
}

/// 決定的な xorshift64。テストの再現性のため OS 乱数は使わない。
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn random_hand(rng: &mut Rng, deck: &[Card], n: usize) -> Vec<Card> {
    let mut deck = deck.to_vec();
    let mut hand = Vec::with_capacity(n);
    for _ in 0..n {
        let i = (rng.next() % deck.len() as u64) as usize;
        hand.push(deck.swap_remove(i));
    }
    hand
}

#[test]
fn ランダム10万手で5枚評価が参照実装と完全一致する() {
    let deck = full_deck();
    let mut rng = Rng(0x0FC0_FFEE);
    for i in 0..100_000 {
        let hand = random_hand(&mut rng, &deck, 5);
        let actual = evaluate_five(&hand).unwrap();
        let expected = ref_five(&hand);
        assert_eq!(actual, expected, "{i} 手目で不一致: {hand:?}");
    }
}

#[test]
fn ランダム10万手で3枚評価が参照実装と完全一致する() {
    let deck = full_deck();
    let mut rng = Rng(0xB0A4_D001);
    for i in 0..100_000 {
        let hand = random_hand(&mut rng, &deck, 3);
        let actual = evaluate_three(&hand).unwrap();
        let expected = ref_three(&hand);
        assert_eq!(actual, expected, "{i} 手目で不一致: {hand:?}");
    }
}

/// 全 2,598,960 通りの網羅検証。重いので通常は ignore。
/// 高速化の節目に `cargo test --release -- --ignored` で実行する。
#[test]
#[ignore = "網羅版は release でのみ実行する"]
fn 全ての5枚組み合わせで参照実装と完全一致する() {
    let deck = full_deck();
    let mut count = 0u64;
    for a in 0..48 {
        for b in (a + 1)..49 {
            for c in (b + 1)..50 {
                for d in (c + 1)..51 {
                    for e in (d + 1)..52 {
                        let hand = [deck[a], deck[b], deck[c], deck[d], deck[e]];
                        let actual = evaluate_five(&hand).unwrap();
                        let expected = ref_five(&hand);
                        assert_eq!(actual, expected, "不一致: {hand:?}");
                        count += 1;
                    }
                }
            }
        }
    }
    assert_eq!(count, 2_598_960);
}
