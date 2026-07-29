//! 突入レベル別 × strength 別の FL 連鎖価値 φ̂ の実測(同 strength 同士の
//! 自己対戦)。突入 1 回につき、継続が切れるまでの FL ハンド得点合計を
//! 突入レベルに帰属させる。0.1 刻み 11 点のグリッドを出力し、
//! CpuConfig::standard の fl_phi_table に貼り付ける。
//! `cargo run --release -p ofc-engine-cpu --example fl_phi_measure`

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, choose_fantasyland, choose_move_seeded};

const SESSIONS: u64 = 4000;
const HANDS: u64 = 20;

fn main() {
    for step in 0..=10u64 {
        let strength = step as f64 / 10.0;
        let mut cfg = CpuConfig::standard();
        cfg.strength = strength;
        measure(strength, &cfg, step);
    }
}

fn measure(strength: f64, cfg: &CpuConfig, seed_base: u64) {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();

    // 突入レベル(14..=17)別: (連鎖数, 得点合計, FLハンド数)
    let mut chains = [(0u64, 0i64, 0u64); 4];
    // 連鎖の途中経過: entry_level[p] = Some(突入時レベル)
    let mut normal_hands = 0u64;
    let mut normal_score = 0i64;

    for session in 0..SESSIONS {
        let mut fl_next = [0u8; 2];
        let mut entry_level: [Option<u8>; 2] = [None, None];
        for hand in 0..HANDS {
            let seed = seed_base * 100_000_000 + session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, 2, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, 2, seed).unwrap()
            };
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                if state.street() == Street::Fantasyland {
                    let (p, d) = choose_fantasyland(state.dealt_cards(), cfg, move_seed).unwrap();
                    state.apply_fantasyland(&p, &d).unwrap();
                } else {
                    let (p, d) = choose_move_seeded(
                        state.board(state.current_player()),
                        state.dealt_cards(),
                        state.street(),
                        cfg,
                        move_seed,
                    )
                    .unwrap();
                    state.apply(&p, d).unwrap();
                }
                move_seed = move_seed.wrapping_add(1);
            }
            let resolved: Vec<_> = (0..2)
                .map(|p| {
                    evaluate_board(state.board(p), &[], &royalty, &fl_rules)
                        .unwrap()
                        .resolved
                })
                .collect();
            let (p0, _) = score_pair(&resolved[0], &resolved[1], &royalty, &scoring).unwrap();
            let scores = [i64::from(p0), -i64::from(p0)];

            for p in 0..2 {
                if let Some(level) = entry_level[p] {
                    // FL 連鎖中のハンド: 得点を突入レベルに帰属
                    let idx = (level - 14) as usize;
                    chains[idx].1 += scores[p];
                    chains[idx].2 += 1;
                } else if fl_next[p] == 0 {
                    normal_hands += 1;
                    normal_score += scores[p];
                }
            }

            // 次ハンドの FL 状態と連鎖の開始・終了
            for p in 0..2 {
                let board = &resolved[p];
                if fl_next[p] > 0 {
                    if fantasyland_stay(board, &fl_rules).unwrap_or(false) {
                        // 継続は配布枚数を維持する
                    } else {
                        fl_next[p] = 0;
                        entry_level[p] = None; // 連鎖終了
                    }
                } else {
                    let cards = fantasyland_entry(board, &fl_rules)
                        .unwrap_or(None)
                        .unwrap_or(0);
                    fl_next[p] = cards;
                    if (14..=17).contains(&cards) {
                        entry_level[p] = Some(cards);
                        chains[(cards - 14) as usize].0 += 1; // 連鎖開始
                    }
                }
            }
        }
    }

    let baseline = normal_score as f64 / normal_hands as f64;
    // 平場基準を差し引いた補正 φ̂(連鎖中も平場を打てたはずの機会費用)
    let phi = |i: usize| -> Option<(f64, u64, f64)> {
        let (n, total, fl_hands) = chains[i];
        (n > 0).then(|| {
            let len = fl_hands as f64 / n as f64;
            (total as f64 / n as f64 - len * baseline, n, len)
        })
    };
    print!("strength {strength:>3}: 平場 {baseline:+.2} |");
    for (i, label) in ["QQ", "KK", "AA", "17"].iter().enumerate() {
        match phi(i) {
            Some((v, n, len)) => print!(" {label} {v:>6.1} (n={n:<4} len={len:.1})"),
            None => print!(" {label}    --- (n=0)"),
        }
    }
    println!();
}
