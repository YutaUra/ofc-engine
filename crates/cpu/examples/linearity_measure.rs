//! ワープ較正用の測定: 内部目盛りの各点について、s=0(最弱)基準の
//! 平均利得 g(internal) を測る。出力は較正スクリプトで読む前提の TSV。
//! 引数: ブックのパス

use ofc_engine::Card;
use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, DeadContext, OpeningBook, choose_fantasyland, choose_move_ctx};

const JOKERS: u8 = 2;

/// 現行ワープ W0 の逆関数(内部目盛り → user 目盛り)。
/// W0: [0,0.5] 恒等 / [0.5,0.7]→[0.5,0.9] / [0.7,1.0]→[0.9,1.0]
fn user_for_internal(u: f64) -> f64 {
    if u <= 0.5 {
        u
    } else if u <= 0.9 {
        0.5 + (u - 0.5) * 0.2 / 0.4
    } else {
        0.7 + (u - 0.9) * 0.3 / 0.1
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("ブックのパス");
    let book = OpeningBook::load(std::path::Path::new(&path)).expect("ブック読込");
    ofc_engine_cpu::set_opening_book(book);

    // mode "user": 現行ワープのまま user 目盛りを直接測る(較正後の検証用)
    let user_mode = std::env::args().nth(2).is_some_and(|m| m == "user");
    let targets = [
        0.0, 0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8, 0.85, 0.9, 0.95, 1.0,
    ];
    println!(
        "{}\tgain\tse\thands",
        if user_mode { "user" } else { "internal" }
    );
    for (ti, &t) in targets.iter().enumerate() {
        let strength = if user_mode { t } else { user_for_internal(t) };
        let sessions: u64 = if user_mode || t > 0.8 { 40 } else { 80 };
        let (gain, se, hands) = measure(strength, ti as u64, sessions);
        println!("{t}\t{gain:.4}\t{se:.4}\t{hands}");
    }
}

/// user 目盛り strength vs s=0 の平均利得(セッション平均の SE 付き)。
fn measure(user_strength: f64, seed_base: u64, sessions: u64) -> (f64, f64, u64) {
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let mut cfg = CpuConfig::standard();
    cfg.strength = user_strength;
    let mut weak = CpuConfig::standard();
    weak.strength = 0.0;
    let configs = [cfg, weak];

    const HANDS: u64 = 20;
    let mut session_means = Vec::with_capacity(sessions as usize);
    for session in 0..sessions {
        let mut fl_next = [0u8; 2];
        let mut session_total = 0i64;
        for hand in 0..HANDS {
            let seed = seed_base * 100_000_000 + session * 1_000 + hand;
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
            let (p0, _) = score_pair(
                &resolved[0].resolved,
                &resolved[1].resolved,
                &royalty,
                &scoring,
            )
            .unwrap();
            session_total += i64::from(p0);
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
        session_means.push(session_total as f64 / HANDS as f64);
    }
    let n = session_means.len() as f64;
    let mean = session_means.iter().sum::<f64>() / n;
    let var = session_means
        .iter()
        .map(|m| (m - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0);
    (mean, (var / n).sqrt(), sessions * HANDS)
}
