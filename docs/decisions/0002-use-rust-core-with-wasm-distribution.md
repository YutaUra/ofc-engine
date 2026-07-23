# 0002: 実装言語は Rust(コア crate + wasm 配布層)とする

- **日付**: 2026-07-24
- **状態**: accepted

## Context

ofc-engine は Web フロント・Node/Bun サーバー・モバイルアプリ(フレームワーク未確定)の 3 環境から利用され、将来 EV 計算・モンテカルロのソルバーを本格化する意向がある。候補は A: TypeScript(npm) / B: Rust(wasm) / C: MoonBit(wasm) の 3 つ。

decision-council(6 役 × 2 ラウンド)と、実利用者である chinese-poker-solver(既存の Rust ソルバー実装を持ち、将来 ofc-engine への置き換えを予定)への照会で検証した。

決定的だった事実:

- **OFC ではモンテカルロの最内ループそのものが役評価・ファウル判定・ロイヤリティ計算**であり、「重い計算だけ後から Rust に切り出す」という切り分け線は引けない。TS を選ぶと「ルールの真実」が TS/Rust に二重実装され、境界ケース(ロイヤリティ端数・ファウル判定)の乖離事故が構造化される
- chinese-poker-solver の実測では役評価ホットループの言語差が方策 1 段で約 300〜1000 倍効く(数十万〜数百万 rollout)
- 「Rust コア一枚岩 + wasm 配布層 + 薄い TS クライアント」構成は chinese-poker-solver が Web(wasm worker)/ React Native(WebView 内 wasm)/ Cloudflare Workers の全環境で実戦済み。モバイル未確定リスクは実証済みパターンの踏襲で緩和できる

## Decision

- コアロジック(役評価・ファウル判定・ロイヤリティ計算・状態管理)は **Rust crate** として実装する。ソルバー系利用者(chinese-poker-solver)は crate を直接依存する
- JS 系利用者(Web / Node / Bun / RN-WebView)向けには **wasm-bindgen による wasm 配布層 + 薄い TS 型**を提供する。wasm 境界は JSON 文字列 API に保つ
- 実装言語より先に **wire 型(盤面表現・役評価の返り値スキーマ)を設計・文書化**する
- 最初のマイルストーンは chinese-poker-solver の `crates/solver` 役評価部の ofc-engine crate への差し替え

## Alternatives Considered

- **A: TypeScript(npm)** — council 6 役中 4 役(Future Self / End User / Pragmatist / Maintainer)が Round 1 で支持。根拠は wasm 境界の保守コスト回避と JS エコシステムの厚さ。しかしその中心前提「重い計算は後から切り出せる」が Round 2 の Steel-Manner の反駁と実利用者の実測で崩れたため不採用。二重実装債務が既存 Rust ソルバーとの間で初日から発生する
- **C: MoonBit(wasm)** — 全 7 視点が非推奨で全会一致。言語・ツールチェーンの成熟度リスク(破壊的変更・情報量・テスト基盤・LLM 支援)が、得られる書きやすさに見合わない
- **ライブラリ化自体の保留(1 アプリ内にベタ書き)** — Devil's Advocate が提案。ただし chinese-poker-solver という 2 つ目の実利用者が既に存在するため、分離の実需要は確認済みと判断

## Devil's Advocate からの反論(記録)

- 高コストの本丸は言語ではなくデータモデル/API 境界設計 → Decision の wire 型先行設計として採用
- wasm ツールチェーン(wasm-bindgen 等)の破壊的変更で「無変更のロジックがビルド不能になる」恒常リスクは B の実コストとして残る。chinese-poker-solver の実証済み構成を流用して最小化する

## Consequences

- 役評価ロジックの単一真実源が Rust に一元化され、ソルバーと表示系の判定乖離が原理的に消える
- wasm ビルドパイプライン・TS 型の同期という恒常保守コストを引き受ける(A なら不要だったもの)
- Web の初回ロードに wasm バイナリ取得・初期化コストが乗る(End User の懸念)。サイズ・遅延は計測して管理する
- **撤退条件**:
  1. 将来選んだモバイル構成で WebView-wasm 方式が成立せず、ネイティブバインディング整備が利用側開発を止めるとき → その環境向け TS 実装の併設を再検討(差分テスト必須)
  2. ソルバー路線を放棄し、wasm 境界の保守が開発時間の大半を食うとき → TS への書き直しを検討(ロジックが枯れていれば移植は安い)
