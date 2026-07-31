//! P2 A/B: 差分 rollout(相手分布固定 + head-to-head)vs ソリティア rollout。
//! 引数: ブックのパス

use ofc_engine::Card;
use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, DeadContext, OpeningBook, choose_fantasyland, choose_move_ctx};

const JOKERS: u8 = 2;
const HANDS: u64 = 20;

fn main() {
    if let Some(path) = std::env::args().nth(1) {
        let book = OpeningBook::load(std::path::Path::new(&path)).expect("ブック読込");
        ofc_engine_cpu::set_opening_book(book);
    }
    // 対戦 1: 差分 vs ソリティア(同格 s=1)を 150 セッション
    run("差分 vs ソリティア(s=1)", 1.0, 150, 0);
    // 対戦 2: 差分 vs 貪欲(0.8)を 40 セッション(ソリティアの +8.4 と比較)
    run("差分 vs 貪欲(0.8)", 0.8, 40, 7_000_000);
}

fn run(label: &str, opp_strength: f64, sessions: u64, seed_base: u64) {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let cfg = CpuConfig::standard();
    let mut opp_cfg = CpuConfig::standard();
    opp_cfg.strength = opp_strength;

    let mut total = 0i64;
    let mut fouls = [0u64; 2];
    for session in 0..sessions {
        let mut fl_next = [0u8; 2];
        for hand in 0..HANDS {
            let seed = seed_base + session * 1_000 + hand;
            let mut state = if fl_next.iter().any(|&f| f > 0) {
                GameState::new_with_fantasyland(2, JOKERS, seed, &fl_next).unwrap()
            } else {
                GameState::new(2, JOKERS, seed).unwrap()
            };
            let mut discards: [Vec<Card>; 2] = [vec![], vec![]];
            let mut move_seed = seed.wrapping_mul(131);
            while state.street() != Street::Finished {
                let p = state.current_player();
                let my_cfg = if p == 0 { &cfg } else { &opp_cfg };
                if state.street() == Street::Fantasyland {
                    let (pl, d) =
                        choose_fantasyland(state.dealt_cards(), my_cfg, move_seed).unwrap();
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
                        // P0 = 差分評価(相手盤面あり)、P1 = ソリティア
                        opponent: (p == 0).then(|| state.board(opp).clone()),
                    };
                    let (pl, d) = choose_move_ctx(
                        state.board(p),
                        state.dealt_cards(),
                        state.street(),
                        &ctx,
                        my_cfg,
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
        "{label}({} ハンド): {:+.3}/hand (foul {:.1}% vs {:.1}%)",
        sessions * HANDS,
        total as f64 / (sessions * HANDS) as f64,
        fouls[0] as f64 * 100.0 / (sessions * HANDS) as f64,
        fouls[1] as f64 * 100.0 / (sessions * HANDS) as f64
    );
}
