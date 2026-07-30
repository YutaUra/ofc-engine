//! オープニングブック(初手テーブル)のテスト。
//! 正準化は chinese-poker-solver の canonical_with_perm 仕様に一致させる。

use ofc_engine::game::{RowKind, Street};
use ofc_engine::{Board, Card};
use ofc_engine_cpu::{CpuConfig, DeadContext, OpeningBook, choose_move_ctx};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

#[test]
fn 正準鍵はスート置換で不変() {
    // 同じ形(スートだけ入れ替えた手)は同じ正準鍵になる
    let a = cards(&["As", "Ah", "9s", "5c", "2d"]);
    let b = cards(&["Ac", "Ad", "9c", "5h", "2s"]); // s→c, h→d, c→h, d→s の置換
    assert_eq!(
        OpeningBook::canonical_key(&a),
        OpeningBook::canonical_key(&b)
    );
}

#[test]
fn 正準鍵はカードの並び順にも不変() {
    let a = cards(&["As", "Ah", "9s", "5c", "2d"]);
    let b = cards(&["2d", "9s", "As", "5c", "Ah"]);
    assert_eq!(
        OpeningBook::canonical_key(&a),
        OpeningBook::canonical_key(&b)
    );
}

#[test]
fn ジョーカーはスート中立に扱われる() {
    let a = cards(&["As", "Kh", "9s", "5c", "Xj"]);
    let b = cards(&["Ah", "Ks", "9h", "5d", "Xj"]);
    assert_eq!(
        OpeningBook::canonical_key(&a),
        OpeningBook::canonical_key(&b)
    );
}

/// フィクスチャ: 正準手 1 行(As Ah 9s 5c 2d 相当の正準形)。
/// 最良配置 = AA を bottom、次点 = AA を middle とする。
fn fixture_book() -> OpeningBook {
    // 正準鍵のテキストを実装から取得してフィクスチャを組み立てる
    // (鍵の具体文字列に依存しないため、実装の canonical_key と往復で整合する)
    let hand = cards(&["As", "Ah", "9s", "5c", "2d"]);
    let key_cards = OpeningBook::canonical_cards(&hand);
    let key_line: Vec<String> = key_cards.iter().map(|c| c.to_string()).collect();
    // key_cards はコード昇順なので並びは [2,5,9,A,A]。rows は 1:1 対応。
    // 1 本目: AA bottom(roy 5.0、fl 低)/ 2 本目: AA middle(roy 3.0、fl0 高め)
    let line = format!(
        "{}\ttmmbb:5.0:0.02,0.01,0.01,0.00:0.20\ttbbmm:3.0:0.05,0.00,0.00,0.00:0.30",
        key_line.join(" ")
    );
    OpeningBook::from_text(&line).unwrap()
}

#[test]
fn ブックから初手配置を引けて元のスートに戻る() {
    let book = fixture_book();
    let hand = cards(&["As", "Ah", "9s", "5c", "2d"]);
    let placements = book
        .lookup(&hand, &CpuConfig::standard())
        .expect("フィクスチャに載っている手");
    assert_eq!(placements.len(), 5);
    // 引いた配置は「元の手札のカード」で構成されている
    let mut used: Vec<String> = placements.iter().map(|p| p.card.to_string()).collect();
    used.sort();
    let mut expected: Vec<String> = hand.iter().map(|c| c.to_string()).collect();
    expected.sort();
    assert_eq!(used, expected);
    // 盤面として合法(行あふれなし)
    let mut rows: [Vec<Card>; 3] = [vec![], vec![], vec![]];
    for p in &placements {
        let idx = match p.row {
            RowKind::Top => 0,
            RowKind::Middle => 1,
            RowKind::Bottom => 2,
        };
        rows[idx].push(p.card);
    }
    let [t, m, b] = rows;
    Board::new(t, m, b).unwrap();
}

#[test]
fn 再ランクはファイの値に応じて配置を入れ替える() {
    let book = fixture_book();
    let hand = cards(&["As", "Ah", "9s", "5c", "2d"]);

    // φ̂ が小さい設定: roy 重視 → 1 本目(roy 5.0)が勝つ
    let mut low_phi = CpuConfig::standard();
    low_phi.fl_phi_table = [[1.0; 4]; 11];
    let p = book.lookup(&hand, &low_phi).unwrap();
    let aa_row = p
        .iter()
        .find(|pl| pl.card.to_string() == "As")
        .map(|pl| pl.row)
        .unwrap();
    assert_eq!(aa_row, RowKind::Bottom, "roy 重視なら 1 本目(AA bottom)");

    // φ̂ が巨大な設定: fl 重視 → 2 本目(fl0 0.05 > 1 本目 0.02)が勝つ
    let mut high_phi = CpuConfig::standard();
    high_phi.fl_phi_table = [[1000.0, 0.0, 0.0, 0.0]; 11];
    let p = book.lookup(&hand, &high_phi).unwrap();
    let aa_row = p
        .iter()
        .find(|pl| pl.card.to_string() == "As")
        .map(|pl| pl.row)
        .unwrap();
    assert_eq!(aa_row, RowKind::Middle, "fl 重視なら 2 本目(AA middle)");
}

#[test]
fn rollout帯の初手はブックがあればブックを使う() {
    let book = fixture_book();
    ofc_engine_cpu::set_opening_book(book);
    let board = Board::new(vec![], vec![], vec![]).unwrap();
    let hand = cards(&["As", "Ah", "9s", "5c", "2d"]);
    let ctx = DeadContext {
        jokers: 0,
        dead: vec![],
        hand_seed: Some(1),
    };
    let (placements, discard) = choose_move_ctx(
        &board,
        &hand,
        Street::Initial,
        &ctx,
        &CpuConfig::standard(),
        1,
    )
    .unwrap();
    assert_eq!(discard, None);
    // ブックの 1 本目(標準 φ̂ では roy 優位)どおり AA が bottom に置かれる
    let aa_rows: Vec<RowKind> = placements
        .iter()
        .filter(|p| p.card.to_string().starts_with('A'))
        .map(|p| p.row)
        .collect();
    assert_eq!(aa_rows, vec![RowKind::Bottom, RowKind::Bottom]);
}
