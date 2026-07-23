# 0001: アーキテクチャ上の決定を ADR として記録する

- **日付**: 2026-07-24
- **状態**: accepted

## Context

ofc-engine は複数アプリの共通土台として長期に使われる想定であり、「なぜこの設計にしたか」が失われると将来の変更判断が困難になる。また `docs/charter.md`(🪨 stable)の改訂履歴を残す仕組みが必要。

## Decision

Michael Nygard 形式の ADR を `docs/decisions/` に記録する。

- **必須**: charter.md の改訂
- **任意**: architecture.md の設計判断のうち重要なもの

ADR は「決定の根拠」、architecture.md は「現状の構成」を担い、二重管理しない(ADR から architecture.md へリンクする)。

## Consequences

- 不変領域(charter)の変更に履歴が残り、将来の文脈復元が可能になる
- 決定のたびに ADR を書くコストが発生するが、対象を絞ることで最小化する
