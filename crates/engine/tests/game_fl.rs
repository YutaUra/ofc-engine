//! Fantasyland ハンドのゲーム進行テスト。
//! FL プレイヤーは開始時に指定枚数(14〜17)を一括配布され、
//! 13 枚配置 + 残りを捨てる 1 回の着手で完了する。通常プレイヤーは従来通り。

use ofc_engine::Card;
use ofc_engine::game::{GameError, GameState, Placement, RowKind, Street};

/// FL の一括配置: 配られた札を先頭から bottom5/middle5/top3 に置き、残りを捨てる。
fn fl_move(dealt: &[Card]) -> (Vec<Placement>, Vec<Card>) {
    let rows = [
        (RowKind::Bottom, 0..5),
        (RowKind::Middle, 5..10),
        (RowKind::Top, 10..13),
    ];
    let placements = rows
        .into_iter()
        .flat_map(|(row, range)| {
            range.map(move |i| Placement {
                card: dealt[i],
                row,
            })
        })
        .collect();
    (placements, dealt[13..].to_vec())
}

fn initial_move(dealt: &[Card]) -> Vec<Placement> {
    [
        (RowKind::Bottom, 0),
        (RowKind::Bottom, 1),
        (RowKind::Middle, 2),
        (RowKind::Middle, 3),
        (RowKind::Top, 4),
    ]
    .into_iter()
    .map(|(row, i)| Placement {
        card: dealt[i],
        row,
    })
    .collect()
}

#[test]
fn flプレイヤーは指定枚数を一括配布される() {
    let state = GameState::new_with_fantasyland(2, 0, 1, &[14, 0]).unwrap();
    assert_eq!(state.current_player(), 0);
    assert_eq!(state.street(), Street::Fantasyland);
    assert_eq!(state.dealt_cards().len(), 14);
}

#[test]
fn flプレイヤーは13枚配置と残り捨てで完了する() {
    let mut state = GameState::new_with_fantasyland(2, 0, 1, &[15, 0]).unwrap();
    let dealt = state.dealt_cards().to_vec();
    assert_eq!(dealt.len(), 15);
    let (placements, discards) = fl_move(&dealt);
    assert_eq!(discards.len(), 2);
    state.apply_fantasyland(&placements, &discards).unwrap();

    assert!(state.board(0).is_complete());
    // 手番は通常プレイヤー(P1)の initial へ
    assert_eq!(state.current_player(), 1);
    assert_eq!(state.street(), Street::Initial);
    assert_eq!(state.dealt_cards().len(), 5);
}

#[test]
fn 通常プレイヤーの街ではflプレイヤーがスキップされる() {
    let mut state = GameState::new_with_fantasyland(2, 0, 1, &[14, 0]).unwrap();
    let dealt = state.dealt_cards().to_vec();
    let (placements, discards) = fl_move(&dealt);
    state.apply_fantasyland(&placements, &discards).unwrap();

    // P1 の initial
    let dealt = state.dealt_cards().to_vec();
    state.apply(&initial_move(&dealt), None).unwrap();

    // P0(FL 完了済み)はスキップされ、P1 の draw1 になる
    assert_eq!(state.current_player(), 1);
    assert_eq!(state.street(), Street::Draw(1));
}

#[test]
fn 両者flなら2手で完走する() {
    let mut state = GameState::new_with_fantasyland(2, 0, 1, &[14, 14]).unwrap();
    for _ in 0..2 {
        let dealt = state.dealt_cards().to_vec();
        let (placements, discards) = fl_move(&dealt);
        state.apply_fantasyland(&placements, &discards).unwrap();
    }
    assert_eq!(state.street(), Street::Finished);
    assert!(state.board(0).is_complete() && state.board(1).is_complete());
}

#[test]
fn flの着手は捨て札枚数まで厳密に検証される() {
    let mut state = GameState::new_with_fantasyland(2, 0, 1, &[14, 0]).unwrap();
    let dealt = state.dealt_cards().to_vec();
    let (placements, discards) = fl_move(&dealt);

    // 捨て札不足
    assert!(matches!(
        state.apply_fantasyland(&placements, &[]),
        Err(GameError::WrongDiscardCount { .. })
    ));
    // 配られていないカードの混入は既存の検証で拒否される
    let mut bad = discards.clone();
    bad[0] = if dealt.iter().any(|c| c.to_string() == "As") {
        "Ks".parse().unwrap()
    } else {
        "As".parse().unwrap()
    };
    assert!(state.apply_fantasyland(&placements, &bad).is_err());
    // 正しい着手は通る
    state.apply_fantasyland(&placements, &discards).unwrap();
}

#[test]
fn 通常手番にflの着手はできずその逆もできない() {
    let mut state = GameState::new_with_fantasyland(2, 0, 1, &[14, 0]).unwrap();
    // FL 手番に通常 apply は拒否
    assert!(matches!(
        state.apply(&[], None),
        Err(GameError::NotFantasylandTurn) | Err(GameError::WrongPlacementCount { .. })
    ));
    let dealt = state.dealt_cards().to_vec();
    let (placements, discards) = fl_move(&dealt);
    state.apply_fantasyland(&placements, &discards).unwrap();

    // 通常手番に FL apply は拒否
    let err = state.apply_fantasyland(&placements, &discards);
    assert!(matches!(err, Err(GameError::NotFantasylandTurn)));
}

#[test]
fn fl枚数の指定は検証される() {
    // プレイヤー数と配列長の不一致
    assert!(GameState::new_with_fantasyland(2, 0, 1, &[14]).is_err());
    // 範囲外の枚数
    assert!(GameState::new_with_fantasyland(2, 0, 1, &[12, 0]).is_err());
    assert!(GameState::new_with_fantasyland(2, 0, 1, &[18, 0]).is_err());
    // 0(通常)と 13〜17 は許容
    assert!(GameState::new_with_fantasyland(2, 0, 1, &[13, 0]).is_ok());
    assert!(GameState::new_with_fantasyland(2, 0, 1, &[17, 17]).is_ok());
}

#[test]
fn 従来のnew_gameはfl無しと等価() {
    let a = GameState::new(2, 0, 5).unwrap();
    let b = GameState::new_with_fantasyland(2, 0, 5, &[0, 0]).unwrap();
    assert_eq!(a, b);
}

#[test]
fn fl状態もserdeで往復できる() {
    let state = GameState::new_with_fantasyland(2, 1, 9, &[16, 0]).unwrap();
    let json = serde_json::to_string(&state).unwrap();
    let restored: GameState = serde_json::from_str(&json).unwrap();
    assert_eq!(state, restored);
    assert_eq!(restored.street(), Street::Fantasyland);
}

#[test]
fn ランダム手はflにも対応して完走できる() {
    use ofc_engine::game::random_move_full;
    for seed in 0..20 {
        let fl = match seed % 4 {
            0 => [14, 0],
            1 => [0, 15],
            2 => [17, 14],
            _ => [0, 0],
        };
        let mut state = GameState::new_with_fantasyland(2, (seed % 3) as u8, seed, &fl).unwrap();
        let mut move_seed = 500 + seed;
        while state.street() != Street::Finished {
            let (placements, discards) = random_move_full(&state, move_seed).unwrap();
            if state.street() == Street::Fantasyland {
                state.apply_fantasyland(&placements, &discards).unwrap();
            } else {
                state.apply(&placements, discards.first().copied()).unwrap();
            }
            move_seed += 1;
        }
        assert!(state.board(0).is_complete() && state.board(1).is_complete());
    }
}
