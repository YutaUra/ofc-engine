//! Joker 解決(行単位)のテスト。
//! 行内の Joker を、その行の役が最強になる標準カードへ置き換える。

use ofc_engine::Card;
use ofc_engine::hand::Category;
use ofc_engine::joker::{RowArity, resolve_row};

fn cards(wires: &[&str]) -> Vec<Card> {
    wires.iter().map(|w| w.parse().unwrap()).collect()
}

/// 解決結果の役カテゴリを返すヘルパ。
fn resolved_category(wires: &[&str], arity: RowArity, used: &[&str]) -> Category {
    let resolved = resolve_row(&cards(wires), arity, &cards(used)).unwrap();
    resolved.hand.category
}

#[test]
fn ジョーカーなしの行はそのまま評価される() {
    let resolved =
        resolve_row(&cards(&["As", "Ad", "9h", "5c", "2s"]), RowArity::Five, &[]).unwrap();
    assert_eq!(resolved.hand.category, Category::Pair);
    assert_eq!(resolved.cards, cards(&["As", "Ad", "9h", "5c", "2s"]));
}

#[test]
fn ジョーカー1枚はペアをトリップスに解決する() {
    assert_eq!(
        resolved_category(&["As", "Ad", "9h", "5c", "Xj"], RowArity::Five, &[]),
        Category::Trips
    );
}

#[test]
fn ジョーカーはストレート完成を優先できる() {
    // 9-8-7-6 + Xj → 5 か T でストレート(トリップスよりも強い解決はない状況)
    assert_eq!(
        resolved_category(&["9s", "8d", "7h", "6c", "Xj"], RowArity::Five, &[]),
        Category::Straight
    );
}

#[test]
fn ジョーカーはフラッシュ完成にも使える() {
    assert_eq!(
        resolved_category(&["As", "Ks", "9s", "5s", "Xj"], RowArity::Five, &[]),
        Category::Flush
    );
}

#[test]
fn ジョーカー2枚は最強の組み合わせに解決される() {
    // AAA + Xj Xj → クワッズ(AAAA)+ 最強キッカーではなくフルハウスでもなく、
    // AAAA + A は不可能なので AAAA + K 相当のクワッズが最強
    assert_eq!(
        resolved_category(&["As", "Ad", "Ah", "Xj", "Xj"], RowArity::Five, &[]),
        Category::Quads
    );
}

#[test]
fn 使用済みカードはジョーカーの解決先にできない() {
    // 9-8-7-6 + Xj のストレート完成に使える 5/T がすべて使用済みなら、
    // ストレートには解決できない(ペア以下の解決になる)
    let used = ["5s", "5h", "5d", "5c", "Ts", "Th", "Td", "Tc"];
    assert_eq!(
        resolved_category(&["9s", "8d", "7h", "6c", "Xj"], RowArity::Five, &used),
        Category::Pair
    );
}

#[test]
fn 三枚行のジョーカーはトリップスを狙う() {
    assert_eq!(
        resolved_category(&["Qs", "Qd", "Xj"], RowArity::Three, &[]),
        Category::Trips
    );
    // ハイカード 2 枚 + Xj は最上位ランクとのペアに解決される
    let resolved = resolve_row(&cards(&["As", "Kd", "Xj"]), RowArity::Three, &[]).unwrap();
    assert_eq!(resolved.hand.category, Category::Pair);
    assert_eq!(resolved.hand.tiebreak[0], ofc_engine::Rank::Ace);
}

#[test]
fn 解決結果には置き換え後のカードが含まれジョーカーは残らない() {
    let resolved =
        resolve_row(&cards(&["As", "Ad", "9h", "5c", "Xj"]), RowArity::Five, &[]).unwrap();
    assert_eq!(resolved.cards.len(), 5);
    assert!(
        resolved
            .cards
            .iter()
            .all(|c| matches!(c, Card::Standard { .. }))
    );
}

#[test]
fn 枚数違いはエラーになる() {
    assert!(resolve_row(&cards(&["As", "Kd"]), RowArity::Three, &[]).is_err());
    assert!(resolve_row(&cards(&["As", "Kd", "Qh"]), RowArity::Five, &[]).is_err());
}
