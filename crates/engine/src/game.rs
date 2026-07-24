//! ゲーム進行(Pineapple)の状態機械。
//! 初手 5 枚配置 → 3 引き 2 置き 1 捨て × 4 街で 13 枚を完成させる。
//! デッキは seed から決定的に生成する(charter の決定性要件。エンジンは
//! OS の乱数に依存せず、同じ seed なら常に同じ進行を再現できる)。

use crate::board::BoardError;
use crate::{Board, Card, Rank, Suit};

// wire では "top"/"middle"/"bottom"(JSON 文字列 API から着手を受けるため)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RowKind {
    Top,
    Middle,
    Bottom,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Placement {
    pub card: Card,
    pub row: RowKind,
}

// GameState ごと保存できるよう serde 対応(アプリの中断復帰用途)。
// deck を含むため、シリアライズ結果には未公開のカード順が入ることに注意
// (対戦相手に渡してよいデータではない。保存はローカル/サーバ秘匿領域で)。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Street {
    Initial,
    /// 1..=4 の引き番。
    Draw(u8),
    Finished,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GameError {
    /// デッキ枚数(52 + Joker)が人数 ×(13 配置 + 4 捨て札)に足りない。
    DeckTooSmall {
        players: u8,
        deck: usize,
    },
    TooFewPlayers {
        actual: u8,
    },
    GameFinished,
    UnexpectedDiscard,
    MissingDiscard,
    CardNotDealt {
        card: String,
    },
    WrongPlacementCount {
        expected: usize,
        actual: usize,
    },
    Board(BoardError),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct GameState {
    boards: Vec<Board>,
    deck: Vec<Card>,
    deck_pos: usize,
    current_player: usize,
    street: Street,
    dealt: Vec<Card>,
}

const CARDS_PER_PLAYER: usize = 13 + 4; // 13 枚配置 + 捨て札 4 枚

impl GameState {
    pub fn new(players: u8, jokers: u8, seed: u64) -> Result<Self, GameError> {
        if players < 2 {
            return Err(GameError::TooFewPlayers { actual: players });
        }
        let mut deck = full_deck(jokers);
        if (players as usize) * CARDS_PER_PLAYER > deck.len() {
            return Err(GameError::DeckTooSmall {
                players,
                deck: deck.len(),
            });
        }
        shuffle(&mut deck, seed);

        let mut state = Self {
            boards: vec![
                Board::new(vec![], vec![], vec![]).expect("空盤面は常に有効");
                players as usize
            ],
            deck,
            deck_pos: 0,
            current_player: 0,
            street: Street::Initial,
            dealt: vec![],
        };
        state.deal();
        Ok(state)
    }

    pub fn current_player(&self) -> usize {
        self.current_player
    }

    pub fn street(&self) -> Street {
        self.street
    }

    /// 現在の手番プレイヤーに配られている未配置のカード。
    pub fn dealt_cards(&self) -> &[Card] {
        &self.dealt
    }

    pub fn board(&self, player: usize) -> &Board {
        &self.boards[player]
    }

    /// 全プレイヤーの盤面(手番順)。
    pub fn boards(&self) -> &[Board] {
        &self.boards
    }

    /// 手番プレイヤーの配置(+捨て札)を適用し、次の手番に進める。
    pub fn apply(
        &mut self,
        placements: &[Placement],
        discard: Option<Card>,
    ) -> Result<(), GameError> {
        let expected = match self.street {
            Street::Finished => return Err(GameError::GameFinished),
            Street::Initial => {
                if discard.is_some() {
                    return Err(GameError::UnexpectedDiscard);
                }
                5
            }
            Street::Draw(_) => {
                if discard.is_none() {
                    return Err(GameError::MissingDiscard);
                }
                2
            }
        };
        if placements.len() != expected {
            return Err(GameError::WrongPlacementCount {
                expected,
                actual: placements.len(),
            });
        }

        // 配置 + 捨て札が「配られたカードちょうど」であることを多重集合で検証する
        // (Joker "Xj" は同一表記で複数枚ありうるため、単純な集合では足りない)。
        let mut remaining = self.dealt.clone();
        let used = placements
            .iter()
            .map(|p| p.card)
            .chain(discard)
            .collect::<Vec<_>>();
        for card in &used {
            let pos = remaining.iter().position(|c| c == card).ok_or_else(|| {
                GameError::CardNotDealt {
                    card: card.to_string(),
                }
            })?;
            remaining.swap_remove(pos);
        }

        // 盤面へ反映。行あふれは Board::new の検証で拒否される
        let board = &self.boards[self.current_player];
        let mut rows = [
            board.top().to_vec(),
            board.middle().to_vec(),
            board.bottom().to_vec(),
        ];
        for p in placements {
            let row = match p.row {
                RowKind::Top => &mut rows[0],
                RowKind::Middle => &mut rows[1],
                RowKind::Bottom => &mut rows[2],
            };
            row.push(p.card);
        }
        let [top, middle, bottom] = rows;
        self.boards[self.current_player] =
            Board::new(top, middle, bottom).map_err(GameError::Board)?;

        self.advance();
        Ok(())
    }

    fn advance(&mut self) {
        self.current_player += 1;
        if self.current_player == self.boards.len() {
            self.current_player = 0;
            self.street = match self.street {
                Street::Initial => Street::Draw(1),
                Street::Draw(4) => Street::Finished,
                Street::Draw(n) => Street::Draw(n + 1),
                Street::Finished => Street::Finished,
            };
        }
        self.deal();
    }

    fn deal(&mut self) {
        let count = match self.street {
            Street::Initial => 5,
            Street::Draw(_) => 3,
            Street::Finished => {
                self.dealt = vec![];
                return;
            }
        };
        self.dealt = self.deck[self.deck_pos..self.deck_pos + count].to_vec();
        self.deck_pos += count;
    }
}

/// 現在の手番に対するランダムな合法手を返す(完走後は None)。
/// CPU の土台・デバッグ・シミュレーション用。seed 固定で決定的。
pub fn random_move(state: &GameState, seed: u64) -> Option<(Vec<Placement>, Option<Card>)> {
    let mut rng = Xorshift::new(seed);
    let mut dealt = state.dealt_cards().to_vec();

    let board = state.board(state.current_player());
    let mut capacity = [
        (RowKind::Top, 3 - board.top().len()),
        (RowKind::Middle, 5 - board.middle().len()),
        (RowKind::Bottom, 5 - board.bottom().len()),
    ];

    let discard = match state.street() {
        Street::Finished => return None,
        Street::Initial => None,
        Street::Draw(_) => {
            let i = (rng.next() % dealt.len() as u64) as usize;
            Some(dealt.swap_remove(i))
        }
    };

    let placements = dealt
        .into_iter()
        .map(|card| {
            // 空きのある行から一様に選ぶ(空きスロット数で重み付け)
            let open: usize = capacity.iter().map(|(_, n)| n).sum();
            let mut pick = (rng.next() % open as u64) as usize;
            let slot = capacity
                .iter_mut()
                .find(|(_, n)| {
                    if pick < *n {
                        true
                    } else {
                        pick -= *n;
                        false
                    }
                })
                .expect("空きスロット数の合計から選んでいるため必ず見つかる");
            slot.1 -= 1;
            Placement { card, row: slot.0 }
        })
        .collect();

    Some((placements, discard))
}

/// 決定的な乱数(shuffle と同系)。seed=0 でも縮退しないよう定数を混ぜる。
struct Xorshift(u64);

impl Xorshift {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_add(0x9E37_79B9_7F4A_7C15))
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
}

fn full_deck(jokers: u8) -> Vec<Card> {
    let ranks = [
        Rank::Two,
        Rank::Three,
        Rank::Four,
        Rank::Five,
        Rank::Six,
        Rank::Seven,
        Rank::Eight,
        Rank::Nine,
        Rank::Ten,
        Rank::Jack,
        Rank::Queen,
        Rank::King,
        Rank::Ace,
    ];
    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    let mut deck: Vec<Card> = ranks
        .into_iter()
        .flat_map(|rank| suits.map(|suit| Card::Standard { rank, suit }))
        .collect();
    deck.extend(std::iter::repeat_n(Card::Joker, jokers as usize));
    deck
}

/// xorshift64 による Fisher-Yates シャッフル。
/// 外部乱数クレートを使わない理由: エンジンの決定性(同 seed 同結果)を
/// 依存クレートのアルゴリズム変更から切り離して保証するため。
fn shuffle(deck: &mut [Card], seed: u64) {
    let mut state = seed.wrapping_add(0x9E37_79B9_7F4A_7C15); // seed=0 でも縮退しない
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for i in (1..deck.len()).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        deck.swap(i, j);
    }
}
