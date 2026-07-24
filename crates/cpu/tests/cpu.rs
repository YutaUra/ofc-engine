//! CPU プレイヤー(貪欲ヒューリスティック)のテスト。
//! 方策の設計は chinese-poker-solver の greedy 方策を参考にしている:
//! 完成段のみのファウル判定 / ロイヤリティ支配項 / 行バランス / FL 補正 /
//! 最終街のみ厳密全列挙。

use ofc_engine::game::{GameState, Placement, RowKind, Street, random_move};
use ofc_engine::royalty::RoyaltyTable;
use ofc_engine::scoring::{ScoringRules, score_pair};
use ofc_engine::{Board, Card};
use ofc_engine_cpu::{CpuConfig, choose_move};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

fn config() -> CpuConfig {
    CpuConfig::standard()
}

#[test]
fn 初手は5枚の合法な配置を返す() {
    let board = Board::new(vec![], vec![], vec![]).unwrap();
    let dealt = cards(&["As", "Kd", "9h", "5c", "2s"]);
    let (placements, discard) = choose_move(&board, &dealt, Street::Initial, &config()).unwrap();
    assert_eq!(placements.len(), 5);
    assert_eq!(discard, None);
    // 全カードがちょうど 1 回ずつ使われる
    let mut used: Vec<Card> = placements.iter().map(|p| p.card).collect();
    used.sort_by_key(|c| c.to_string());
    let mut expected = dealt.clone();
    expected.sort_by_key(|c| c.to_string());
    assert_eq!(used, expected);
}

#[test]
fn 同じ入力には同じ手を返す() {
    let board = Board::new(vec![], vec![], vec![]).unwrap();
    let dealt = cards(&["As", "Kd", "9h", "5c", "2s"]);
    assert_eq!(
        choose_move(&board, &dealt, Street::Initial, &config()),
        choose_move(&board, &dealt, Street::Initial, &config())
    );
}

#[test]
fn 完成した段の順序が崩れる配置は選ばない() {
    // middle が trips 888 で完成済み。top を AA で完成させるとファウルなので、
    // AA を top に置く手は選ばれないはず
    let board = Board::new(
        cards(&["3s", "4h"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh"]),
    )
    .unwrap();
    // street: 3 枚から 2 枚置く。As Ad を両方 top に置くと top=AA+3s+4h?
    // top は残り 1 枠なので AA 両方は置けない。Th を bottom に置けばストレート完成
    let dealt = cards(&["As", "Th", "2c"]);
    let (placements, _) = choose_move(&board, &dealt, Street::Draw(4), &config()).unwrap();

    // Th は bottom のストレートを完成させる(ロイヤリティ 2)
    assert!(
        placements
            .iter()
            .any(|p| p.card.to_string() == "Th" && p.row == RowKind::Bottom)
    );
    // As を top に置いても A ハイ(ペア未満)なのでファウルしない。
    // どちらにせよ、選んだ手を適用した完成盤面はファウルではないこと
    let mut top = board.top().to_vec();
    let mut middle = board.middle().to_vec();
    let mut bottom = board.bottom().to_vec();
    for p in &placements {
        match p.row {
            RowKind::Top => top.push(p.card),
            RowKind::Middle => middle.push(p.card),
            RowKind::Bottom => bottom.push(p.card),
        }
    }
    let final_board = Board::new(top, middle, bottom).unwrap();
    assert_eq!(ofc_engine::foul::check_foul(&final_board), Ok(false));
}

#[test]
fn 最終街ではロイヤリティ最大の厳密解を選ぶ() {
    // top 2 枚(QQ 目前)+ bottom 4 枚(ストレート目前)。
    // 厳密評価なら As→top(QQ 維持 + A キッカー)ではなく
    // 「Th→bottom + 2c or As→top」でロイヤリティ合計を最大化する
    let board = Board::new(
        cards(&["Qs", "Qh"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Kh", "Qd", "Jh"]),
    )
    .unwrap();
    let dealt = cards(&["As", "Th", "2c"]);
    let (placements, discard) = choose_move(&board, &dealt, Street::Draw(4), &config()).unwrap();

    // 最適: Th→bottom(straight +2)、As→top(QQ は維持され royalty 7)、2c 捨て
    assert!(
        placements
            .iter()
            .any(|p| p.card.to_string() == "Th" && p.row == RowKind::Bottom)
    );
    assert!(
        placements
            .iter()
            .any(|p| p.card.to_string() == "As" && p.row == RowKind::Top)
    );
    assert_eq!(discard.map(|c| c.to_string()), Some("2c".to_string()));
}

#[test]
fn fl意識モードは下段が優位なときに上段のペア形成を優先する() {
    // middle はトリップスで完成済み・bottom も AAKK と強い。
    // この「top が QQ になっても秩序リスクがない」局面でだけ、
    // FL 意識モードは Qd を top に置いて QQ(FL 突入形)を作る
    let board = Board::new(
        cards(&["Qs"]),
        cards(&["8c", "8d", "8h", "Kc", "2s"]),
        cards(&["Ah", "Ad", "Kh", "Kd"]),
    )
    .unwrap();
    let dealt = cards(&["Qd", "3h", "4s"]);

    let fl_on = config();
    let (placements_on, _) = choose_move(&board, &dealt, Street::Draw(3), &fl_on).unwrap();
    assert!(
        placements_on
            .iter()
            .any(|p| p.card.to_string() == "Qd" && p.row == RowKind::Top),
        "FL 意識モードは Qd を top に置くはず: {placements_on:?}"
    );

    let fl_off = CpuConfig {
        fl_aware: false,
        ..config()
    };
    let (placements_off, _) = choose_move(&board, &dealt, Street::Draw(3), &fl_off).unwrap();
    assert!(
        !placements_off
            .iter()
            .any(|p| p.card.to_string() == "Qd" && p.row == RowKind::Top),
        "FL 非意識モードは QQ を top に作らないはず: {placements_off:?}"
    );
}

#[test]
fn fl意識モードでも秩序リスクがある序盤はqqを上段に置かない() {
    // 盤面がほぼ空(下の段の優位が未確立)なら、FL 意識モードでも
    // QQ top の博打は打たない
    let board = Board::new(cards(&["Qs"]), cards(&["8c", "8d"]), cards(&["Ah", "Kh"])).unwrap();
    let dealt = cards(&["Qd", "3h", "4s"]);
    let (placements, _) = choose_move(&board, &dealt, Street::Draw(1), &config()).unwrap();
    assert!(
        !placements
            .iter()
            .any(|p| p.card.to_string() == "Qd" && p.row == RowKind::Top),
        "秩序リスクのある局面では QQ top を避けるはず: {placements:?}"
    );
}

#[test]
fn cpuはランダムより明確に強い() {
    // CPU(player0) vs ランダム(player1) を 30 ゲーム対戦させ、合計得点が正
    let royalty = RoyaltyTable::standard_american();
    let scoring = ScoringRules::standard();
    let mut total = 0i32;
    for game_seed in 0..30 {
        let mut state = GameState::new(2, 0, game_seed).unwrap();
        let mut rand_seed = 10_000 + game_seed;
        while state.street() != Street::Finished {
            let mv: (Vec<Placement>, Option<Card>) = if state.current_player() == 0 {
                choose_move(
                    state.board(0),
                    state.dealt_cards(),
                    state.street(),
                    &config(),
                )
                .unwrap()
            } else {
                random_move(&state, rand_seed).unwrap()
            };
            rand_seed += 1;
            state.apply(&mv.0, mv.1).unwrap();
        }
        let (p0, _) = score_pair(state.board(0), state.board(1), &royalty, &scoring).unwrap();
        total += p0;
    }
    assert!(total > 0, "CPU の合計得点が正であるべき: {total}");
}
