# CLAUDE.md

> **Stability**: 🌊 living
> **最終更新**: 2026-07-24

## このリポジトリについて

OFC(Open Face Chinese Poker)アプリ群の共通土台となるコアロジックライブラリ。作業前に [docs/charter.md](docs/charter.md)(スコープ・Out of Scope)を確認すること。

## AI 協働ルール

- **スコープ厳守**: UI・ネットワーク・サーバー機能のコードを追加しない。求められたら charter の Out of Scope を指摘する
- **charter.md の変更は ADR 必須**: `docs/decisions/` に Nygard 形式で記録してから変更する
- **技術スタックは Rust で確定**(ADR 0002): コアは純 Rust crate(wasm 非依存)、JS 系向けは wasm-bindgen + JSON 文字列 API + 薄い TS 型。この 3 層構成を崩さない。コア crate に wasm/JS 依存を持ち込まない
- ロジックは決定的・シリアライズ可能に保つ(利用側がネットワーク対戦を実装できるようにするため)
- **wire 型の不変条件**(ADR 0003): 段構成 3/5/5 は固定。ルールは RuleSet→`compile()`→CompiledRules 方式で、評価関数のホットループ内でルール config を参照しない。wire のキー名は TS 型定義と serde で厳密一致

## ドメイン用語

| 用語 | 意味 |
|------|------|
| OFC | Open Face Chinese Poker。13 枚を top(3)/middle(5)/bottom(5) の 3 列に配置するポーカー変種 |
| ファウル(Foul) | bottom ≥ middle ≥ top の強さ順を破った配置。全列没収 |
| ロイヤリティ(Royalty) | 特定の強い役に与えられるボーナス点 |
| Fantasyland | top に QQ 以上などの条件で突入する特殊ラウンド(14 枚一括配置) |
| Pineapple | 主流バリアント。初回 5 枚、以降 3 枚引いて 2 枚置き 1 枚捨てる |
