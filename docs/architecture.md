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

TBD(次回更新時に決定)— 実装開始時に記載する。

## テスト・品質

TBD(次回更新時に決定)— 言語決定後にテストフレームワークを選定。役評価などは網羅的なテーブルテストが有効な領域。
