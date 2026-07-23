# 0003: wire 型は solver 互換とし、ルールバリアントは RuleSet→CompiledRules 方式で可変化する

- **日付**: 2026-07-24
- **状態**: accepted

## Context

ofc-engine は OFC のローカルルール(Joker 有無、progressive FL の配布枚数、FL 継続条件、ロイヤリティ表、scoop 点、人数など)に対応したい。一方、最初の移行先である chinese-poker-solver はこれらを決めうちで実装しており、かつモンテカルロのホットループ性能上「実行時にルール config を参照されると困る」制約がある(実利用者照会で確認)。

solver 側から得た制約:

- 段サイズ 3/5/5・13 枚構造は配置列挙・盤面ビット表現・opening book のキー空間すべての前提。実行時可変は全域に効く
- ホットループ内のカード表現は u8/bit。wire 文字列との変換は境界 1 回に限定
- 役評価・ロイヤリティ表は init 時に lookup 表へ固定化するならバリアント可変でも性能無害
- Joker 解決は組合せ爆発するため、Joker なしバリアントでは解決パスを完全スキップできる構造が必要
- FL の再帰 EV は solver 側の責務。エンジンは注入値を受ける口だけ用意する

## Decision

1. **wire 形式は chinese-poker-solver の現行形式に合わせる**: カード = `"As"` 形式(Joker は `"Xj"`)、盤面 = `{top, middle, bottom}` の Card 配列、役カテゴリは安定キー(`"straight_flush"` 等)、境界は JSON 文字列で TS 型と serde のキー名を厳密一致させる
2. **段構成 3/5/5(13 枚)は不変条件として固定する**。可変化する場合は const generics 等のコンパイル時分岐で行い、実行時分岐にはしない
3. **ルールバリアントは `RuleSet`(JSON で完全データ化)として受け、`compile(ruleset) -> CompiledRules` で init 時に検証+lookup 表化する**。評価関数は CompiledRules のみを参照し、実行時にルール config を読まない。progressive FL は entry 役→枚数のマップで表現
4. **`jokers: 0` のとき Joker 解決パスを構造的にスキップする**。Joker 解決の既定セマンティクスは「盤面全体で最適(ファウル回避のため弱める解決も可)」
5. **FL の EV 再帰はエンジンの責務外**。`FantasylandValues` 相当の注入口のみ提供する。同様に、探索・サンプリング系の入力(draw/samples/seed 等)はソルバー層の型であり ofc-engine には持ち込まない

## Consequences

- ローカルルール対応がエンジン改修なしのデータ変更で可能になり、かつ solver のホットループ性能を損なわない
- solver 互換の wire 形式により、chinese-poker-solver の役評価部差し替え(最初のマイルストーン)の移行コストが下がる
- RuleSet の検証(不正なロイヤリティ表や矛盾する FL 条件の拒否)を compile 段で実装する義務を負う
- 段サイズを変えるバリアント(2-7 Pineapple 等の非 3/5/5 系)は当面対応外。必要になったら本 ADR を改訂し const generics で対応する
