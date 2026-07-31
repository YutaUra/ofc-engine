//! 実ブックの検証: ロード・カバレッジ・対戦効果(book+rollout vs 貪欲)。
//! 引数: ブックのパス(非圧縮テキスト)

use std::time::Instant;

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine::{Card, Rank, Suit};
use ofc_engine_cpu::{
    CpuConfig, DeadContext, OpeningBook, choose_fantasyland, choose_move_ctx, choose_move_seeded,
};

fn main() {
    let path = std::env::args().nth(1).expect("ブックのパスを引数で渡す");
    let started = Instant::now();
    let book = OpeningBook::load(std::path::Path::new(&path)).expect("ブックを読めること");
    println!(
        "ロード: {} 正準手, {:.1} 秒",
        book.len(),
        started.elapsed().as_secs_f64()
    );

    // カバレッジ: ランダムな初手 20,000 手が全部引けるか
    let mut deck: Vec<Card> = {
        let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
        let mut d: Vec<Card> = "23456789TJQKA"
            .chars()
            .flat_map(|r| {
                suits.iter().map(move |s| {
                    format!(
                        "{r}{}",
                        match s {
                            Suit::Spades => 's',
                            Suit::Hearts => 'h',
                            Suit::Diamonds => 'd',
                            Suit::Clubs => 'c',
                        }
                    )
                })
            })
            .map(|w| w.parse().unwrap())
            .collect();
        d.push(Card::Joker);
        d.push(Card::Joker);
        d
    };
    let cfg = CpuConfig::standard();
    let mut rng: u64 = 0xB00C;
    let mut misses = 0u32;
    let mut lookup_ns = 0u128;
    for _ in 0..20_000 {
        // 決定的シャッフルの先頭 5 枚
        for i in (1..deck.len()).rev() {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let j = (rng % (i as u64 + 1)) as usize;
            deck.swap(i, j);
        }
        let hand: Vec<Card> = deck[..5].to_vec();
        let t = Instant::now();
        if book.lookup(&hand, &cfg).is_none() {
            misses += 1;
        }
        lookup_ns += t.elapsed().as_nanos();
    }
    println!(
        "カバレッジ: 20,000 手中ミス {misses} / lookup 平均 {:.1} µs",
        lookup_ns as f64 / 20_000.0 / 1000.0
    );

    ofc_engine_cpu::set_opening_book(book);

    // 対戦: book+rollout(P0) vs 貪欲(P1)
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let strong = CpuConfig::standard();
    let greedy = {
        let mut c = CpuConfig::standard();
        c.strength = 0.8;
        c
    };
    let mut total = 0i64;
    let mut fouls = [0u64; 2];
    let mut initial_ns = (0u64, 0u128, 0u128);
    const SESSIONS: u64 = 40;
    const HANDS: u64 = 20;
    for session in 0..SESSIONS {
        let mut fl_next = [0u8; 2];
        for hand in 0..HANDS {
            let seed = session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, 2, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, 2, seed).unwrap()
            };
            let mut discards: [Vec<Card>; 2] = [vec![], vec![]];
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                let p = state.current_player();
                if state.street() == Street::Fantasyland {
                    let cfgp = if p == 0 { &strong } else { &greedy };
                    let (pl, d) = choose_fantasyland(state.dealt_cards(), cfgp, move_seed).unwrap();
                    state.apply_fantasyland(&pl, &d).unwrap();
                } else if p == 0 {
                    let mut dead = discards[0].clone();
                    let ob = state.board(1);
                    dead.extend(ob.top().iter().chain(ob.middle()).chain(ob.bottom()));
                    let ctx = DeadContext {
                        jokers: 2,
                        dead,
                        hand_seed: Some(seed),
                        opponent: None,
                    };
                    let is_initial = state.street() == Street::Initial;
                    let t = Instant::now();
                    let (pl, d) = choose_move_ctx(
                        state.board(0),
                        state.dealt_cards(),
                        state.street(),
                        &ctx,
                        &strong,
                        move_seed,
                    )
                    .unwrap();
                    if is_initial {
                        let ns = t.elapsed().as_nanos();
                        initial_ns = (initial_ns.0 + 1, initial_ns.1 + ns, initial_ns.2.max(ns));
                    }
                    if let Some(card) = d {
                        discards[0].push(card);
                    }
                    state.apply(&pl, d).unwrap();
                } else {
                    let (pl, d) = choose_move_seeded(
                        state.board(1),
                        state.dealt_cards(),
                        state.street(),
                        &greedy,
                        move_seed,
                    )
                    .unwrap();
                    state.apply(&pl, d).unwrap();
                }
                move_seed = move_seed.wrapping_add(1);
            }
            let resolved: Vec<_> = (0..2)
                .map(|p| evaluate_board(state.board(p), &[], &royalty, &fl_rules).unwrap())
                .collect();
            for p in 0..2 {
                if resolved[p].foul {
                    fouls[p] += 1;
                }
            }
            let (p0, _) = score_pair(
                &resolved[0].resolved,
                &resolved[1].resolved,
                &royalty,
                &scoring,
            )
            .unwrap();
            total += i64::from(p0);
            for p in 0..2 {
                let board = &resolved[p].resolved;
                fl_next[p] = if fl_next[p] > 0 {
                    if fantasyland_stay(board, &fl_rules).unwrap_or(false) {
                        fl_next[p]
                    } else {
                        0
                    }
                } else {
                    fantasyland_entry(board, &fl_rules)
                        .unwrap_or(None)
                        .unwrap_or(0)
                };
            }
        }
    }
    println!(
        "book+rollout vs 貪欲(0.8): {:+.3}/hand (foul {:.1}% vs {:.1}%)",
        total as f64 / (SESSIONS * HANDS) as f64,
        fouls[0] as f64 * 100.0 / (SESSIONS * HANDS) as f64,
        fouls[1] as f64 * 100.0 / (SESSIONS * HANDS) as f64
    );
    println!(
        "初手: {} 回, 平均 {:.2} ms, 最大 {:.2} ms",
        initial_ns.0,
        initial_ns.1 as f64 / initial_ns.0.max(1) as f64 / 1e6,
        initial_ns.2 as f64 / 1e6
    );
    let _ = Rank::Two;
}
