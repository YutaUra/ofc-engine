//! strength の検証: 5 段階の総当たり自己対戦で単調性と計算時間を測る。
//! `cargo run --release -p ofc-engine-cpu --example strength_ladder`

use std::time::Instant;

use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, choose_move_seeded};

const STRENGTHS: [f64; 5] = [0.0, 0.25, 0.5, 0.75, 1.0];
const GAMES_PER_PAIR: u64 = 400;

fn main() {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();

    let mut totals = [0i64; STRENGTHS.len()];
    let mut matrix = [[0i64; 5]; 5];
    let mut move_count = 0u64;
    let mut move_time_ns = 0u128;
    let mut max_move_ns = 0u128;

    for (i, &si) in STRENGTHS.iter().enumerate() {
        for (j, &sj) in STRENGTHS.iter().enumerate() {
            if i == j {
                continue;
            }
            let mut config_i = CpuConfig::standard();
            config_i.strength = si;
            let mut config_j = CpuConfig::standard();
            config_j.strength = sj;

            for game in 0..GAMES_PER_PAIR {
                // 座席と配牌の偏りを消すため、seed はペアとゲーム番号から決める
                let game_seed = (i as u64) * 1_000_000 + (j as u64) * 10_000 + game;
                let mut state = GameState::new(2, 0, game_seed).unwrap();
                let mut move_seed = game_seed.wrapping_mul(31);
                while state.street() != Street::Finished {
                    let config = if state.current_player() == 0 {
                        &config_i
                    } else {
                        &config_j
                    };
                    let started = Instant::now();
                    let (p, d) = choose_move_seeded(
                        state.board(state.current_player()),
                        state.dealt_cards(),
                        state.street(),
                        config,
                        move_seed,
                    )
                    .unwrap();
                    let elapsed = started.elapsed().as_nanos();
                    move_time_ns += elapsed;
                    max_move_ns = max_move_ns.max(elapsed);
                    move_count += 1;
                    state.apply(&p, d).unwrap();
                    move_seed = move_seed.wrapping_add(1);
                }
                let (p0, _) =
                    score_pair(state.board(0), state.board(1), &royalty, &scoring).unwrap();
                totals[i] += i64::from(p0);
                totals[j] -= i64::from(p0);
                matrix[i][j] += i64::from(p0);
            }
        }
    }

    println!("== 対戦マトリクス(行プレイヤーの合計得点 / {GAMES_PER_PAIR} games) ==");
    print!("{:>8}", "");
    for s in STRENGTHS {
        print!("{s:>8}");
    }
    println!();
    for (i, s) in STRENGTHS.iter().enumerate() {
        print!("{s:>8}");
        for j in 0..STRENGTHS.len() {
            if i == j {
                print!("{:>8}", "-");
            } else {
                print!("{:>8}", matrix[i][j]);
            }
        }
        println!();
    }

    println!("\n== 強さ別の総合得点(全対戦の合計。単調増加が期待値) ==");
    for (i, s) in STRENGTHS.iter().enumerate() {
        let games = (STRENGTHS.len() - 1) as i64 * 2 * GAMES_PER_PAIR as i64;
        println!(
            "strength {s:>4}: total {:>7}  (avg {:+.3}/game)",
            totals[i],
            totals[i] as f64 / games as f64
        );
    }

    println!("\n== 計算時間 ==");
    println!(
        "着手回数 {move_count}, 平均 {:.1} µs/手, 最大 {:.1} µs/手",
        move_time_ns as f64 / move_count as f64 / 1000.0,
        max_move_ns as f64 / 1000.0
    );
}
