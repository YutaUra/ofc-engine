//! 3/5/5 の盤面。段構成は ADR 0003 で固定した不変条件。

use crate::Card;

pub const TOP_SIZE: usize = 3;
pub const MIDDLE_SIZE: usize = 5;
pub const BOTTOM_SIZE: usize = 5;

// wire では {top, middle, bottom} の rows 形式(ADR 0003)。
// デシリアライズ時も Board::new の不変条件検証を通す。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "BoardWire", into = "BoardWire")]
pub struct Board {
    top: Vec<Card>,
    middle: Vec<Card>,
    bottom: Vec<Card>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct BoardWire {
    top: Vec<Card>,
    middle: Vec<Card>,
    bottom: Vec<Card>,
}

impl TryFrom<BoardWire> for Board {
    type Error = String;

    fn try_from(wire: BoardWire) -> Result<Self, Self::Error> {
        Board::new(wire.top, wire.middle, wire.bottom).map_err(|e| format!("{e:?}"))
    }
}

impl From<Board> for BoardWire {
    fn from(board: Board) -> Self {
        Self {
            top: board.top,
            middle: board.middle,
            bottom: board.bottom,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BoardError {
    RowOverflow {
        row: String,
        max: usize,
        actual: usize,
    },
    DuplicateCard {
        card: String,
    },
}

impl Board {
    pub fn new(top: Vec<Card>, middle: Vec<Card>, bottom: Vec<Card>) -> Result<Self, BoardError> {
        for (row, cards, max) in [
            ("top", &top, TOP_SIZE),
            ("middle", &middle, MIDDLE_SIZE),
            ("bottom", &bottom, BOTTOM_SIZE),
        ] {
            if cards.len() > max {
                return Err(BoardError::RowOverflow {
                    row: row.to_string(),
                    max,
                    actual: cards.len(),
                });
            }
        }

        // 重複検出は標準カードのみ。Joker("Xj")の許容枚数はデッキ設定
        // (RuleSet)の責務であり、盤面構造としては複数枚を許す。
        let mut seen = std::collections::HashSet::new();
        for card in top.iter().chain(&middle).chain(&bottom) {
            if matches!(card, Card::Standard { .. }) && !seen.insert(*card) {
                return Err(BoardError::DuplicateCard {
                    card: card.to_string(),
                });
            }
        }

        Ok(Self {
            top,
            middle,
            bottom,
        })
    }

    pub fn is_complete(&self) -> bool {
        self.top.len() == TOP_SIZE
            && self.middle.len() == MIDDLE_SIZE
            && self.bottom.len() == BOTTOM_SIZE
    }

    pub fn top(&self) -> &[Card] {
        &self.top
    }

    pub fn middle(&self) -> &[Card] {
        &self.middle
    }

    pub fn bottom(&self) -> &[Card] {
        &self.bottom
    }
}
