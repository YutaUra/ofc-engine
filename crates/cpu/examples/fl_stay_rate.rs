//! FL 継続率の診断: 枚数別に「継続可能な配置が存在する率」と
//! 「完全解が実際に継続を選ぶ率」を測る。デッキはジョーカー 2 枚入り
//! (チャイポ標準)。`cargo run --release -p ofc-engine-cpu --example fl_stay_rate`

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::fantasyland_stay;
use ofc_engine::game::RowKind;
use ofc_engine::{Board, Card, Rank, Suit};
use ofc_engine_cpu::{CpuConfig, choose_fantasyland};

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn random_hand(rng: &mut Rng, n: usize) -> Vec<Card> {
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    let mut deck: Vec<Card> = Rank::ALL_TEST
        .iter()
        .flat_map(|r| suits.map(|s| Card::Standard { rank: *r, suit: s }))
        .collect();
    deck.push(Card::Joker);
    deck.push(Card::Joker);
    let mut hand = Vec::with_capacity(n);
    for _ in 0..n {
        let i = (rng.next() % deck.len() as u64) as usize;
        hand.push(deck.swap_remove(i));
    }
    hand
}

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

fn board_of(placements: &[ofc_engine::game::Placement]) -> Board {
    let mut rows: [Vec<Card>; 3] = [vec![], vec![], vec![]];
    for p in placements {
        let idx = match p.row {
            RowKind::Top => 0,
            RowKind::Middle => 1,
            RowKind::Bottom => 2,
        };
        rows[idx].push(p.card);
    }
    let [t, m, b] = rows;
    Board::new(t, m, b).unwrap()
}

fn main() {
    const SAMPLES: usize = 2000;
    let normal = CpuConfig::standard(); // fl_stay_value = 6.0
    let mut stay_seeker = CpuConfig::standard();
    stay_seeker.fl_stay_value = 100_000.0; // 継続可能なら必ず継続を選ぶ

    println!(
        "{:>4} {:>14} {:>16} {:>14}",
        "枚数", "継続可能率", "完全解の継続率", "継続の機会損失"
    );
    for n in 13..=17 {
        let mut rng = Rng(0xC0FFEE + n as u64);
        let mut achievable = 0u32;
        let mut chosen = 0u32;
        for _ in 0..SAMPLES {
            let dealt = random_hand(&mut rng, n);
            // 盤面にジョーカーが残るため、evaluate_board の解決後盤面で
            // 継続判定する(fantasyland_stay は未解決ジョーカーを評価できない)
            let stays = |board: &Board| -> bool {
                let eval =
                    evaluate_board(board, &[], &normal.royalty, &normal.fantasyland).unwrap();
                fantasyland_stay(&eval.resolved, &normal.fantasyland).unwrap()
            };
            let (p, _) = choose_fantasyland(&dealt, &stay_seeker, 1).unwrap();
            let can_stay = stays(&board_of(&p));
            let (p, _) = choose_fantasyland(&dealt, &normal, 1).unwrap();
            let does_stay = stays(&board_of(&p));
            achievable += u32::from(can_stay);
            chosen += u32::from(does_stay);
        }
        println!(
            "{:>4} {:>13.1}% {:>15.1}% {:>13.1}%",
            n,
            achievable as f64 * 100.0 / SAMPLES as f64,
            chosen as f64 * 100.0 / SAMPLES as f64,
            (achievable - chosen) as f64 * 100.0 / SAMPLES as f64
        );
    }
}
