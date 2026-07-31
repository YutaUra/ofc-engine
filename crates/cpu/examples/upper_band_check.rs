//! strength 上位帯(貪欲↔rollout 混合)の平滑性チェック。
//! 各 strength を共通の相手(s=0.8 = 純貪欲)と対戦させ、
//! 得点が崖なしに単調増加することを確認する。

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, DeadContext, choose_fantasyland, choose_move_ctx};

const STRENGTHS: [f64; 6] = [0.7, 0.8, 0.85, 0.9, 0.95, 1.0];
const SESSIONS: u64 = 40;
const HANDS: u64 = 20;
const JOKERS: u8 = 2;

fn main() {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();

    let mut opponent = CpuConfig::standard();
    opponent.strength = 0.8;

    println!("対戦相手: s=0.8(純貪欲) / 各 {}", SESSIONS * HANDS);
    for (si, &strength) in STRENGTHS.iter().enumerate() {
        let mut cfg = CpuConfig::standard();
        cfg.strength = strength;
        let configs = [cfg, opponent.clone()];

        let mut total = 0i64;
        let mut fouls = [0u64; 2];
        for session in 0..SESSIONS {
            let mut fl_next = [0u8; 2];
            for hand in 0..HANDS {
                let seed = (si as u64) * 10_000_000 + session * 1_000 + hand;
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
                            choose_fantasyland(state.dealt_cards(), &configs[p], move_seed)
                                .unwrap();
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
                        let (pl, d) = choose_move_ctx(
                            state.board(p),
                            state.dealt_cards(),
                            state.street(),
                            &ctx,
                            &configs[p],
                            move_seed,
                        )
                        .unwrap();
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
            "s={strength:>4}: {:+.3}/hand (foul {:.1}%)",
            total as f64 / (SESSIONS * HANDS) as f64,
            fouls[0] as f64 * 100.0 / (SESSIONS * HANDS) as f64
        );
    }
}
