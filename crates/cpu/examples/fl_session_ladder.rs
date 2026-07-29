//! FL 込みのセッション自己対戦で strength の単調性と計算時間を検証する。
//! FL 突入・継続をハンドを跨いで引き継ぐ(実戦と同じ構造)。
//! デッキはジョーカー 2 枚入り(チャイポ標準)。ジョーカーが盤面に残る
//! ため、採点・FL 判定は evaluate_board の解決後盤面に対して行う。
//! `cargo run --release -p ofc-engine-cpu --example fl_session_ladder`

use std::time::Instant;

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, choose_fantasyland, choose_move_seeded};

const STRENGTHS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
const SESSIONS_PER_PAIR: u64 = 200;
const HANDS_PER_SESSION: u64 = 20;
/// FL 継続時の配布枚数(標準では 14 枚固定が主流)。
const FL_STAY_CARDS: u8 = 14;
/// デッキのジョーカー枚数(チャイポ標準は 2)。
const JOKERS: u8 = 2;

fn main() {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();

    let mut totals = [0i64; STRENGTHS.len()];
    // 突入種別: [QQ(14), KK(15), AA(16), trips(17)] + 継続
    let mut fl_entries = [[0u64; 4]; STRENGTHS.len()];
    let mut fl_stays = [0u64; STRENGTHS.len()];
    let mut matrix = [[0i64; STRENGTHS.len()]; STRENGTHS.len()];
    let mut normal_moves = (0u64, 0u128, 0u128); // (count, total_ns, max_ns)
    let mut fl_moves = (0u64, 0u128, 0u128);

    for (i, &si) in STRENGTHS.iter().enumerate() {
        for (j, &sj) in STRENGTHS.iter().enumerate() {
            // 同一 strength 同士(i == j)も対戦させる(期待値 ~0 の検算を兼ねる)
            let mut cfg_i = CpuConfig::standard();
            cfg_i.strength = si;
            let mut cfg_j = CpuConfig::standard();
            cfg_j.strength = sj;

            for session in 0..SESSIONS_PER_PAIR {
                let mut fl_next = [0u8; 2];
                for hand in 0..HANDS_PER_SESSION {
                    let seed = ((i * 5 + j) as u64) * 1_000_000 + session * 1_000 + hand;
                    let mut state = if fl_next.iter().any(|&f| f > 0) {
                        GameState::new_with_fantasyland(2, JOKERS, seed, &fl_next).unwrap()
                    } else {
                        GameState::new(2, JOKERS, seed).unwrap()
                    };
                    let mut move_seed = seed.wrapping_mul(131);
                    while state.street() != Street::Finished {
                        let cfg = if state.current_player() == 0 {
                            &cfg_i
                        } else {
                            &cfg_j
                        };
                        let started = Instant::now();
                        if state.street() == Street::Fantasyland {
                            let (p, d) =
                                choose_fantasyland(state.dealt_cards(), cfg, move_seed).unwrap();
                            let ns = started.elapsed().as_nanos();
                            fl_moves = (fl_moves.0 + 1, fl_moves.1 + ns, fl_moves.2.max(ns));
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
                            let ns = started.elapsed().as_nanos();
                            normal_moves = (
                                normal_moves.0 + 1,
                                normal_moves.1 + ns,
                                normal_moves.2.max(ns),
                            );
                            state.apply(&p, d).unwrap();
                        }
                        move_seed = move_seed.wrapping_add(1);
                    }

                    // ジョーカーが残る盤面は解決後の盤面で採点・FL 判定する
                    let resolved: Vec<_> = (0..2)
                        .map(|p| {
                            evaluate_board(state.board(p), &[], &royalty, &fl_rules)
                                .unwrap()
                                .resolved
                        })
                        .collect();
                    let (p0, _) =
                        score_pair(&resolved[0], &resolved[1], &royalty, &scoring).unwrap();
                    totals[i] += i64::from(p0);
                    totals[j] -= i64::from(p0);
                    matrix[i][j] += i64::from(p0);

                    // 次ハンドの FL 状態(突入 / 継続)を決める
                    for (player, strength_idx) in [(0usize, i), (1usize, j)] {
                        let board = &resolved[player];
                        fl_next[player] = if fl_next[player] > 0 {
                            if fantasyland_stay(board, &fl_rules).unwrap_or(false) {
                                fl_stays[strength_idx] += 1;
                                FL_STAY_CARDS
                            } else {
                                0
                            }
                        } else {
                            let cards = fantasyland_entry(board, &fl_rules)
                                .unwrap_or(None)
                                .unwrap_or(0);
                            if (14..=17).contains(&cards) {
                                fl_entries[strength_idx][(cards - 14) as usize] += 1;
                            }
                            cards
                        };
                    }
                }
            }
        }
    }

    let hands_per_pair = (SESSIONS_PER_PAIR * HANDS_PER_SESSION) as f64;
    // 各 strength は全列 + 全行(対角は両側)に登場する
    let games = (STRENGTHS.len() * 2) as f64 * hands_per_pair;
    println!("== FL 込みセッションの強さ別総合得点(単調増加が期待値) ==");
    for (i, s) in STRENGTHS.iter().enumerate() {
        println!(
            "strength {s:>4}: total {:>7}  (avg {:+.3}/hand)",
            totals[i],
            totals[i] as f64 / games
        );
    }

    println!("\n== FL 突入・継続の回数(種別ごと) ==");
    println!(
        "{:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
        "strength", "QQ(14)", "KK(15)", "AA(16)", "3枚(17)", "突入計", "継続"
    );
    for (i, s) in STRENGTHS.iter().enumerate() {
        let total: u64 = fl_entries[i].iter().sum();
        println!(
            "{:>8} {:>8} {:>8} {:>8} {:>8} {:>8} {:>8}",
            s,
            fl_entries[i][0],
            fl_entries[i][1],
            fl_entries[i][2],
            fl_entries[i][3],
            total,
            fl_stays[i]
        );
    }

    println!("\n== ヘッズアップ利得マトリクス(行プレイヤーの平均得点/hand) ==");
    print!("{:>8}", "");
    for s in STRENGTHS {
        print!("{s:>8}");
    }
    println!();
    for (i, s) in STRENGTHS.iter().enumerate() {
        print!("{s:>8}");
        for row in matrix[i].iter() {
            print!("{:>8.3}", *row as f64 / hands_per_pair);
        }
        println!();
    }

    println!("\n== 計算時間 ==");
    println!(
        "通常手: {} 回, 平均 {:.1} µs, 最大 {:.1} µs",
        normal_moves.0,
        normal_moves.1 as f64 / normal_moves.0.max(1) as f64 / 1000.0,
        normal_moves.2 as f64 / 1000.0
    );
    println!(
        "FL 手 : {} 回, 平均 {:.1} ms, 最大 {:.1} ms",
        fl_moves.0,
        fl_moves.1 as f64 / fl_moves.0.max(1) as f64 / 1_000_000.0,
        fl_moves.2 as f64 / 1_000_000.0
    );
}
