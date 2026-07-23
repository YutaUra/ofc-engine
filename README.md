# ofc-engine

OFC(Open Face Chinese Poker)アプリを構築するためのコアロジックライブラリ。

作者の OFC 系アプリ群の共通土台として、役評価・ルール判定などの重複実装を解消するために存在する。詳細な背景とスコープは [docs/charter.md](docs/charter.md) を参照。

## 提供する機能(予定含む)

- カード・デッキ・ハンドの基本データ構造と役評価
- OFC 固有ルール: ファウル判定 / ロイヤリティ計算 / Fantasyland 判定
- ゲーム進行の状態管理
- AI / ソルバー(将来候補)

UI とネットワーク機能は含まない(利用側アプリの責務)。

## 使い方

TBD — 実装は Rust コア crate + wasm 配布層 + 薄い TS クライアントの 3 層構成([docs/architecture.md](docs/architecture.md) 参照)。API が固まり次第、利用方法を記載する。

## ドキュメント

[docs/README.md](docs/README.md) に文書構成と読む順序のガイドがある。

## メンテナ

個人プロジェクト(私的利用が主。公開・外部貢献の受け入れは成り行き)。
