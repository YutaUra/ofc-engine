# Architecture

> **Stability**: 🌊 living
> **最終更新**: 2026-07-24
> **直近の変更 ADR**: [0002](decisions/0002-use-rust-core-with-wasm-distribution.md)(実装言語の選定)

実装・技術構成の現状を記す文書。自由に更新してよい。重要な決定の根拠は ADR(`docs/decisions/`)に残し、ここからリンクする。

## 技術スタック

**Rust**(決定の経緯は [ADR 0002](decisions/0002-use-rust-core-with-wasm-distribution.md))。構成は 3 層:

| 層 | 内容 | 利用者 |
|----|------|--------|
| コア crate | 役評価・ファウル・ロイヤリティ・状態管理の純 Rust 実装(wasm 非依存) | ソルバー系(chinese-poker-solver が crate 直接依存) |
| wasm 配布層 | wasm-bindgen。境界は JSON 文字列 API | JS 系ランタイム |
| 薄い TS クライアント | wasm のロードと型付け(npm パッケージ) | Web / Node / Bun / RN-WebView |

この構成は chinese-poker-solver で Web(wasm worker)/ React Native(WebView 内 wasm)/ Cloudflare Workers の全環境で実戦済みのパターンを踏襲する。

## wire 型(最優先の設計対象)

言語より API 境界のほうが変更困難なため、実装前に以下を設計・文書化する:

- 盤面表現(rows 文字列などのシリアライズ形式)
- 役評価・ファウル判定・ロイヤリティの返り値スキーマ

TBD(次回更新時に決定)— 設計したらこの節に記載する。

## 設計上の制約(charter 由来)

- UI・ネットワークの関心事をエンジンに持ち込まない(純粋なロジックライブラリに保つ)
- 利用側がネットワーク対戦を実装できるよう、ゲーム状態はシリアライズ可能・決定的に扱えることが望ましい

## モジュール構成

TBD(次回更新時に決定)— 実装開始時に記載する。

## テスト・品質

TBD(次回更新時に決定)— 言語決定後にテストフレームワークを選定。役評価などは網羅的なテーブルテストが有効な領域。
