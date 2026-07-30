//! rollout(strength 1.0)の検証: 対貪欲・対 s=0 の得点、ファウル率、
//! 街別の思考時間。`cargo run --release -p ofc-engine-cpu --example rollout_verify`

use std::time::Instant;

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{
    CpuConfig, DeadContext, choose_fantasyland, choose_move_ctx, choose_move_seeded,
};

const SESSIONS: u64 = 40;
const HANDS: u64 = 20;
const JOKERS: u8 = 2;

#[derive(Clone, Copy, PartialEq)]
enum Player {
    Rollout,
    Greedy,
    Weak,
}

fn main() {
    run_match("rollout vs 貪欲(旧 s=1)", [Player::Rollout, Player::Greedy]);
    run_match("rollout vs s=0", [Player::Rollout, Player::Weak]);
}

fn run_match(label: &str, players: [Player; 2]) {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();

    let config_of = |p: Player| -> CpuConfig {
        let mut cfg = CpuConfig::standard();
        if p == Player::Weak {
            cfg.strength = 0.0;
        }
        cfg
    };
    let configs = [config_of(players[0]), config_of(players[1])];

    let mut hands = 0u64;
    let mut total = 0i64;
    let mut fouls = [0u64; 2];
    // rollout の街別時間 [初手, 街1, 街2, 街3, 街4]
    let mut times = [(0u64, 0u128, 0u128); 5];

    for session in 0..SESSIONS {
        let mut fl_next = [0u8; 2];
        for hand in 0..HANDS {
            let seed = session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, JOKERS, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, JOKERS, seed).unwrap()
            };
            let mut discards: [Vec<ofc_engine::Card>; 2] = [vec![], vec![]];
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                let p = state.current_player();
                if state.street() == Street::Fantasyland {
                    let (pl, d) =
                        choose_fantasyland(state.dealt_cards(), &configs[p], move_seed).unwrap();
                    state.apply_fantasyland(&pl, &d).unwrap();
                } else {
                    let mv = if players[p] == Player::Rollout {
                        let opp = 1 - p;
                        let mut dead = discards[p].clone();
                        let ob = state.board(opp);
                        dead.extend(ob.top().iter().chain(ob.middle()).chain(ob.bottom()));
                        let ctx = DeadContext {
                            jokers: JOKERS,
                            dead,
                        };
                        let street_idx = match state.street() {
                            Street::Initial => 0usize,
                            Street::Draw(n) => n as usize,
                            _ => unreachable!(),
                        };
                        let started = Instant::now();
                        let mv = choose_move_ctx(
                            state.board(p),
                            state.dealt_cards(),
                            state.street(),
                            &ctx,
                            &configs[p],
                            move_seed,
                        )
                        .unwrap();
                        let ns = started.elapsed().as_nanos();
                        let t = &mut times[street_idx];
                        *t = (t.0 + 1, t.1 + ns, t.2.max(ns));
                        mv
                    } else {
                        choose_move_seeded(
                            state.board(p),
                            state.dealt_cards(),
                            state.street(),
                            &configs[p],
                            move_seed,
                        )
                        .unwrap()
                    };
                    if let Some(d) = mv.1 {
                        discards[p].push(d);
                    }
                    state.apply(&mv.0, mv.1).unwrap();
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
            hands += 1;

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

    println!("== {label}({hands} ハンド) ==");
    println!(
        "  平均得点(先手視点): {:+.3}/hand | foul率: 先手 {:.1}% / 後手 {:.1}%",
        total as f64 / hands as f64,
        fouls[0] as f64 * 100.0 / hands as f64,
        fouls[1] as f64 * 100.0 / hands as f64
    );
    if players[0] == Player::Rollout {
        for (i, name) in ["初手", "街1", "街2", "街3", "街4"].iter().enumerate() {
            let (n, sum, max) = times[i];
            if n > 0 {
                println!(
                    "  {name}: {n} 回, 平均 {:.1} ms, 最大 {:.1} ms",
                    sum as f64 / n as f64 / 1e6,
                    max as f64 / 1e6
                );
            }
        }
    }
    println!();
}
