# CLAUDE.md

> **Stability**: 🌊 living
> **最終更新**: 2026-07-24

## このリポジトリについて

OFC(Open Face Chinese Poker)アプリ群の共通土台となるコアロジックライブラリ。作業前に [docs/charter.md](docs/charter.md)(スコープ・Out of Scope)を確認すること。

## AI 協働ルール

- **スコープ厳守**: UI・ネットワーク・サーバー機能のコードを追加しない。求められたら charter の Out of Scope を指摘する
- **charter.md の変更は ADR 必須**: `docs/decisions/` に Nygard 形式で記録してから変更する
- **技術スタック未確定**: TypeScript / Rust(wasm) / MoonBit(wasm) を検討中。勝手に確定させない。選定の議論をしたら結論を ADR + architecture.md に反映する
- ロジックは決定的・シリアライズ可能に保つ(利用側がネットワーク対戦を実装できるようにするため)

## ドメイン用語

| 用語 | 意味 |
|------|------|
| OFC | Open Face Chinese Poker。13 枚を top(3)/middle(5)/bottom(5) の 3 列に配置するポーカー変種 |
| ファウル(Foul) | bottom ≥ middle ≥ top の強さ順を破った配置。全列没収 |
| ロイヤリティ(Royalty) | 特定の強い役に与えられるボーナス点 |
| Fantasyland | top に QQ 以上などの条件で突入する特殊ラウンド(14 枚一括配置) |
| Pineapple | 主流バリアント。初回 5 枚、以降 3 枚引いて 2 枚置き 1 枚捨てる |
