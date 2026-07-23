//! カードの型と wire 表記(ADR 0003)のパース・表示。

use std::fmt;
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rank {
    Two,
    Three,
    Four,
    Five,
    Six,
    Seven,
    Eight,
    Nine,
    Ten,
    Jack,
    Queen,
    King,
    Ace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Suit {
    Spades,
    Hearts,
    Diamonds,
    Clubs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Card {
    Standard { rank: Rank, suit: Suit },
    Joker,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseCardError {
    pub input: String,
}

impl Rank {
    fn from_wire(c: char) -> Option<Self> {
        Some(match c {
            '2' => Rank::Two,
            '3' => Rank::Three,
            '4' => Rank::Four,
            '5' => Rank::Five,
            '6' => Rank::Six,
            '7' => Rank::Seven,
            '8' => Rank::Eight,
            '9' => Rank::Nine,
            'T' => Rank::Ten,
            'J' => Rank::Jack,
            'Q' => Rank::Queen,
            'K' => Rank::King,
            'A' => Rank::Ace,
            _ => return None,
        })
    }

    fn to_wire(self) -> char {
        match self {
            Rank::Two => '2',
            Rank::Three => '3',
            Rank::Four => '4',
            Rank::Five => '5',
            Rank::Six => '6',
            Rank::Seven => '7',
            Rank::Eight => '8',
            Rank::Nine => '9',
            Rank::Ten => 'T',
            Rank::Jack => 'J',
            Rank::Queen => 'Q',
            Rank::King => 'K',
            Rank::Ace => 'A',
        }
    }
}

impl Suit {
    fn from_wire(c: char) -> Option<Self> {
        Some(match c {
            's' => Suit::Spades,
            'h' => Suit::Hearts,
            'd' => Suit::Diamonds,
            'c' => Suit::Clubs,
            _ => return None,
        })
    }

    fn to_wire(self) -> char {
        match self {
            Suit::Spades => 's',
            Suit::Hearts => 'h',
            Suit::Diamonds => 'd',
            Suit::Clubs => 'c',
        }
    }
}

impl FromStr for Card {
    type Err = ParseCardError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let err = || ParseCardError {
            input: s.to_string(),
        };
        let mut chars = s.chars();
        let (first, second) = match (chars.next(), chars.next(), chars.next()) {
            (Some(a), Some(b), None) => (a, b),
            _ => return Err(err()),
        };
        if (first, second) == ('X', 'j') {
            return Ok(Card::Joker);
        }
        let rank = Rank::from_wire(first).ok_or_else(err)?;
        let suit = Suit::from_wire(second).ok_or_else(err)?;
        Ok(Card::Standard { rank, suit })
    }
}

impl fmt::Display for Card {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Card::Standard { rank, suit } => write!(f, "{}{}", rank.to_wire(), suit.to_wire()),
            Card::Joker => write!(f, "Xj"),
        }
    }
}
