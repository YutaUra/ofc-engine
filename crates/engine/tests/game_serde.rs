//! GameState の serialize/deserialize(中断復帰用途)のテスト。
//! charter の決定性要件: 復元後の進行は元の状態と完全一致する。

use ofc_engine::game::{GameState, Placement, RowKind};

fn advance_one(state: &mut GameState) {
    let dealt = state.dealt_cards().to_vec();
    let placements = vec![
        Placement {
            card: dealt[0],
            row: RowKind::Bottom,
        },
        Placement {
            card: dealt[1],
            row: RowKind::Bottom,
        },
        Placement {
            card: dealt[2],
            row: RowKind::Middle,
        },
        Placement {
            card: dealt[3],
            row: RowKind::Middle,
        },
        Placement {
            card: dealt[4],
            row: RowKind::Top,
        },
    ];
    state.apply(&placements, None).unwrap();
}

#[test]
fn ゲーム状態はjsonで往復できる() {
    let mut state = GameState::new(2, 1, 99).unwrap();
    advance_one(&mut state);

    let json = serde_json::to_string(&state).unwrap();
    let restored: GameState = serde_json::from_str(&json).unwrap();

    assert_eq!(state, restored);
}

#[test]
fn 復元後の進行は元の状態と完全一致する() {
    let mut original = GameState::new(2, 0, 7).unwrap();
    advance_one(&mut original);

    let json = serde_json::to_string(&original).unwrap();
    let mut restored: GameState = serde_json::from_str(&json).unwrap();

    // 同じ操作を両方に適用し続けても一致し続ける(配牌も含めて決定的)
    advance_one(&mut original);
    advance_one(&mut restored);
    assert_eq!(original, restored);
    assert_eq!(original.dealt_cards(), restored.dealt_cards());
}
