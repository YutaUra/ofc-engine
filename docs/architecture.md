# Architecture

> **Stability**: 🌊 living
> **最終更新**: 2026-07-24
> **直近の変更 ADR**: [0003](decisions/0003-wire-types-and-ruleset-compilation.md)(wire 型と RuleSet 方式)

実装・技術構成の現状を記す文書。自由に更新してよい。重要な決定の根拠は ADR(`docs/decisions/`)に残し、ここからリンクする。

## 技術スタック

**Rust**(決定の経緯は [ADR 0002](decisions/0002-use-rust-core-with-wasm-distribution.md))。構成は 3 層:

| 層 | 内容 | 利用者 |
|----|------|--------|
| コア crate | 役評価・ファウル・ロイヤリティ・状態管理の純 Rust 実装(wasm 非依存) | ソルバー系(chinese-poker-solver が crate 直接依存) |
| wasm 配布層 | wasm-bindgen。境界は JSON 文字列 API | JS 系ランタイム |
| 薄い TS クライアント | wasm のロードと型付け(npm パッケージ) | Web / Node / Bun / RN-WebView |

この構成は chinese-poker-solver で Web(wasm worker)/ React Native(WebView 内 wasm)/ Cloudflare Workers の全環境で実戦済みのパターンを踏襲する。

## wire 型(設計確定: [ADR 0003](decisions/0003-wire-types-and-ruleset-compilation.md))

chinese-poker-solver の現行形式と互換。境界は JSON 文字列、TS 型と serde のキー名は厳密一致させる(TS 型定義を単一真実源とする)。

### 基本型

- **Card**: `"As"`, `"Td"`, `"2c"`(ランク大文字 1 字 + スート小文字 1 字)。Joker は `"Xj"`
- **Board**: `{"top": Card[3], "middle": Card[5], "bottom": Card[5]}` — **段構成 3/5/5・13 枚は不変条件**(可変化するなら const generics。実行時分岐にしない)
- 役カテゴリは安定キー(`"pair"`, `"trips"`, `"straight_flush"`, …)で返し、ローカライズは UI 側の責務

### RuleSet → CompiledRules

ローカルルールは `RuleSet`(JSON で完全データ化)として受ける:

```json
{
  "variant": "pineapple",
  "players": 2,
  "deck": { "jokers": 1 },
  "scoring": { "rowPoint": 1, "scoopBonus": 3 },
  "royalties": { "top": {}, "middle": {}, "bottom": {} },
  "foul": { "royaltyZero": true },
  "fantasyland": {
    "entry": { "minTop": "pair_QQ" },
    "cards": { "pair_QQ": 14, "pair_KK": 15, "pair_AA": 16, "trips": 17 },
    "stay": { "topTrips": true, "bottomQuadsOrBetter": true }
  }
}
```

- `compile(ruleset) -> CompiledRules` を init 時に 1 回呼び、検証 + lookup 表化する。評価関数は CompiledRules のみ参照(ホットループで config を読まない)
- progressive FL は `fantasyland.cards` の役→枚数マップで表現(非 progressive は全キー同値)
- `jokers: 0` のとき Joker 解決パスは構造的にスキップ。解決の既定セマンティクスは「盤面全体で最適(ファウル回避のため弱める解決も可)」で、解決結果は位置ベースの `jokerResolution` として返す
- 標準ルールはプリセット関数で提供し、wire 上は常に完全展開形

### エンジンの責務境界

- 提供: `evaluate_board`(役 + ロイヤリティ + ファウル + FL 判定)、`score_matchup`(複数盤面の 1-6 + scoop 採点。人数は配列)
- 非提供: FL の EV 再帰(`FantasylandValues` 相当の注入口のみ)、探索・サンプリング系入力(draw / samples / seed 等はソルバー層の型)

## 設計上の制約(charter 由来)

- UI・ネットワークの関心事をエンジンに持ち込まない(純粋なロジックライブラリに保つ)
- 利用側がネットワーク対戦を実装できるよう、ゲーム状態はシリアライズ可能・決定的に扱えることが望ましい

## モジュール構成

```
crates/engine        — コア crate(純 Rust、wasm 非依存)
  card / board       — wire 表記・3/5/5 不変条件
  hand / foul        — 役評価・ファウル判定
  royalty / scoring  — ロイヤリティ表・1-6 採点
  fantasyland        — FL 突入/継続(progressive、データ駆動)
  joker / evaluate   — Joker 解決(行単位 / 盤面全体最適)・evaluate_board
  ruleset            — RuleSet(JSON)→ compile() → CompiledRules
  game               — Pineapple 進行の状態機械(seed 決定的)
crates/engine-wasm   — wasm 配布層。JSON 文字列 API のみ(evaluate_board_json 等)
packages/engine/src/types.ts — wire 型の TS 定義(キー名の単一真実源)
```

## テスト・品質

- `cargo test`(統合テストが日本語テスト名の「動く仕様書」)+ `cargo clippy -- -D warnings` + `cargo fmt`
- 開発は TDD(Red → Green → fmt/clippy)。エッジケース(ホイール、スチールホイール、Joker のファウル回避解決など)をテストで固定している
- wasm 層は host ターゲットで同一関数をテストし、`cargo check --target wasm32-unknown-unknown` でビルドを検証

## パフォーマンス

`cargo bench --bench eval`(criterion)で計測。最適化は「参照実装オラクル
(役評価は全 2,598,960 手網羅、Joker 解決はランダム盤面)との厳密一致」を
安全網に段階的に実施。release/bench は codegen-units=1 + thin LTO 固定
(codegen 分割の偶然でベンチが 2 倍以上ぶれる事象があったため)。

主な手法: 役評価のカウント配列+ビットマスク化 / RoyaltyTable の配列化 /
Joker 解決の行キャッシュ+ランク×フラッシュ関与の同値類削減 / 遅延 clone。

参考値(M系 Mac、2026-07): evaluate_five ~21ns/手、score_pair ~130ns、
evaluate_board = joker なし ~150ns / 1 枚 ~2.3µs / 2 枚 ~6.7µs。
