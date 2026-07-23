/**
 * ofc-engine wire 型定義(ADR 0003)。
 *
 * この型は Rust 側 serde のキー名と厳密一致させる単一の真実。
 * 変更するときは crates/engine/src/ruleset.rs / crates/engine-wasm/src/lib.rs と
 * 同時に変更し、テスト(crates/engine-wasm/tests/json_api.rs)で往復を確認する。
 */

/** "As" / "Td" / "2c" 形式。Joker は "Xj"。 */
export type Card = string;

/** ランク 1 文字: "2".."9" | "T" | "J" | "Q" | "K" | "A" */
export type RankChar =
  | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9"
  | "T" | "J" | "Q" | "K" | "A";

/** 役カテゴリの安定キー。ローカライズは UI 側の責務。 */
export type Category =
  | "high_card"
  | "pair"
  | "two_pair"
  | "trips"
  | "straight"
  | "flush"
  | "full_house"
  | "quads"
  | "straight_flush"
  | "royal_flush";

/** 段構成 3/5/5 は不変条件(ADR 0003)。 */
export interface Board {
  top: Card[];
  middle: Card[];
  bottom: Card[];
}

export interface RuleSet {
  variant: string;
  players: number;
  deck: { jokers: number };
  scoring: { rowPoint: number; scoopBonus: number };
  royalties: {
    top: {
      pair: Partial<Record<RankChar, number>>;
      trips: Partial<Record<RankChar, number>>;
    };
    middle: Partial<Record<Category, number>>;
    bottom: Partial<Record<Category, number>>;
  };
  fantasyland: {
    pairCards: Partial<Record<RankChar, number>>;
    tripsCards: number;
    stayTopTrips: boolean;
    stayBottomQuadsOrBetter: boolean;
  };
}

export interface HandRank {
  category: Category;
  tiebreak: RankChar[];
}

export interface RowEvaluation {
  hand: HandRank;
  royalty: number;
}

/** evaluate_board_json の成功時の返り値。 */
export interface BoardEvaluation {
  foul: boolean;
  top: RowEvaluation;
  middle: RowEvaluation;
  bottom: RowEvaluation;
  /** ファウル時は 0。 */
  royaltyTotal: number;
  /** FL 突入時の配布枚数。突入しない/ファウル時は null。 */
  fantasylandCards: number | null;
  /** Joker 解決後の盤面。 */
  resolved: Board;
}

/** score_matchup_json の成功時の返り値。プレイヤー順の得点(ゼロサム)。 */
export interface MatchupResult {
  totals: number[];
}

/** 失敗時は各 API がこの形を返す(wasm 境界で panic しない)。 */
export interface EngineError {
  error: string;
}

export type EvaluateBoardResult = BoardEvaluation | EngineError;
export type ScoreMatchupResult = MatchupResult | EngineError;

export function isEngineError(result: unknown): result is EngineError {
  return typeof result === "object" && result !== null && "error" in result;
}
