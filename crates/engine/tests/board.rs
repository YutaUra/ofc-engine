//! Board(3/5/5 の盤面)の構築と不変条件のテスト。
//! 段構成 3/5/5 は ADR 0003 で固定した不変条件。

use ofc_engine::{Board, BoardError, Card};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

#[test]
fn 完成した盤面を構築できる() {
    // Arrange
    let top = cards(&["Qs", "Qh", "3d"]);
    let middle = cards(&["8c", "8d", "8h", "Kc", "2s"]);
    let bottom = cards(&["Ah", "Kh", "Qd", "Jh", "Th"]);

    // Act
    let board = Board::new(top, middle, bottom).unwrap();

    // Assert
    assert!(board.is_complete());
}

#[test]
fn 配置途中の盤面も構築できる() {
    let board = Board::new(cards(&["Qs"]), cards(&[]), cards(&["Ah", "Kh"])).unwrap();
    assert!(!board.is_complete());
}

#[test]
fn 空の盤面は完成ではない() {
    let board = Board::new(vec![], vec![], vec![]).unwrap();
    assert!(!board.is_complete());
}

#[test]
fn topが4枚以上ならエラーになる() {
    let result = Board::new(cards(&["Qs", "Qh", "3d", "4d"]), vec![], vec![]);
    assert_eq!(
        result.unwrap_err(),
        BoardError::RowOverflow {
            row: "top".to_string(),
            max: 3,
            actual: 4
        }
    );
}

#[test]
fn middleとbottomは6枚以上ならエラーになる() {
    let six = cards(&["2c", "3c", "4c", "5c", "6c", "7c"]);
    assert!(matches!(
        Board::new(vec![], six.clone(), vec![]),
        Err(BoardError::RowOverflow {
            max: 5,
            actual: 6,
            ..
        })
    ));
    assert!(matches!(
        Board::new(vec![], vec![], six),
        Err(BoardError::RowOverflow {
            max: 5,
            actual: 6,
            ..
        })
    ));
}

#[test]
fn 同一カードが盤面に2枚あるとエラーになる() {
    // 同じ列内の重複
    let result = Board::new(cards(&["Qs", "Qs", "3d"]), vec![], vec![]);
    assert_eq!(
        result.unwrap_err(),
        BoardError::DuplicateCard {
            card: "Qs".to_string()
        }
    );

    // 列をまたいだ重複
    let result = Board::new(cards(&["Qs"]), cards(&["Qs"]), vec![]);
    assert_eq!(
        result.unwrap_err(),
        BoardError::DuplicateCard {
            card: "Qs".to_string()
        }
    );
}

#[test]
fn ジョーカーは複数枚あってもエラーにならない() {
    // Joker の許容枚数はデッキ設定(RuleSet)の責務であり、Board は構造としては拒否しない。
    // 2 枚 Joker 入りのローカルルールで "Xj" が 2 回現れるのは正常な盤面。
    let board = Board::new(cards(&["Xj", "Xj"]), vec![], vec![]).unwrap();
    assert!(!board.is_complete());
}

#[test]
fn 各列のカードを参照できる() {
    let board = Board::new(cards(&["Qs"]), cards(&["8c", "8d"]), cards(&["Ah"])).unwrap();
    assert_eq!(board.top(), cards(&["Qs"]).as_slice());
    assert_eq!(board.middle(), cards(&["8c", "8d"]).as_slice());
    assert_eq!(board.bottom(), cards(&["Ah"]).as_slice());
}
