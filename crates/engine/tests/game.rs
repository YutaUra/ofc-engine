//! ゲーム進行(Pineapple)のテスト。
//! 初手 5 枚配置 → 3 引き 2 置き 1 捨て × 4 街 → 13 枚完成、を状態機械で管理する。
//! デッキは seed から決定的に生成される(charter の決定性要件)。

use ofc_engine::Card;
use ofc_engine::game::{GameError, GameState, Placement, RowKind, Street};

fn new_game() -> GameState {
    GameState::new(2, 0, 42).unwrap()
}

/// 配られたカードを機械的に順番配置するヘルパ(初手 5 枚)。
fn auto_initial(state: &mut GameState) {
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

/// 3 引き 2 置き 1 捨てを機械的に行うヘルパ。
fn auto_street(state: &mut GameState) {
    let dealt = state.dealt_cards().to_vec();
    let board = state.board(state.current_player());
    // 空きのある行に順に置く
    let mut placements = Vec::new();
    let mut capacity = [
        (RowKind::Bottom, 5 - board.bottom().len()),
        (RowKind::Middle, 5 - board.middle().len()),
        (RowKind::Top, 3 - board.top().len()),
    ];
    for card in &dealt[..2] {
        for slot in capacity.iter_mut() {
            if slot.1 > 0 {
                placements.push(Placement {
                    card: *card,
                    row: slot.0,
                });
                slot.1 -= 1;
                break;
            }
        }
    }
    state.apply(&placements, Some(dealt[2])).unwrap();
}

#[test]
fn シード固定でデッキは決定的に再現される() {
    let a = GameState::new(2, 0, 123).unwrap();
    let b = GameState::new(2, 0, 123).unwrap();
    assert_eq!(a.dealt_cards(), b.dealt_cards());

    let c = GameState::new(2, 0, 124).unwrap();
    assert_ne!(a.dealt_cards(), c.dealt_cards()); // 別シードは別配牌(ほぼ確実)
}

#[test]
fn 初手は5枚配られ全て配置する() {
    let mut state = new_game();
    assert_eq!(state.street(), Street::Initial);
    assert_eq!(state.current_player(), 0);
    assert_eq!(state.dealt_cards().len(), 5);

    auto_initial(&mut state);

    // 次のプレイヤーの初手へ
    assert_eq!(state.current_player(), 1);
    assert_eq!(state.street(), Street::Initial);
}

#[test]
fn 初手で捨て札は許されない() {
    let mut state = new_game();
    let dealt = state.dealt_cards().to_vec();
    let placements: Vec<Placement> = dealt[..4]
        .iter()
        .map(|c| Placement {
            card: *c,
            row: RowKind::Bottom,
        })
        .collect();
    let result = state.apply(&placements, Some(dealt[4]));
    assert!(matches!(result, Err(GameError::UnexpectedDiscard)));
}

#[test]
fn 街では3枚配られ2枚置いて1枚捨てる() {
    let mut state = new_game();
    auto_initial(&mut state);
    auto_initial(&mut state);

    assert_eq!(state.street(), Street::Draw(1));
    assert_eq!(state.dealt_cards().len(), 3);

    auto_street(&mut state);
    assert_eq!(state.current_player(), 1);
}

#[test]
fn 配られていないカードは配置できない() {
    let mut state = new_game();
    let outside: Card = if state.dealt_cards().iter().any(|c| c.to_string() == "As") {
        "Ks".parse().unwrap()
    } else {
        "As".parse().unwrap()
    };
    // 配られたカードに含まれない可能性のあるカードを 1 枚混ぜる
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
            card: outside,
            row: RowKind::Top,
        },
    ];
    let result = state.apply(&placements, None);
    assert!(matches!(result, Err(GameError::CardNotDealt { .. })));
}

#[test]
fn 枚数が合わない配置は拒否される() {
    let mut state = new_game();
    let dealt = state.dealt_cards().to_vec();
    // 初手なのに 4 枚しか置かない
    let placements: Vec<Placement> = dealt[..4]
        .iter()
        .map(|c| Placement {
            card: *c,
            row: RowKind::Bottom,
        })
        .collect();
    assert!(matches!(
        state.apply(&placements, None),
        Err(GameError::WrongPlacementCount { .. })
    ));
}

#[test]
fn ゲームは4街で完走し全盤面が完成する() {
    let mut state = new_game();
    auto_initial(&mut state);
    auto_initial(&mut state);
    for _ in 0..4 {
        auto_street(&mut state); // player 0
        auto_street(&mut state); // player 1
    }
    assert_eq!(state.street(), Street::Finished);
    assert!(state.board(0).is_complete());
    assert!(state.board(1).is_complete());
}

#[test]
fn 完走後はカードが配られず操作もできない() {
    let mut state = new_game();
    auto_initial(&mut state);
    auto_initial(&mut state);
    for _ in 0..8 {
        auto_street(&mut state);
    }
    assert!(state.dealt_cards().is_empty());
    assert!(matches!(
        state.apply(&[], None),
        Err(GameError::GameFinished)
    ));
}

#[test]
fn 全プレイヤーに配られたカードは重複しない() {
    let mut state = new_game();
    let mut seen: Vec<Card> = Vec::new();
    seen.extend(state.dealt_cards());
    auto_initial(&mut state);
    seen.extend(state.dealt_cards());
    auto_initial(&mut state);
    for _ in 0..8 {
        seen.extend(state.dealt_cards());
        auto_street(&mut state);
    }
    let unique: std::collections::HashSet<String> = seen.iter().map(|c| c.to_string()).collect();
    assert_eq!(unique.len(), seen.len());
}

#[test]
fn ジョーカー入りデッキも作れる() {
    let state = GameState::new(2, 2, 7).unwrap();
    // デッキ 54 枚(52 + Joker 2)から配られる。Joker が引かれるかは配牌次第
    // なので、デッキ生成が拒否されないことのみ確認する
    assert_eq!(state.dealt_cards().len(), 5);
}

#[test]
fn 人数が多すぎてデッキが足りない場合は拒否される() {
    // 1 人あたり 13 枚 + 捨て札 4 枚 = 17 枚。52 枚デッキで 4 人は 68 枚必要で不足
    assert!(GameState::new(4, 0, 1).is_err());
}
