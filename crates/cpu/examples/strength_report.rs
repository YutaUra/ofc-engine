//! strength 0.0..=1.0(0.1 刻み)の総合レポート:
//! 設定 φ / 平均利得(vs s=0.5)/ FL 突入内訳・継続 / FL 中の平均利得。
//! 引数: オープニングブックのパス(省略可)

use ofc_engine::Card;
use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::{FantasylandRules, fantasyland_entry, fantasyland_stay};
use ofc_engine::game::{GameState, Street};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine_cpu::{CpuConfig, DeadContext, OpeningBook, choose_fantasyland, choose_move_ctx};

const SESSIONS: u64 = 40;
const HANDS: u64 = 20;
const JOKERS: u8 = 2;

fn main() {
    if let Some(path) = std::env::args().nth(1) {
        let book = OpeningBook::load(std::path::Path::new(&path)).expect("ブック読込");
        ofc_engine_cpu::set_opening_book(book);
        println!("オープニングブック: 使用");
    } else {
        println!("オープニングブック: なし");
    }
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let fl_rules = FantasylandRules::standard_progressive();
    let mut opponent = CpuConfig::standard();
    opponent.strength = 0.5;

    println!("対戦相手: s=0.5 / 各 {} ハンド(JK2)\n", SESSIONS * HANDS);
    println!(
        "{:>5} | {:>22} | {:>8} | {:>19} | {:>4} | {:>10}",
        "s", "φ(QQ/KK/AA/17)", "利得/hand", "FL突入(QQ/KK/AA/17)", "継続", "FL中利得"
    );

    for step in 0..=10u64 {
        let strength = step as f64 / 10.0;
        let mut cfg = CpuConfig::standard();
        cfg.strength = strength;
        let phi = [
            cfg.fl_value_for_cards(14),
            cfg.fl_value_for_cards(15),
            cfg.fl_value_for_cards(16),
            cfg.fl_value_for_cards(17),
        ];
        let configs = [cfg, opponent.clone()];

        let mut total = 0i64;
        let mut entries = [0u64; 4];
        let mut stays = 0u64;
        let mut fl_hands = 0u64;
        let mut fl_score = 0i64;

        for session in 0..SESSIONS {
            let mut fl_next = [0u8; 2];
            for hand in 0..HANDS {
                let seed = step * 10_000_000 + session * 1_000 + hand;
                let p0_in_fl = fl_next[0] > 0;
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
                total += i64::from(p0);
                if p0_in_fl {
                    fl_hands += 1;
                    fl_score += i64::from(p0);
                }
                for p in 0..2 {
                    let board = &resolved[p].resolved;
                    fl_next[p] = if fl_next[p] > 0 {
                        if fantasyland_stay(board, &fl_rules).unwrap_or(false) {
                            if p == 0 {
                                stays += 1;
                            }
                            fl_next[p]
                        } else {
                            0
                        }
                    } else {
                        let cards = fantasyland_entry(board, &fl_rules)
                            .unwrap_or(None)
                            .unwrap_or(0);
                        if p == 0 && (14..=17).contains(&cards) {
                            entries[(cards - 14) as usize] += 1;
                        }
                        cards
                    };
                }
            }
        }
        println!(
            "{:>5} | {:>4.1}/{:>4.1}/{:>4.1}/{:>4.1} | {:>+8.3} | {:>4}/{:>4}/{:>4}/{:>2} | {:>4} | {:>+9.2}",
            strength,
            phi[0],
            phi[1],
            phi[2],
            phi[3],
            total as f64 / (SESSIONS * HANDS) as f64,
            entries[0],
            entries[1],
            entries[2],
            entries[3],
            stays,
            if fl_hands > 0 {
                fl_score as f64 / fl_hands as f64
            } else {
                0.0
            }
        );
    }
}
