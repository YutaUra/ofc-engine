/**
 * ofc-engine の薄い TS クライアント。
 * wasm の JSON 文字列 API を型付きで包むだけで、ロジックは一切持たない(ADR 0002)。
 */

import init, {
  evaluate_board_json,
  score_matchup_json,
  standard_ruleset_json,
  type InitInput,
} from "../wasm/ofc_engine.js";
import type {
  Board,
  Card,
  EvaluateBoardResult,
  RuleSet,
  ScoreMatchupResult,
} from "./types.ts";

export * from "./types.ts";

export interface Engine {
  /** 標準 Pineapple ルールセット。部分編集してローカルルールを作れる。 */
  standardRuleSet(): RuleSet;
  /** 盤面評価(役 + ロイヤリティ + ファウル + FL + Joker 解決)。 */
  evaluateBoard(board: Board, used: Card[], ruleset: RuleSet): EvaluateBoardResult;
  /** 総当たり採点(1-6 + scoop + ロイヤリティ差分)。 */
  scoreMatchup(boards: Board[], ruleset: RuleSet): ScoreMatchupResult;
}

/**
 * wasm を初期化してエンジンを返す。
 * `wasmInput` を省略すると wasm ファイルを相対 URL で fetch する(Web 向け)。
 * Node やバンドラ構成では ArrayBuffer / URL を明示的に渡す。
 */
export async function createEngine(wasmInput?: InitInput): Promise<Engine> {
  await init(wasmInput === undefined ? undefined : { module_or_path: wasmInput });
  return {
    standardRuleSet: () => JSON.parse(standard_ruleset_json()) as RuleSet,
    evaluateBoard: (board, used, ruleset) =>
      JSON.parse(
        evaluate_board_json(
          JSON.stringify(board),
          JSON.stringify(used),
          JSON.stringify(ruleset),
        ),
      ) as EvaluateBoardResult,
    scoreMatchup: (boards, ruleset) =>
      JSON.parse(
        score_matchup_json(JSON.stringify(boards), JSON.stringify(ruleset)),
      ) as ScoreMatchupResult,
  };
}
