//! P1: rollout 予算スケーリングの計測。
//! 予算違いの s=1 同士を直接対戦させ、サンプル不足の伸びしろを測る。
//! 引数: ブックのパス

use std::time::Instant;

use ofc_engine::Card;
use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{
    CpuConfig, DeadContext, OpeningBook, RolloutBudget, choose_fantasyland, choose_move_ctx,
};

const JOKERS: u8 = 2;
const SESSIONS: u64 = 30;
const HANDS: u64 = 20;

fn main() {
    if let Some(path) = std::env::args().nth(1) {
        let book = OpeningBook::load(std::path::Path::new(&path)).expect("ブック読込");
        ofc_engine_cpu::set_opening_book(book);
    }
    let base = RolloutBudget::default(); // K=8, [60,60,80,200]
    let candidates = [
        (
            "K16 x2samples",
            RolloutBudget {
                top_k: 16,
                samples: [120, 120, 160, 400],
                opponent_samples: 40,
            },
        ),
        (
            "K16 x4samples",
            RolloutBudget {
                top_k: 16,
                samples: [240, 240, 320, 800],
                opponent_samples: 40,
            },
        ),
        (
            "K20 x8samples",
            RolloutBudget {
                top_k: 20,
                samples: [480, 480, 640, 1600],
                opponent_samples: 40,
            },
        ),
    ];
    for (label, budget) in candidates {
        run(label, base.clone(), budget);
    }
}

fn run(label: &str, base: RolloutBudget, big: RolloutBudget) {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let mut cfg_big = CpuConfig::standard();
    cfg_big.rollout = big;
    let mut cfg_base = CpuConfig::standard();
    cfg_base.rollout = base;
    let configs = [cfg_big, cfg_base];

    let mut total = 0i64;
    let mut big_time = (0u64, 0u128);
    for session in 0..SESSIONS {
        let mut fl_next = [0u8; 2];
        for hand in 0..HANDS {
            let seed = session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, JOKERS, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, JOKERS, seed).unwrap()
            };
            let mut discards: [Vec<Card>; 2] = [vec![], vec![]];
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                let p = state.current_player();
                if state.street() == Street::Fantasyland {
                    let (pl, d) =
                        choose_fantasyland(state.dealt_cards(), &configs[p], move_seed).unwrap();
                    state.apply_fantasyland(&pl, &d).unwrap();
                } else {
                    let opp = 1 - p;
                    let mut dead = discards[p].clone();
                    let ob = state.board(opp);
                    dead.extend(ob.top().iter().chain(ob.middle()).chain(ob.bottom()));
                    let ctx = DeadContext {
                        jokers: JOKERS,
                        dead,
                        hand_seed: Some(seed),
                        opponent: None,
                    };
                    let t = Instant::now();
                    let (pl, d) = choose_move_ctx(
                        state.board(p),
                        state.dealt_cards(),
                        state.street(),
                        &ctx,
                        &configs[p],
                        move_seed,
                    )
                    .unwrap();
                    if p == 0 {
                        big_time = (big_time.0 + 1, big_time.1 + t.elapsed().as_nanos());
                    }
                    if let Some(card) = d {
                        discards[p].push(card);
                    }
                    state.apply(&pl, d).unwrap();
                }
                move_seed = move_seed.wrapping_add(1);
            }
            let resolved: Vec<_> = (0..2)
                .map(|p| evaluate_board(state.board(p), &[], &royalty, &fl_rules).unwrap())
                .collect();
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
        "{label}: 大予算側 {:+.3}/hand, 平均 {:.1} ms/手",
        total as f64 / (SESSIONS * HANDS) as f64,
        big_time.1 as f64 / big_time.0.max(1) as f64 / 1e6
    );
}
