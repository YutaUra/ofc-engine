//! 軽量ヒューリスティック CPU(着手選択)。
//!
//! 方策は chinese-poker-solver の rollout 内貪欲方策(normal.rs の
//! greedy_score / fl_setup_bonus)を参考にした単発貪欲 + 最終街のみ厳密全列挙:
//!
//! - ファウル判定は「完成した段同士」だけで行う(未完成段まで順序を強制すると
//!   下段の強役ドローを序盤で捨ててしまい大幅に弱くなる、という solver の知見)
//! - 完成段のロイヤリティ合計を支配項(×2000)にする
//! - タイブレークは行バランス「高札は下・低札は上」(2Σbottom + Σmiddle − Σtop)
//! - FL 意識モードでは上段のペア形成にボーナスを与える(素朴なバランス項は
//!   FL 突入価値を系統的に殺すため、その相殺。係数は solver の A/B 調整値に準拠)
//! - 最終街(Draw(4))は候補が少ないため evaluate_board による厳密評価で選ぶ
//!   (ここを貪欲のままにすると目に見えて弱い、という solver の知見)

use ofc_engine::evaluate::evaluate_board;
use ofc_engine::fantasyland::FantasylandRules;
use ofc_engine::game::{Placement, RowKind, Street};
use ofc_engine::hand::{Category, HandRank, evaluate_five, evaluate_three};
use ofc_engine::joker::{RowArity, resolve_row};
use ofc_engine::royalty::{Row, RoyaltyTable, royalty_points};
use ofc_engine::{Board, Card, Rank, Suit};

#[derive(Debug, Clone, PartialEq)]
pub struct CpuConfig {
    pub royalty: RoyaltyTable,
    pub fantasyland: FantasylandRules,
    /// 上段のペア形成(FL 突入)を優先するか。
    pub fl_aware: bool,
    /// FL 突入の差分価値 φ̂ のグリッド(strength 0.0..=1.0 を 0.1 刻みの
    /// 11 点 × 配布枚数 14/15/16/17 の 4 値、ロイヤリティ点単位)。
    /// φ̂ は「その強さの自分がその後 FL をどうプレイするか」に依存するため
    /// strength 別に持ち、実際の値は strength の位置で線形補間して使う。
    /// 既定値は examples/fl_phi_measure.rs の自己対戦実測
    /// (継続=枚数維持ルール・平場基準補正済み)。17 枚はサンプル希少のため
    /// 概算の外挿値。
    pub fl_phi_table: [[f64; 4]; 11],

    /// strength 1.0(rollout)の探索予算。
    pub rollout: RolloutBudget,
    /// 強さ(0.0..=1.0)。各手番で貪欲方策を選ぶ確率(ε 混合)。
    /// 0.0 = 常にランダム合法手、1.0 = 常に貪欲手。範囲外は丸める。
    /// 1.0 未満で使うときは choose_move_seeded に手番ごとに異なる seed を
    /// 渡すこと(同じ seed は同じ判定を再現する = 決定性の担保)。
    pub strength: f64,
}

impl CpuConfig {
    /// 配布枚数(14..=17)に対応する φ。strength のグリッド区間で線形補間する。
    pub fn fl_value_for_cards(&self, cards: u8) -> f64 {
        let level = (cards.clamp(14, 17) - 14) as usize;
        let s = self.strength.clamp(0.0, 1.0) * 10.0;
        let lower = (s.floor() as usize).min(10);
        let upper = (lower + 1).min(10);
        let frac = s - lower as f64;
        self.fl_phi_table[lower][level] * (1.0 - frac) + self.fl_phi_table[upper][level] * frac
    }

    pub fn standard() -> Self {
        Self {
            royalty: RoyaltyTable::standard_american(),
            fantasyland: FantasylandRules::standard_progressive(),
            fl_aware: true,
            rollout: RolloutBudget::default(),
            // examples/fl_phi_measure.rs の実測(4,000 セッション/点)。
            // 17 枚列はサンプル希少のため「AA(16 枚)以上」を下限に平滑化。
            // s=0.9→1.0 で跳ねるのは、帯選択の「最適よりわずかに下」の配置が
            // 継続条件(不連続)を落としやすく、弱いプレイでは FL を換金し
            // きれないため(strength 別に φ を持つ理由そのもの)。
            fl_phi_table: [
                [13.2, 16.7, 16.1, 22.0],
                [13.2, 15.8, 20.4, 20.4],
                [14.8, 17.6, 20.2, 20.2],
                [15.6, 17.9, 22.2, 22.7],
                [15.8, 18.0, 19.5, 32.4],
                [16.0, 19.9, 21.6, 38.2],
                [17.6, 19.3, 20.9, 20.9],
                [16.8, 19.9, 22.9, 31.3],
                [18.8, 20.9, 22.4, 23.1],
                [18.0, 21.2, 23.2, 33.2],
                // s=1.0 は rollout 方策の自己対戦で実測(他は ε 混合貪欲)
                [22.1, 35.5, 64.3, 97.6],
            ],
            strength: 1.0,
        }
    }
}

const ROW_KINDS: [RowKind; 3] = [RowKind::Top, RowKind::Middle, RowKind::Bottom];
const ROW_LIMITS: [usize; 3] = [3, 5, 5];

/// 現在の手番の着手を選ぶ(完走後は None)。
/// 同じ入力には常に同じ手を返す(候補の列挙順を固定し、真に上回る候補
/// だけで最良を更新するため)。
/// 1 手分の着手(配置列と捨て札)。
pub type Move = (Vec<Placement>, Option<Card>);

/// strength(ε 混合)を考慮した着手選択。強さ 1.0 なら choose_move と同一。
/// ランダム分岐も seed から決定的に引くため、同じ入力 + 同じ seed は
/// 常に同じ手を返す。
pub fn choose_move_seeded(
    board: &Board,
    dealt: &[Card],
    street: Street,
    config: &CpuConfig,
    seed: u64,
) -> Option<Move> {
    let strength = config.strength.clamp(0.0, 1.0);
    let mut rng = Xorshift::new(seed);
    // 10000 分率で比較する(f64 の乱数を作らずに済ませるため)
    let greedy = (rng.next() % 10_000) < (strength * 10_000.0) as u64;
    if greedy {
        choose_move(board, dealt, street, config)
    } else {
        random_legal_move(board, dealt, street, &mut rng)
    }
}

/// ランダムな合法手(engine の random_move と同じ方針だが、GameState では
/// なく盤面 + 配札 + 街から直接作る。ε 混合の乱数列を共有するため)。
fn random_legal_move(
    board: &Board,
    dealt: &[Card],
    street: Street,
    rng: &mut Xorshift,
) -> Option<Move> {
    let mut dealt = dealt.to_vec();
    let mut capacity = [
        (RowKind::Top, 3 - board.top().len()),
        (RowKind::Middle, 5 - board.middle().len()),
        (RowKind::Bottom, 5 - board.bottom().len()),
    ];
    let discard = match street {
        Street::Finished | Street::Fantasyland => return None,
        Street::Initial => None,
        Street::Draw(_) => {
            let i = (rng.next() % dealt.len() as u64) as usize;
            Some(dealt.swap_remove(i))
        }
    };
    let placements = dealt
        .into_iter()
        .map(|card| {
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
                .expect("空きスロット合計から選ぶため必ず見つかる");
            slot.1 -= 1;
            Placement { card, row: slot.0 }
        })
        .collect();
    Some((placements, discard))
}

/// 決定的な乱数(engine の shuffle と同系)。
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

pub fn choose_move(
    board: &Board,
    dealt: &[Card],
    street: Street,
    config: &CpuConfig,
) -> Option<Move> {
    let exact = street == Street::Draw(4);
    let mut best: Option<(MoveScore, Move)> = None;

    for candidate in enumerate_moves(board, dealt, street)? {
        let score = score_move(board, &candidate, exact, config);
        if best.as_ref().is_none_or(|(bs, _)| score > *bs) {
            best = Some((score, candidate));
        }
    }
    best.map(|(_, mv)| mv)
}

/// 貪欲スコア(i64)か、最終街の厳密キー。同じ街の候補同士でしか比較しない。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
enum MoveScore {
    Greedy(i64),
    Exact(bool, u32, [HandRank; 3]),
}

fn enumerate_moves(board: &Board, dealt: &[Card], street: Street) -> Option<Vec<Move>> {
    let capacity = [
        3 - board.top().len(),
        5 - board.middle().len(),
        5 - board.bottom().len(),
    ];
    match street {
        // FL の一括配置(13 枚 + 複数捨て札)は探索空間が別次元のため
        // v1 では未対応(None)。必要になったら専用の方策を実装する
        Street::Finished | Street::Fantasyland => None,
        Street::Initial => Some(
            assignments(dealt, &capacity)
                .into_iter()
                .map(|p| (p, None))
                .collect(),
        ),
        Street::Draw(_) => {
            let mut moves = Vec::new();
            for discard_idx in 0..dealt.len() {
                let mut remaining = dealt.to_vec();
                let discard = remaining.remove(discard_idx);
                for p in assignments(&remaining, &capacity) {
                    moves.push((p, Some(discard)));
                }
            }
            Some(moves)
        }
    }
}

/// カード列を空き容量の範囲で 3 行へ配る全割り当て(3^n を容量で枝刈り)。
fn assignments(cards: &[Card], capacity: &[usize; 3]) -> Vec<Vec<Placement>> {
    let mut result = Vec::new();
    let mut current = Vec::with_capacity(cards.len());
    fn rec(
        cards: &[Card],
        capacity: &mut [usize; 3],
        current: &mut Vec<Placement>,
        result: &mut Vec<Vec<Placement>>,
    ) {
        let Some((card, rest)) = cards.split_first() else {
            result.push(current.clone());
            return;
        };
        for row_idx in 0..3 {
            if capacity[row_idx] > 0 {
                capacity[row_idx] -= 1;
                current.push(Placement {
                    card: *card,
                    row: ROW_KINDS[row_idx],
                });
                rec(rest, capacity, current, result);
                current.pop();
                capacity[row_idx] += 1;
            }
        }
    }
    let mut capacity = *capacity;
    rec(cards, &mut capacity, &mut current, &mut result);
    result
}

fn score_move(board: &Board, (placements, _): &Move, exact: bool, config: &CpuConfig) -> MoveScore {
    let mut rows = [
        board.top().to_vec(),
        board.middle().to_vec(),
        board.bottom().to_vec(),
    ];
    for p in placements {
        let idx = ROW_KINDS
            .iter()
            .position(|r| *r == p.row)
            .expect("3 行のいずれか");
        rows[idx].push(p.card);
    }

    if exact {
        let [top, middle, bottom] = rows;
        let complete = Board::new(top, middle, bottom).expect("候補手は常に合法な盤面になる");
        let eval = evaluate_board(&complete, &[], &config.royalty, &config.fantasyland)
            .expect("完成盤面の評価は失敗しない");
        return MoveScore::Exact(
            !eval.foul,
            eval.royalty_total,
            [eval.bottom.hand, eval.middle.hand, eval.top.hand],
        );
    }

    MoveScore::Greedy(greedy_score(&rows, config))
}

fn greedy_score(rows: &[Vec<Card>; 3], config: &CpuConfig) -> i64 {
    // 完成段の役とロイヤリティ。
    // ロイヤリティは「自分より下の段がすべて完成している段」だけ数える。
    // 完成段ロイヤリティを無条件に支配項にすると、単発貪欲では上段を
    // 強いペアで早期完成させて自縄自縛(下の段が超えられずファウル)に
    // なることが対ランダム自己対戦で判明したため(28/30 がファウルした)。
    // solver の greedy は rollout の内側で使われるためこの弱点が顕在化
    // しないが、単発利用ではこのゲートが必要。
    let mut hands: [Option<HandRank>; 3] = [None, None, None];
    let mut royalty_total: u32 = 0;
    for idx in 0..3 {
        if rows[idx].len() != ROW_LIMITS[idx] {
            continue;
        }
        let others: Vec<Card> = (0..3)
            .filter(|i| *i != idx)
            .flat_map(|i| rows[i].iter().copied())
            .collect();
        let hand = evaluate_completed_row(idx, &rows[idx], &others);
        let below_complete = ((idx + 1)..3).all(|i| rows[i].len() == ROW_LIMITS[i]);
        if below_complete {
            royalty_total += royalty_points(
                [Row::Top, Row::Middle, Row::Bottom][idx],
                &hand,
                &config.royalty,
            );
        }
        hands[idx] = Some(hand);
    }

    // 完成段同士の順序違反(top ≤ middle ≤ bottom)は即除外
    for (lower, upper) in [(0, 1), (1, 2), (0, 2)] {
        if let (Some(l), Some(u)) = (&hands[lower], &hands[upper])
            && u < l
        {
            return i64::MIN;
        }
    }

    // 役ベースの秩序圧: 各段の「現時点で作られている役の強さ」を数値化し、
    // 上の段が下の段より強い分をペナルティにする。ランク和のバランス項だけだと
    // 「低いペア(役としては強い)が top に集まりファウル」する欠陥があるため。
    // 下の段が未完成の間は埋まり具合で減衰させる(まだ強くなる余地がある段に
    // 対して秩序を強制しすぎると弱くなる、という solver の知見のソフト版)。
    let strengths: [i64; 3] = std::array::from_fn(|idx| made_strength(&rows[idx], &hands[idx]));
    let mut pressure = 0i64;
    for (upper, lower) in [(0usize, 1usize), (1, 2), (0, 2)] {
        let over = (strengths[upper] - strengths[lower]).max(0);
        // 減衰: 下の段の埋まり具合(placed/limit)。完成段なら 1
        pressure += over * rows[lower].len() as i64 / ROW_LIMITS[lower] as i64;
    }

    // 行バランス: 高札は下・低札は上(2Σbottom + Σmiddle − Σtop)
    let rank_sum = |cards: &[Card]| -> i64 {
        cards
            .iter()
            .filter_map(|c| match c {
                Card::Standard { rank, .. } => Some(*rank as i64 + 2),
                Card::Joker => None,
            })
            .sum()
    };
    let balance = 2 * rank_sum(&rows[2]) + rank_sum(&rows[1]) - rank_sum(&rows[0]);

    // FL ボーナスは「top が他の段より強くない(= 秩序リスクを生まない)」
    // ときだけ与える。序盤の空いた盤面で QQ を top に置く博打は打たず、
    // 下の段が既に優位なときだけ FL 突入形を後押しする(安全な FL ハント)。
    // スケールはロイヤリティ項(×2000)と比較可能な大きさにしないと
    // made/pressure 項に埋もれて意思決定が一度も覆らない。
    let fl_bonus =
        if config.fl_aware && strengths[0] <= strengths[1] && strengths[0] <= strengths[2] {
            // φ はロイヤリティ点単位なので、ロイヤリティ項と同じ ×2000 で換算する
            (fl_setup_bonus(&rows[0], config) * 2000.0) as i64
        } else {
            0
        };

    // 作られた役は下の段ほど価値が高い(ペアやトリップスを下に集める動機。
    // ランク和のバランス項は「札の高さ」しか見ないため、この項がないと
    // 役を作る動機自体が生まれず、ロイヤリティ獲得もほぼゼロになる)
    let made = 2 * strengths[2] + strengths[1] - strengths[0];

    2000 * i64::from(royalty_total) + fl_bonus + balance + made - 12 * pressure
}

/// 段の「現時点で作られている役」の強さの数値化。
/// 完成段は役カテゴリ、未完成段はランクの重なり(ペア/トリップス等)のみで
/// 測る(ストレート/フラッシュのドローは数えない = 完成するまで秩序圧の
/// 対象にしない)。Joker は最大グループへの補完として近似する。
fn made_strength(cards: &[Card], complete_hand: &Option<HandRank>) -> i64 {
    if let Some(hand) = complete_hand {
        let top_rank = hand.tiebreak.first().map(|r| *r as i64).unwrap_or(12);
        return (hand.category as i64) * 200 + top_rank * 4;
    }
    let mut counts = [0i64; 13];
    let mut jokers = 0i64;
    for card in cards {
        match card {
            Card::Standard { rank, .. } => counts[*rank as usize] += 1,
            Card::Joker => jokers += 1,
        }
    }
    let (best_rank, best_count) = (0..13)
        .map(|r| (r as i64, counts[r]))
        .max_by_key(|(r, c)| (*c, *r))
        .expect("13 ランク分ある");
    let group = best_count + jokers;
    let base = match group {
        0 | 1 => return 0, // 役なし(ハイカード)は秩序圧の対象にしない
        2 => 200,          // pair 相当(Category::Pair=1 の ×200 に揃える)
        3 => 600,          // trips 相当
        _ => 1400,         // quads 相当
    };
    let second_pair = (0..13)
        .filter(|r| counts[*r] >= 2 && (*r as i64) != best_rank)
        .count() as i64;
    base + best_rank * 4 + second_pair * 200
}

fn evaluate_completed_row(idx: usize, cards: &[Card], others: &[Card]) -> HandRank {
    let has_joker = cards.iter().any(|c| matches!(c, Card::Joker));
    if has_joker {
        // 貪欲評価では行単独の最強解決で近似する(厳密な盤面全体解決は
        // 最終街の evaluate_board が行う)
        let arity = if idx == 0 {
            RowArity::Three
        } else {
            RowArity::Five
        };
        resolve_row(cards, arity, others)
            .expect("完成段の Joker 解決は失敗しない")
            .hand
    } else if idx == 0 {
        evaluate_three(cards).expect("完成段の評価は失敗しない")
    } else {
        evaluate_five(cards).expect("完成段の評価は失敗しない")
    }
}

/// 上段の FL 突入ポテンシャルへのボーナス(φ 傾斜)。
/// 素朴な行バランス項(高札を上段から遠ざける)が FL 突入価値を殺すのを
/// 相殺する。値は突入レベル別の φ(config.fl_entry_values)で、
/// 完成形はその φ、未完成の潜在形は割引した φ を与える。
/// 一律ボーナスだと「KK/AA を狙えるのに QQ で妥協する」歪みが出る。
/// 突入ハント(通常ハンド方策)に φ を反映する強度。
/// φ は「完璧に近い突入変換」を前提とした差分価値であり、単発貪欲が
/// 額面どおり支払うと平場の行力を売りすぎて総得点が下がることが
/// セッション検証で判明した(1.0 で +1.55/hand、baseline +1.88)。
/// 値はセッション総得点の掃引で校正する。
const FL_HUNT_SCALE: f64 = 0.075;

fn fl_setup_bonus(top: &[Card], config: &CpuConfig) -> f64 {
    let mut counts = [0u8; 13];
    let mut jokers = 0u8;
    for card in top {
        match card {
            Card::Standard { rank, .. } => counts[*rank as usize] += 1,
            Card::Joker => jokers += 1,
        }
    }
    let max_count = counts.iter().max().copied().unwrap_or(0);
    let best_paired_rank = (0..13).rev().find(|r| counts[*r] >= 2);
    let best_single_rank = (0..13).rev().find(|r| counts[*r] >= 1);
    let has_open_slot = top.len() < 3;

    let phi_for_pair_rank = |rank: usize| -> f64 {
        match rank {
            r if r == Rank::Queen as usize => config.fl_value_for_cards(14),
            r if r == Rank::King as usize => config.fl_value_for_cards(15),
            r if r == Rank::Ace as usize => config.fl_value_for_cards(16),
            _ => 0.0,
        }
    };

    let mut bonus = 0.0f64;
    // トリップス(17 枚突入)濃厚
    if max_count + jokers >= 3 && top.len() >= 2 {
        bonus = bonus.max(config.fl_value_for_cards(17));
    }
    // QQ 以上の完成ペア(Joker 補完込み)
    let paired_with_joker =
        best_paired_rank.or_else(|| (jokers >= 1).then_some(best_single_rank).flatten());
    if let Some(rank) = paired_with_joker
        && (counts[rank] >= 2 || jokers >= 1)
    {
        bonus = bonus.max(phi_for_pair_rank(rank));
    }
    // 潜在形(未完成)の割引は小さく抑える。0.2×φ 級にすると
    // 「エースを top に寝かせて AA を待つ」動きが平場の行力を削り、
    // セッション総得点が下がることが検証で判明した(+1.88→+1.55/hand)。
    if let Some(rank) = best_single_rank
        && has_open_slot
    {
        bonus = bonus.max(0.05 * phi_for_pair_rank(rank));
    }
    bonus * FL_HUNT_SCALE
}

// ---- Fantasyland 配置方策 ----

/// FL 盤面の価値: ファウルなら大きな負値、そうでなければ
/// 素点(ロイヤリティ合計)+ 継続できるなら φ(現在の配布枚数)。
/// Joker は evaluate_board が盤面全体で最適解決する。
pub fn fantasyland_value(board: &Board, config: &CpuConfig, fl_cards: u8) -> f64 {
    let eval = evaluate_board(board, &[], &config.royalty, &config.fantasyland)
        .expect("完成盤面の評価は失敗しない");
    if eval.foul {
        return -1000.0;
    }
    let stay = ofc_engine::fantasyland::fantasyland_stay(&eval.resolved, &config.fantasyland)
        .unwrap_or(false);
    // 継続は枚数維持ルール: 継続価値 = 現在の配布枚数の φ
    f64::from(eval.royalty_total)
        + if stay {
            config.fl_value_for_cards(fl_cards)
        } else {
            0.0
        }
}

/// FL の一括配置を選ぶ(13 枚配置 + 残り捨て)。
///
/// strength 1.0 は分枝限定法による厳密な最適盤面。下段候補をロイヤリティ
/// 降順に試し、「下段の役を固定すると中段は同カテゴリ以下・上段はさらに
/// 残りカードで実際に作れるペア/トリップスまで、と上限が確定する」ことを
/// 使って、既知の最良値を超えられない枝を丸ごと刈る(14 枚では独立実装の
/// 全列挙と一致することをテストで保証)。
///
/// strength s < 1.0 では、最適値 best に対して [1/3 + 2/3·s, 2/3 + 1/3·s]
/// × best の帯からターゲット値を seed で引き、ランダム候補サンプルの中から
/// 最も近い非ファウル盤面を選ぶ(s=0 で「最適の 1/3〜2/3 の出来」)。
/// どの強さでもファウル盤面は選ばない。
pub fn choose_fantasyland(
    dealt: &[Card],
    config: &CpuConfig,
    seed: u64,
) -> Option<(Vec<Placement>, Vec<Card>)> {
    if !(13..=17).contains(&dealt.len()) {
        return None;
    }

    let stay_value = config.fl_value_for_cards(dealt.len() as u8);
    let (best_value, best_masks) = fl_search_best(dealt, config, stay_value)?;

    let strength = config.strength.clamp(0.0, 1.0);
    let masks = if strength >= 1.0 {
        best_masks
    } else {
        let lo = 1.0 / 3.0 + (2.0 / 3.0) * strength;
        let hi = 2.0 / 3.0 + (1.0 / 3.0) * strength;
        let mut rng = Xorshift::new(seed);
        let u = (rng.next() % 10_000) as f64 / 10_000.0;
        let target = best_value * (lo + u * (hi - lo));
        fl_sample_near(dealt, config, stay_value, target, &mut rng).unwrap_or(best_masks)
    };

    let [b_mask, m_mask, t_mask] = masks;
    let mut placements = Vec::with_capacity(13);
    let mut discards = Vec::new();
    for (i, &card) in dealt.iter().enumerate() {
        let bit = 1u32 << i;
        let row = if b_mask & bit != 0 {
            Some(RowKind::Bottom)
        } else if m_mask & bit != 0 {
            Some(RowKind::Middle)
        } else if t_mask & bit != 0 {
            Some(RowKind::Top)
        } else {
            None
        };
        match row {
            Some(row) => placements.push(Placement { card, row }),
            None => discards.push(card),
        }
    }
    Some((placements, discards))
}

/// 分枝限定法で FL の厳密最適(価値, [bottom, middle, top] マスク)を求める。
fn fl_search_best(dealt: &[Card], config: &CpuConfig, stay_value: f64) -> Option<(f64, [u32; 3])> {
    let n = dealt.len();
    let stay_top = config.fantasyland.stays_on_top_trips();
    let stay_bottom = config.fantasyland.stays_on_bottom_quads_or_better();

    // 5 枚部分集合の役を前計算し、下段候補としてロイヤリティ→役の強さ順に並べる
    let five: Vec<(u32, HandRank, u32)> = combo_masks(n, 5)
        .into_iter()
        .map(|mask| {
            let hand = eval_subset(dealt, mask, RowArity::Five);
            let roy = royalty_points(Row::Bottom, &hand, &config.royalty);
            (mask, hand, roy)
        })
        .collect();
    let mut bottoms: Vec<&(u32, HandRank, u32)> = five.iter().collect();
    bottoms.sort_unstable_by(|a, b| (b.2, &b.1).cmp(&(a.2, &a.1)));

    // 中段ロイヤリティの「カテゴリ以下での最大値」前計算(下段カテゴリで引く)
    let mut mid_prefix_max = [0u32; 10];
    for cat in 0..10usize {
        let hand = HandRank {
            category: CATEGORY_ORDER[cat],
            tiebreak: vec![],
        };
        let roy = royalty_points(Row::Middle, &hand, &config.royalty);
        mid_prefix_max[cat] = roy.max(if cat > 0 { mid_prefix_max[cat - 1] } else { 0 });
    }

    let mut best: Option<(f64, [u32; 3])> = None;
    for entry in &bottoms {
        let (b_mask, b_hand, b_roy) = (entry.0, &entry.1, entry.2);
        let b_cat = b_hand.category as usize;

        // 下段確定分の継続価値と、残りカードから作れる上段の上限
        let stay_b = stay_bottom && b_hand.category >= Category::Quads;
        let remaining = rank_pool(dealt, b_mask);
        let top_ub = top_upper_bound(&remaining, b_cat, config);
        let stay_ub = stay_b
            || (stay_top && b_cat >= Category::Trips as usize && trips_achievable(&remaining));
        let bound = f64::from(b_roy + mid_prefix_max[b_cat] + top_ub)
            + if stay_ub { stay_value } else { 0.0 };
        if best.as_ref().is_some_and(|(bv, _)| bound <= *bv) {
            continue; // この下段では既知の最良を超えられない
        }

        for (m_mask, m_hand, _) in &five {
            if m_mask & b_mask != 0 || m_hand > b_hand {
                continue;
            }
            let m_roy = royalty_points(Row::Middle, m_hand, &config.royalty);
            let after_mid = rank_pool_minus(&remaining, dealt, *m_mask);
            let m_cat = m_hand.category as usize;
            let top_ub2 = top_upper_bound(&after_mid, m_cat, config);
            let stay_ub2 = stay_b
                || (stay_top && m_cat >= Category::Trips as usize && trips_achievable(&after_mid));
            let bound2 =
                f64::from(b_roy + m_roy + top_ub2) + if stay_ub2 { stay_value } else { 0.0 };
            if best.as_ref().is_some_and(|(bv, _)| bound2 <= *bv) {
                continue;
            }

            let rest: Vec<usize> = (0..n)
                .filter(|i| (b_mask | m_mask) & (1 << i) == 0)
                .collect();
            for t_indices in combo_masks(rest.len(), 3) {
                let t_mask: u32 = rest
                    .iter()
                    .enumerate()
                    .filter(|(j, _)| t_indices & (1 << j) != 0)
                    .map(|(_, i)| 1u32 << i)
                    .sum();
                let t_hand = eval_subset(dealt, t_mask, RowArity::Three);
                if &t_hand > m_hand {
                    continue;
                }
                let t_roy = royalty_points(Row::Top, &t_hand, &config.royalty);
                let stay = stay_b || (stay_top && t_hand.category == Category::Trips);
                let value = f64::from(b_roy + m_roy + t_roy) + if stay { stay_value } else { 0.0 };
                if best.as_ref().is_none_or(|(bv, _)| value > *bv) {
                    best = Some((value, [b_mask, *m_mask, t_mask]));
                }
            }
        }
    }
    best
}

/// 判別値順のカテゴリ一覧(mid_prefix_max の添字用)。
const CATEGORY_ORDER: [Category; 10] = [
    Category::HighCard,
    Category::Pair,
    Category::TwoPair,
    Category::Trips,
    Category::Straight,
    Category::Flush,
    Category::FullHouse,
    Category::Quads,
    Category::StraightFlush,
    Category::RoyalFlush,
];

/// (ランク別枚数, ジョーカー枚数)。上段の到達可能ロイヤリティ上限の計算用。
type RankPool = ([u8; 13], u8);

fn rank_pool(dealt: &[Card], used_mask: u32) -> RankPool {
    let mut counts = [0u8; 13];
    let mut jokers = 0u8;
    for (i, card) in dealt.iter().enumerate() {
        if used_mask & (1 << i) != 0 {
            continue;
        }
        match card {
            Card::Standard { rank, .. } => counts[*rank as usize] += 1,
            Card::Joker => jokers += 1,
        }
    }
    (counts, jokers)
}

fn rank_pool_minus(pool: &RankPool, dealt: &[Card], remove_mask: u32) -> RankPool {
    let (mut counts, mut jokers) = *pool;
    for (i, card) in dealt.iter().enumerate() {
        if remove_mask & (1 << i) == 0 {
            continue;
        }
        match card {
            Card::Standard { rank, .. } => counts[*rank as usize] -= 1,
            Card::Joker => jokers -= 1,
        }
    }
    (counts, jokers)
}

/// 残りカードから実際に作れる上段役(カテゴリ ≤ cap_cat)のロイヤリティ上限。
/// 「AA が残っていなければ TT が上限」のように、表ではなく在庫で絞る。
fn top_upper_bound(pool: &RankPool, cap_cat: usize, config: &CpuConfig) -> u32 {
    let (counts, jokers) = pool;
    let mut ub = 0u32;
    if cap_cat >= Category::Trips as usize {
        for (r, &count) in counts.iter().enumerate() {
            if count >= 1 && count + jokers >= 3 {
                let hand = HandRank {
                    category: Category::Trips,
                    tiebreak: vec![Rank::ALL_CPU[r]],
                };
                ub = ub.max(royalty_points(Row::Top, &hand, &config.royalty));
            }
        }
    }
    if cap_cat >= Category::Pair as usize {
        for (r, &count) in counts.iter().enumerate() {
            if count >= 2 || (count >= 1 && *jokers >= 1) {
                let hand = HandRank {
                    category: Category::Pair,
                    tiebreak: vec![Rank::ALL_CPU[r]],
                };
                ub = ub.max(royalty_points(Row::Top, &hand, &config.royalty));
            }
        }
    }
    ub
}

fn trips_achievable(pool: &RankPool) -> bool {
    let (counts, jokers) = pool;
    counts.iter().any(|&c| c >= 1 && c + jokers >= 3)
}

/// 帯選択用: ランダムな非ファウル分割をサンプリングし、価値が target に
/// 最も近いものを返す(弱い強さの精度は重要でないため厳密性は要らない)。
fn fl_sample_near(
    dealt: &[Card],
    config: &CpuConfig,
    stay_value: f64,
    target: f64,
    rng: &mut Xorshift,
) -> Option<[u32; 3]> {
    const SAMPLES: usize = 2000;
    let n = dealt.len();
    let mut best: Option<(f64, [u32; 3])> = None;
    for _ in 0..SAMPLES {
        // 13 枚をランダムに選び 5/5/3 に割る。2 つの 5 枚組は強い方を bottom に
        let mut indices: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            let j = (rng.next() % (i as u64 + 1)) as usize;
            indices.swap(i, j);
        }
        let mask_of = |slice: &[usize]| -> u32 { slice.iter().map(|i| 1u32 << i).sum() };
        let (five_a, five_b, three) = (
            mask_of(&indices[0..5]),
            mask_of(&indices[5..10]),
            mask_of(&indices[10..13]),
        );
        let hand_a = eval_subset(dealt, five_a, RowArity::Five);
        let hand_b = eval_subset(dealt, five_b, RowArity::Five);
        let (b_mask, b_hand, m_mask, m_hand) = if hand_a >= hand_b {
            (five_a, hand_a, five_b, hand_b)
        } else {
            (five_b, hand_b, five_a, hand_a)
        };
        let t_hand = eval_subset(dealt, three, RowArity::Three);
        if t_hand > m_hand {
            continue; // ファウル形はサンプルし直す
        }
        let roy = royalty_points(Row::Bottom, &b_hand, &config.royalty)
            + royalty_points(Row::Middle, &m_hand, &config.royalty)
            + royalty_points(Row::Top, &t_hand, &config.royalty);
        let stay = (config.fantasyland.stays_on_top_trips() && t_hand.category == Category::Trips)
            || (config.fantasyland.stays_on_bottom_quads_or_better()
                && b_hand.category >= Category::Quads);
        let value = f64::from(roy) + if stay { stay_value } else { 0.0 };
        let dist = (value - target).abs();
        if best.is_none_or(|(bd, _)| dist < bd) {
            best = Some((dist, [b_mask, m_mask, three]));
        }
    }
    best.map(|(_, masks)| masks)
}

/// n 要素から k 個選ぶ全組み合わせをビットマスクで返す。
fn combo_masks(n: usize, k: usize) -> Vec<u32> {
    let mut result = Vec::new();
    let mut mask = (1u32 << k) - 1;
    while mask < (1u32 << n) {
        result.push(mask);
        // Gosper's hack: 次に大きい同 popcount のビット列
        let c = mask & mask.wrapping_neg();
        let r = mask + c;
        mask = (((r ^ mask) >> 2) / c) | r;
    }
    result
}

/// マスクで選んだカード部分集合の役。Joker 入りは行単独の最強解決で近似する
/// (最終盤面の厳密解決は fantasyland_value / 採点側の evaluate_board が行う)。
fn eval_subset(dealt: &[Card], mask: u32, arity: RowArity) -> HandRank {
    let cards: Vec<Card> = (0..dealt.len())
        .filter(|i| mask & (1 << i) != 0)
        .map(|i| dealt[i])
        .collect();
    evaluate_completed_row(if arity == RowArity::Three { 0 } else { 2 }, &cards, &[])
}

/// Rank の判別値順一覧(engine の Rank::ALL は crate 内部のため複製)。
trait RankAllCpu {
    const ALL_CPU: [Rank; 13];
}
impl RankAllCpu for Rank {
    const ALL_CPU: [Rank; 13] = [
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
}

// ---- CRN rollout(最上位 strength の着手選択) ----

/// rollout の予算。samples は [初手, 街1, 街2, 街3] の CRN サンプル数。
/// 既定値はモバイル実機で 1 手数十 ms を狙う構成(solver の目安
/// 「30〜60 本 × 上位 K≈8」に準拠。街 3 は playout が最終街のみで
/// 軽いため多めに割く)。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RolloutBudget {
    /// 貪欲ランク上位いくつを rollout するか。小さすぎると FL 狙いの
    /// 正解を落とす(solver の経験則)。
    pub top_k: usize,
    pub samples: [u32; 4],
}

impl Default for RolloutBudget {
    fn default() -> Self {
        Self {
            top_k: 8,
            samples: [60, 60, 80, 200],
        }
    }
}

/// rollout に必要な盤面外の情報。
/// dead には「自分の捨て札 + 相手の公開盤面」など見えている消費札を渡す。
/// 相手はシミュレートしない(ソリティア値。solver の設計判断に準拠)。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DeadContext {
    /// デッキのジョーカー枚数(live デッキの復元に必要)。
    pub jokers: u8,
    pub dead: Vec<Card>,
}

/// strength 1.0 で CRN rollout を使う着手選択。strength < 1.0 は
/// choose_move_seeded(ε 混合貪欲)へ委譲する。
/// 街 4 は従来どおり厳密全列挙、街 3 以前は「貪欲上位 K 候補 ×
/// CRN サンプルの貪欲 playout 平均」で選ぶ。
pub fn choose_move_ctx(
    board: &Board,
    dealt: &[Card],
    street: Street,
    ctx: &DeadContext,
    config: &CpuConfig,
    seed: u64,
) -> Option<Move> {
    if config.strength.clamp(0.0, 1.0) < 1.0 {
        return choose_move_seeded(board, dealt, street, config, seed);
    }
    match street {
        Street::Finished | Street::Fantasyland => None,
        Street::Draw(4) => choose_move(board, dealt, street, config),
        _ => rollout_choose(board, dealt, street, ctx, config, seed),
    }
}

fn rollout_choose(
    board: &Board,
    dealt: &[Card],
    street: Street,
    ctx: &DeadContext,
    config: &CpuConfig,
    seed: u64,
) -> Option<Move> {
    let candidates = enumerate_moves(board, dealt, street)?;
    // 貪欲スコアで事前ランクし、上位 K のみ rollout する
    let mut ranked: Vec<(MoveScore, Move)> = candidates
        .into_iter()
        .map(|mv| (score_move(board, &mv, false, config), mv))
        .collect();
    ranked.sort_by(|a, b| b.0.cmp(&a.0));
    ranked.truncate(config.rollout.top_k);

    let live = live_deck(board, dealt, ctx);
    let (street_idx, draws_needed) = match street {
        Street::Initial => (0usize, 12usize),
        Street::Draw(n) => (n as usize, (4 - n as usize) * 3),
        _ => unreachable!("rollout_choose は Initial/Draw(1..=3) のみ"),
    };
    let samples = config.rollout.samples[street_idx.min(3)];

    let mut totals = vec![0.0f64; ranked.len()];
    let mut scenario = Vec::with_capacity(draws_needed);
    for index in 0..samples {
        // CRN: 全候補が同一 index の scenario を共有する(運が相殺され、
        // 候補間の EV 差の分散が激減する。CRN なしの少サンプルは
        // 事実上ランダム選択に退化する — solver の注意点)
        crn_scenario(seed, u64::from(index), &live, draws_needed, &mut scenario);
        for (i, (_, mv)) in ranked.iter().enumerate() {
            totals[i] += playout_value(board, mv, street, &scenario, config);
        }
    }

    let best = totals
        .iter()
        .enumerate()
        .max_by(|a, b| a.1.partial_cmp(b.1).expect("値は有限"))
        .map(|(i, _)| i)?;
    Some(ranked.swap_remove(best).1)
}

/// live デッキ = 全カード − 盤面 − 手札 − dead(ジョーカーは枚数で復元)。
fn live_deck(board: &Board, dealt: &[Card], ctx: &DeadContext) -> Vec<Card> {
    let mut used: Vec<Card> = Vec::with_capacity(30);
    used.extend(
        board
            .top()
            .iter()
            .chain(board.middle())
            .chain(board.bottom()),
    );
    used.extend(dealt);
    used.extend(&ctx.dead);
    let joker_used = used.iter().filter(|c| matches!(c, Card::Joker)).count() as u8;

    let suits = [Suit::Spades, Suit::Hearts, Suit::Diamonds, Suit::Clubs];
    let mut live: Vec<Card> = Rank::ALL_CPU
        .iter()
        .flat_map(|r| suits.map(|s| Card::Standard { rank: *r, suit: s }))
        .filter(|c| !used.contains(c))
        .collect();
    for _ in joker_used..ctx.jokers {
        live.push(Card::Joker);
    }
    live
}

/// (seed, index) から決定的に scenario(live デッキ先頭 need 枚の並び)を作る。
fn crn_scenario(seed: u64, index: u64, live: &[Card], need: usize, out: &mut Vec<Card>) {
    // SplitMix64 で index ごとに独立したシードを作る
    let mut z = seed ^ index.wrapping_mul(0x9E37_79B9_7F4A_7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    let mut rng = Xorshift::new(z ^ (z >> 31));

    // 部分 Fisher-Yates: 先頭 need 枚だけ確定させる
    out.clear();
    let mut pool = live.to_vec();
    for _ in 0..need.min(pool.len()) {
        let i = (rng.next() % pool.len() as u64) as usize;
        out.push(pool.swap_remove(i));
    }
}

/// 候補手を適用し、残りの街を貪欲(最終街のみ厳密)で打ち切り、終端価値を返す。
/// 終端価値 = ファウルなら 0、非ファウルなら自分のロイヤリティ +
/// FL 突入なら φ̂(枚数)。相手との差分は取らない(ソリティア値)。
/// playout が貪欲なので φ̂ も貪欲実現ベースの実測値と整合する
/// (完璧プレイ前提の φ を足すと FL を過大評価する — solver の注意点)。
fn playout_value(
    board: &Board,
    (placements, _discard): &Move,
    street: Street,
    scenario: &[Card],
    config: &CpuConfig,
) -> f64 {
    let mut rows = [
        board.top().to_vec(),
        board.middle().to_vec(),
        board.bottom().to_vec(),
    ];
    for p in placements {
        let idx = ROW_KINDS.iter().position(|r| *r == p.row).expect("3 行");
        rows[idx].push(p.card);
    }

    let mut next_street = match street {
        Street::Initial => 1u8,
        Street::Draw(n) => n + 1,
        _ => unreachable!(),
    };
    let mut cursor = 0usize;
    while next_street <= 4 {
        if cursor + 3 > scenario.len() {
            return 0.0; // live デッキ不足(通常の対戦では起きない)
        }
        let dealt = &scenario[cursor..cursor + 3];
        cursor += 3;
        let current = Board::new(rows[0].clone(), rows[1].clone(), rows[2].clone())
            .expect("playout 中の盤面は常に有効");
        let Some((placements, _)) = choose_move(&current, dealt, Street::Draw(next_street), config)
        else {
            return 0.0;
        };
        for p in &placements {
            let idx = ROW_KINDS.iter().position(|r| *r == p.row).expect("3 行");
            rows[idx].push(p.card);
        }
        next_street += 1;
    }

    let [top, middle, bottom] = rows;
    let final_board = Board::new(top, middle, bottom).expect("13 枚の完成盤面");
    let eval = evaluate_board(&final_board, &[], &config.royalty, &config.fantasyland)
        .expect("完成盤面の評価は失敗しない");
    if eval.foul {
        return 0.0;
    }
    f64::from(eval.royalty_total)
        + eval
            .fantasyland_cards
            .map(|cards| config.fl_value_for_cards(cards))
            .unwrap_or(0.0)
}
