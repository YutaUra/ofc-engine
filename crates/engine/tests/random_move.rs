//! ランダム合法配置(random_move)のテスト。
//! CPU の土台・デバッグ・シミュレーション用に「常に合法な手」を決定的に返す。

use ofc_engine::game::{GameState, Street, random_move};

#[test]
fn 同一シードなら同じ手が返る() {
    let state = GameState::new(2, 0, 5).unwrap();
    assert_eq!(random_move(&state, 1), random_move(&state, 1));
}

#[test]
fn 初手は5枚配置で捨て札なし() {
    let state = GameState::new(2, 0, 5).unwrap();
    let (placements, discard) = random_move(&state, 1).unwrap();
    assert_eq!(placements.len(), 5);
    assert_eq!(discard, None);
}

#[test]
fn 街では2枚配置と捨て札1枚() {
    let mut state = GameState::new(2, 0, 5).unwrap();
    for i in 0..2 {
        let (p, d) = random_move(&state, i).unwrap();
        state.apply(&p, d).unwrap();
    }
    assert_eq!(state.street(), Street::Draw(1));
    let (placements, discard) = random_move(&state, 9).unwrap();
    assert_eq!(placements.len(), 2);
    assert!(discard.is_some());
}

#[test]
fn ランダムな手は常に合法でゲームを完走できる() {
    // 多数のゲームで、返された手が一度も apply に拒否されないこと
    for game_seed in 0..50 {
        let mut state =
            GameState::new(2, if game_seed % 2 == 0 { 0 } else { 2 }, game_seed).unwrap();
        let mut move_seed = 1000 + game_seed;
        while state.street() != Street::Finished {
            let (placements, discard) = random_move(&state, move_seed).expect("未完走なら手が返る");
            state
                .apply(&placements, discard)
                .expect("random_move の手は常に合法");
            move_seed += 1;
        }
        assert!(state.board(0).is_complete());
        assert!(state.board(1).is_complete());
    }
}

#[test]
fn 完走後はnoneが返る() {
    let mut state = GameState::new(2, 0, 3).unwrap();
    let mut seed = 0;
    while state.street() != Street::Finished {
        let (p, d) = random_move(&state, seed).unwrap();
        state.apply(&p, d).unwrap();
        seed += 1;
    }
    assert_eq!(random_move(&state, 0), None);
}

#[test]
fn シードが違えば手が変わりうる() {
    let state = GameState::new(2, 0, 5).unwrap();
    // 5 枚を 3 行に割り振る組み合わせは多いため、異なる seed 100 個が
    // すべて同一の手になることは実質ありえない
    let baseline = random_move(&state, 0);
    assert!((1..100).any(|s| random_move(&state, s) != baseline));
}
