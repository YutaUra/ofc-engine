//! Joker 盤面解決の厳密性オラクル。
//! evaluate_board の解決結果(ファウル回避 > ロイヤリティ > 行の強さ)が、
//! 素朴な全候補総当たりの参照実装と完全一致することをランダム盤面で検証する。

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::FantasylandRules;
use ofc_engine::hand::{HandRank, evaluate_five, evaluate_three};
use ofc_engine::royalty::{Row, RoyaltyTable, royalty_points};
use ofc_engine::{Board, Card, Rank, Suit};

fn full_deck() -> Vec<Card> {
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    Rank::ALL_TEST
        .iter()
        .flat_map(|rank| suits.map(|suit| Card::Standard { rank: *rank, suit }))
        .collect()
}

// Rank::ALL は crate 内部のため、テスト側で複製する
trait RankAll {
    const ALL_TEST: [Rank; 13];
}
impl RankAll for Rank {
    const ALL_TEST: [Rank; 13] = [
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
}

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

/// ランダムな完成盤面(joker_count 枚の Joker 入り)を作る。
fn random_board(rng: &mut Rng, joker_count: usize) -> Board {
    let mut deck = full_deck();
    let mut cards = Vec::with_capacity(13);
    for _ in 0..13 {
        let i = (rng.next() % deck.len() as u64) as usize;
        cards.push(deck.swap_remove(i));
    }
    for slot in cards.iter_mut().take(joker_count) {
        *slot = Card::Joker;
    }
    // 位置もランダム化(Joker が特定の行に偏らないように)
    for i in (1..cards.len()).rev() {
        let j = (rng.next() % (i as u64 + 1)) as usize;
        cards.swap(i, j);
    }
    Board::new(
        cards[..3].to_vec(),
        cards[3..8].to_vec(),
        cards[8..].to_vec(),
    )
    .unwrap()
}

/// 参照実装: 全割り当てを素朴に総当たりし、同じ優先度キーで最良を選ぶ。
fn ref_best_key(board: &Board, royalty: &RoyaltyTable) -> (bool, u32, [HandRank; 3]) {
    let board_cards: Vec<Card> = board
        .top()
        .iter()
        .chain(board.middle())
        .chain(board.bottom())
        .copied()
        .collect();
    let jokers: Vec<usize> = board_cards
        .iter()
        .enumerate()
        .filter(|(_, c)| matches!(c, Card::Joker))
        .map(|(i, _)| i)
        .collect();
    let candidates: Vec<Card> = full_deck()
        .into_iter()
        .filter(|c| !board_cards.contains(c))
        .collect();

    let assignments: Vec<Vec<Card>> = match jokers.len() {
        0 => vec![vec![]],
        1 => candidates.iter().map(|c| vec![*c]).collect(),
        2 => {
            let mut all = Vec::new();
            for (i, &c0) in candidates.iter().enumerate() {
                for (j, &c1) in candidates.iter().enumerate() {
                    if i != j {
                        all.push(vec![c0, c1]);
                    }
                }
            }
            all
        }
        _ => unreachable!(),
    };

    let mut best: Option<(bool, u32, [HandRank; 3])> = None;
    for assignment in assignments {
        let mut cards = board_cards.clone();
        for (pos, card) in jokers.iter().zip(&assignment) {
            cards[*pos] = *card;
        }
        let top = evaluate_three(&cards[..3]).unwrap();
        let middle = evaluate_five(&cards[3..8]).unwrap();
        let bottom = evaluate_five(&cards[8..]).unwrap();
        let foul = middle < top || bottom < middle;
        let total = if foul {
            0
        } else {
            royalty_points(Row::Top, &top, royalty)
                + royalty_points(Row::Middle, &middle, royalty)
                + royalty_points(Row::Bottom, &bottom, royalty)
        };
        let key = (!foul, total, [bottom, middle, top]);
        if best.as_ref().is_none_or(|b| key > *b) {
            best = Some(key);
        }
    }
    best.unwrap()
}

fn assert_matches_reference(board: &Board) {
    let royalty = RoyaltyTable::standard_american();
    let fl = FantasylandRules::standard_progressive();
    let result = evaluate_board(board, &[], &royalty, &fl).unwrap();

    let (ref_not_foul, ref_royalty, ref_hands) = ref_best_key(board, &royalty);
    assert_eq!(result.foul, !ref_not_foul, "foul 不一致: {board:?}");
    if !result.foul {
        assert_eq!(
            result.royalty_total, ref_royalty,
            "royalty 不一致: {board:?}"
        );
    }
    // 解決後の行の強さもキーと一致すること(bottom, middle, top の順)
    assert_eq!(
        [
            result.bottom.hand.clone(),
            result.middle.hand.clone(),
            result.top.hand.clone()
        ],
        ref_hands,
        "行の強さ不一致: {board:?}"
    );
}

#[test]
fn ジョーカー1枚のランダム盤面100個で参照実装と一致する() {
    let mut rng = Rng(0x01AB_0701);
    for _ in 0..100 {
        assert_matches_reference(&random_board(&mut rng, 1));
    }
}

#[test]
fn ジョーカー2枚のランダム盤面30個で参照実装と一致する() {
    let mut rng = Rng(0x02AB_0702);
    for _ in 0..30 {
        assert_matches_reference(&random_board(&mut rng, 2));
    }
}

#[test]
fn ジョーカーなしのランダム盤面100個でも一致する() {
    let mut rng = Rng(0x03AB_0703);
    for _ in 0..100 {
        assert_matches_reference(&random_board(&mut rng, 0));
    }
}
