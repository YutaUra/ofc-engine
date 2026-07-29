//! FL(Fantasyland)配置方策のテスト。
//! strength 1 = 完全解(14 枚では全列挙と一致することを保証)、
//! strength 0 = 最適値の 1/3〜2/3 の帯からのランダム選択。

use ofc_engine::game::{GameState, RowKind, Street};
use ofc_engine::{Board, Card};
use ofc_engine_cpu::{CpuConfig, choose_fantasyland, fantasyland_value};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

fn config() -> CpuConfig {
    CpuConfig::standard()
}

/// 選ばれた手から盤面を組み立てる。
fn to_board(placements: &[ofc_engine::game::Placement]) -> Board {
    let mut rows: [Vec<Card>; 3] = [vec![], vec![], vec![]];
    for p in placements {
        let idx = match p.row {
            RowKind::Top => 0,
            RowKind::Middle => 1,
            RowKind::Bottom => 2,
        };
        rows[idx].push(p.card);
    }
    let [top, middle, bottom] = rows;
    Board::new(top, middle, bottom).unwrap()
}

#[test]
fn fl配置は13枚配置と残り捨てを返す() {
    let dealt = cards(&[
        "As", "Ad", "Ah", "Kc", "Ks", "Qd", "Qh", "Jh", "Th", "9c", "8s", "7d", "6h", "2c",
    ]);
    let (placements, discards) = choose_fantasyland(&dealt, &config(), 1).unwrap();
    assert_eq!(placements.len(), 13);
    assert_eq!(discards.len(), 1);
    let board = to_board(&placements);
    assert!(board.is_complete());
}

#[test]
fn 強さ1はファウルしない最適盤面を選ぶ() {
    // AAA + KK + QQ + straight 素材。最適解はかなり高い素点になるはず
    let dealt = cards(&[
        "As", "Ad", "Ah", "Kc", "Ks", "Qd", "Qh", "Jh", "Th", "9c", "8s", "7d", "6h", "2c",
    ]);
    let (placements, _) = choose_fantasyland(&dealt, &config(), 1).unwrap();
    let board = to_board(&placements);
    assert_eq!(ofc_engine::foul::check_foul(&board), Ok(false));
    // AAA(フルハウス素材)があるので素点 0 はありえない
    assert!(fantasyland_value(&board, &config()) > 0.0);
}

#[test]
fn 強さ1は14枚の全列挙と同じ価値になる() {
    // ビーム探索が完全解から外れていないことを、独立実装の全列挙で検証する
    let mut rng: u64 = 0xF1;
    for _ in 0..5 {
        // 決定的にランダムな 14 枚(Joker なし)を作る
        let mut deck = full_deck();
        let mut dealt = Vec::with_capacity(14);
        for _ in 0..14 {
            rng ^= rng << 13;
            rng ^= rng >> 7;
            rng ^= rng << 17;
            let i = (rng % deck.len() as u64) as usize;
            dealt.push(deck.swap_remove(i));
        }

        let (placements, _) = choose_fantasyland(&dealt, &config(), 1).unwrap();
        let chosen = fantasyland_value(&to_board(&placements), &config());
        let best = exhaustive_best_value(&dealt);
        assert!(
            (chosen - best).abs() < 1e-9,
            "ビーム解 {chosen} が全列挙の最適 {best} と一致しない: {dealt:?}"
        );
    }
}

#[test]
fn 強さ0は最適値のおよそ3分の1から3分の2の盤面を選ぶ() {
    let dealt = cards(&[
        "As", "Ad", "Ah", "Kc", "Ks", "Qd", "Qh", "Jh", "Th", "9c", "8s", "7d", "6h", "2c",
    ]);
    let cfg = config();
    let best = {
        let (p, _) = choose_fantasyland(&dealt, &cfg, 1).unwrap();
        fantasyland_value(&to_board(&p), &cfg)
    };
    let mut weak = cfg.clone();
    weak.strength = 0.0;
    for seed in 0..20 {
        let (p, _) = choose_fantasyland(&dealt, &weak, seed).unwrap();
        let board = to_board(&p);
        // 帯選択でもファウル盤面は選ばない
        assert_eq!(ofc_engine::foul::check_foul(&board), Ok(false));
        let v = fantasyland_value(&board, &weak);
        // 候補の離散性があるため帯 [1/3, 2/3] ± 20% まで許容する
        assert!(
            v >= best * 0.15 && v <= best * 0.85,
            "seed {seed}: value {v} が帯から大きく外れている(best {best})"
        );
    }
}

#[test]
fn 同じシードなら同じfl配置を返す() {
    let dealt = cards(&[
        "As", "Ad", "Ah", "Kc", "Ks", "Qd", "Qh", "Jh", "Th", "9c", "8s", "7d", "6h", "2c",
    ]);
    let mut cfg = config();
    cfg.strength = 0.4;
    assert_eq!(
        choose_fantasyland(&dealt, &cfg, 5),
        choose_fantasyland(&dealt, &cfg, 5)
    );
}

#[test]
fn ゲーム状態のfl手番にそのまま適用できる() {
    for seed in 0..5 {
        let mut state = GameState::new_with_fantasyland(2, 0, seed, &[15, 0]).unwrap();
        assert_eq!(state.street(), Street::Fantasyland);
        let (placements, discards) =
            choose_fantasyland(state.dealt_cards(), &config(), seed).unwrap();
        state.apply_fantasyland(&placements, &discards).unwrap();
        assert!(state.board(0).is_complete());
    }
}

#[test]
fn ジョーカー入りのfl手札でも合法な配置を返す() {
    let mut dealt = cards(&[
        "As", "Ad", "Kc", "Ks", "Qd", "Qh", "Jh", "Th", "9c", "8s", "7d", "6h", "2c",
    ]);
    dealt.push("Xj".parse().unwrap());
    let (placements, discards) = choose_fantasyland(&dealt, &config(), 1).unwrap();
    assert_eq!(placements.len(), 13);
    assert_eq!(discards.len(), 1);
    to_board(&placements); // 盤面として有効(重複なし・3/5/5)
}

// ---- テスト用の独立した全列挙(参照実装) ----

fn full_deck() -> Vec<Card> {
    let ranks = "23456789TJQKA";
    let suits = "shdc";
    ranks
        .chars()
        .flat_map(|r| suits.chars().map(move |s| format!("{r}{s}")))
        .map(|w| w.parse().unwrap())
        .collect()
}

/// C(n,k) の全組み合わせ添字。
fn combos(n: usize, k: usize) -> Vec<Vec<usize>> {
    let mut result = Vec::new();
    let mut current: Vec<usize> = (0..k).collect();
    loop {
        result.push(current.clone());
        // 次の組み合わせへ
        let mut i = k;
        loop {
            if i == 0 {
                return result;
            }
            i -= 1;
            if current[i] != i + n - k {
                break;
            }
            if i == 0 {
                return result;
            }
        }
        current[i] += 1;
        for j in (i + 1)..k {
            current[j] = current[j - 1] + 1;
        }
    }
}

fn exhaustive_best_value(dealt: &[Card]) -> f64 {
    let cfg = config();
    let n = dealt.len();
    let mut best = f64::NEG_INFINITY;
    for bottom_idx in combos(n, 5) {
        let bottom: Vec<Card> = bottom_idx.iter().map(|i| dealt[*i]).collect();
        let rest1: Vec<Card> = (0..n)
            .filter(|i| !bottom_idx.contains(i))
            .map(|i| dealt[i])
            .collect();
        for middle_idx in combos(rest1.len(), 5) {
            let middle: Vec<Card> = middle_idx.iter().map(|i| rest1[*i]).collect();
            let rest2: Vec<Card> = (0..rest1.len())
                .filter(|i| !middle_idx.contains(i))
                .map(|i| rest1[i])
                .collect();
            for top_idx in combos(rest2.len(), 3) {
                let top: Vec<Card> = top_idx.iter().map(|i| rest2[*i]).collect();
                let board = Board::new(top.clone(), middle.clone(), bottom.clone()).unwrap();
                if ofc_engine::foul::check_foul(&board) == Ok(false) {
                    best = best.max(fantasyland_value(&board, &cfg));
                }
            }
        }
    }
    best
}
