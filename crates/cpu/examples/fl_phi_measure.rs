//! 突入レベル別の FL 連鎖価値 φ̂ の実測(s=1 同士の自己対戦)。
//! 突入 1 回につき、継続が切れるまでの FL ハンド得点合計を突入レベルに帰属させる。
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
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let cfg = CpuConfig::standard();

    // 突入レベル(14..=17)別: (連鎖数, 得点合計, FLハンド数)
    let mut chains = [(0u64, 0i64, 0u64); 4];
    // 連鎖の途中経過: entry_level[p] = Some(突入時レベル)
    let mut normal_hands = 0u64;
    let mut normal_score = 0i64;

    for session in 0..SESSIONS {
        let mut fl_next = [0u8; 2];
        let mut entry_level: [Option<u8>; 2] = [None, None];
        for hand in 0..HANDS {
            let seed = session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, 2, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, 2, seed).unwrap()
            };
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                if state.street() == Street::Fantasyland {
                    let (p, d) = choose_fantasyland(state.dealt_cards(), &cfg, move_seed).unwrap();
                    state.apply_fantasyland(&p, &d).unwrap();
                } else {
                    let (p, d) = choose_move_seeded(
                        state.board(state.current_player()),
                        state.dealt_cards(),
                        state.street(),
                        &cfg,
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

    println!(
        "平場ハンド平均(基準値 g 相当、対称なので ~0): {:+.3}",
        normal_score as f64 / normal_hands as f64
    );
    println!("\n== 突入レベル別の FL 連鎖価値 φ̂(連鎖全体の得点合計 / 連鎖数) ==");
    for (i, label) in ["QQ(14)", "KK(15)", "AA(16)", "3枚(17)"].iter().enumerate() {
        let (n, total, fl_hands) = chains[i];
        if n == 0 {
            println!("{label}: サンプルなし");
        } else {
            println!(
                "{label}: φ̂ = {:+.2}/突入  (連鎖数 {n}, 平均連鎖長 {:.2} ハンド)",
                total as f64 / n as f64,
                fl_hands as f64 / n as f64
            );
        }
    }
}
