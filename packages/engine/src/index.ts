/**
 * ofc-engine の薄い TS クライアント。
 * wasm の JSON 文字列 API を型付きで包むだけで、ロジックは一切持たない(ADR 0002)。
 */

import init, {
  evaluate_board_json,
  game_apply_json,
  game_new_fl_json,
  game_new_json,
  game_view_json,
  score_matchup_json,
  standard_ruleset_json,
  type InitInput,
} from "../wasm/ofc_engine.js";
import type {
  Board,
  Card,
  EvaluateBoardResult,
  GameApiResult,
  GamePlacement,
  GameStateBlob,
  GameViewResult,
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
  /** ゲーム開始。seed は number/bigint/string いずれでも可(内部で文字列化)。 */
  newGame(players: number, jokers: number, seed: number | bigint | string): GameApiResult;
  /**
   * FL ハンドを含むゲームを開始する。flCards はプレイヤーごとの FL 配布枚数
   * (0 = 通常。例 [14, 0])。FL プレイヤーは 13 枚配置 + 残りを Card[] で
   * 捨てる 1 回の applyMove で完了する。
   */
  newFantasylandGame(
    players: number,
    jokers: number,
    seed: number | bigint | string,
    flCards: number[],
  ): GameApiResult;
  /**
   * 着手適用。初手は discard を null、街では Card 1 枚、FL 手番では
   * Card[](残り全部)を渡す。
   */
  applyMove(
    state: GameStateBlob,
    placements: GamePlacement[],
    discard: Card | Card[] | null,
  ): GameApiResult;
  /** 保存済み state から現在状態を再構築する(中断復帰)。 */
  gameView(state: GameStateBlob): GameViewResult;
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
    newGame: (players, jokers, seed) =>
      JSON.parse(game_new_json(players, jokers, String(seed))) as GameApiResult,
    newFantasylandGame: (players, jokers, seed, flCards) =>
      JSON.parse(
        game_new_fl_json(players, jokers, String(seed), JSON.stringify(flCards)),
      ) as GameApiResult,
    applyMove: (state, placements, discard) =>
      JSON.parse(
        game_apply_json(state, JSON.stringify(placements), JSON.stringify(discard)),
      ) as GameApiResult,
    gameView: (state) => JSON.parse(game_view_json(state)) as GameViewResult,
  };
}
