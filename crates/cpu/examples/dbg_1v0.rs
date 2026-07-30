//! s=1 vs s=0 の得点分解: ファウル率・ロイヤリティ・行勝ちの内訳を測る。

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::foul::check_foul;
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, choose_fantasyland, choose_move_seeded};

fn main() {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let mut cfg = [CpuConfig::standard(), CpuConfig::standard()];
    cfg[1].strength = 0.0;

    let mut hands = 0u64;
    let mut total = 0i64;
    let mut fouls = [0u64; 2];
    let mut royalty_sum = [0u64; 2];
    let mut both_foul = 0u64;

    for session in 0..500u64 {
        let mut fl_next = [0u8; 2];
        for hand in 0..20u64 {
            let seed = session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, 2, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, 2, seed).unwrap()
            };
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                let p = state.current_player();
                if state.street() == Street::Fantasyland {
                    let (pl, d) =
                        choose_fantasyland(state.dealt_cards(), &cfg[p], move_seed).unwrap();
                    state.apply_fantasyland(&pl, &d).unwrap();
                } else {
                    let (pl, d) = choose_move_seeded(
                        state.board(p),
                        state.dealt_cards(),
                        state.street(),
                        &cfg[p],
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
            let f = [resolved[0].foul, resolved[1].foul];
            for p in 0..2 {
                if f[p] {
                    fouls[p] += 1;
                }
                royalty_sum[p] += u64::from(resolved[p].royalty_total);
            }
            if f[0] && f[1] {
                both_foul += 1;
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
            let _ = check_foul;
        }
    }
    println!(
        "hands={hands} avg(+s1視点)={:+.3}",
        total as f64 / hands as f64
    );
    println!(
        "foul率: s1={:.1}% s0={:.1}% 両者={:.1}%",
        fouls[0] as f64 * 100.0 / hands as f64,
        fouls[1] as f64 * 100.0 / hands as f64,
        both_foul as f64 * 100.0 / hands as f64
    );
    println!(
        "平均ロイヤリティ(非ファウル時のみ計上): s1={:.2} s0={:.2}",
        royalty_sum[0] as f64 / hands as f64,
        royalty_sum[1] as f64 / hands as f64
    );
}
