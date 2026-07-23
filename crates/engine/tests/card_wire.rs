//! Card の wire 表記(ADR 0003)のテスト。
//! 表記仕様: ランク大文字 1 字(A,K,Q,J,T,9..2) + スート小文字 1 字(s,h,d,c)。Joker は "Xj"。

use ofc_engine::{Card, Rank, Suit};

#[test]
fn 標準カードの文字列をパースできる() {
    // Arrange
    let wire = "As";

    // Act
    let card: Card = wire.parse().unwrap();

    // Assert
    assert_eq!(
        card,
        Card::Standard {
            rank: Rank::Ace,
            suit: Suit::Spades
        }
    );
}

#[test]
fn ランク10はtで表記される() {
    let card: Card = "Td".parse().unwrap();
    assert_eq!(
        card,
        Card::Standard {
            rank: Rank::Ten,
            suit: Suit::Diamonds
        }
    );
}

#[test]
fn 数字ランクとスート全種をパースできる() {
    let card: Card = "2c".parse().unwrap();
    assert_eq!(
        card,
        Card::Standard {
            rank: Rank::Two,
            suit: Suit::Clubs
        }
    );
    let card: Card = "9h".parse().unwrap();
    assert_eq!(
        card,
        Card::Standard {
            rank: Rank::Nine,
            suit: Suit::Hearts
        }
    );
}

#[test]
fn ジョーカーはxjで表記される() {
    let card: Card = "Xj".parse().unwrap();
    assert_eq!(card, Card::Joker);
}

#[test]
fn パースと表示は往復して一致する() {
    for wire in ["As", "Td", "2c", "9h", "Kd", "Xj"] {
        let card: Card = wire.parse().unwrap();
        assert_eq!(card.to_string(), wire);
    }
}

#[test]
fn 不正な文字列はエラーになる() {
    // ランクが小文字 / スートが大文字 / 未知の文字 / 長さ違反はすべて拒否する
    for invalid in ["as", "AS", "1s", "Ax", "A", "Asd", "", "xj", "XJ"] {
        assert!(
            invalid.parse::<Card>().is_err(),
            "{invalid:?} がパースできてしまった"
        );
    }
}

#[test]
fn ランクは強さ順に比較できる() {
    assert!(Rank::Ace > Rank::King);
    assert!(Rank::King > Rank::Queen);
    assert!(Rank::Three > Rank::Two);
    assert!(Rank::Ten > Rank::Nine);
}
