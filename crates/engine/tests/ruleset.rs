//! RuleSet(JSON で完全データ化されたルール)と compile のテスト(ADR 0003)。
//! wire のキーは camelCase、ランクは "6".."A" の 1 文字。

use ofc_engine::hand::Category;
use ofc_engine::royalty::{Row, royalty_points};
use ofc_engine::ruleset::RuleSet;

fn standard_json() -> serde_json::Value {
    serde_json::json!({
        "variant": "pineapple",
        "players": 2,
        "deck": { "jokers": 1 },
        "scoring": { "rowPoint": 1, "scoopBonus": 3 },
        "royalties": {
            "top": {
                "pair": { "6": 1, "7": 2, "8": 3, "9": 4, "T": 5, "J": 6, "Q": 7, "K": 8, "A": 9 },
                "trips": { "2": 10, "3": 11, "4": 12, "5": 13, "6": 14, "7": 15, "8": 16,
                            "9": 17, "T": 18, "J": 19, "Q": 20, "K": 21, "A": 22 }
            },
            "middle": { "trips": 2, "straight": 4, "flush": 8, "full_house": 12,
                         "quads": 20, "straight_flush": 30, "royal_flush": 50 },
            "bottom": { "straight": 2, "flush": 4, "full_house": 6,
                         "quads": 10, "straight_flush": 15, "royal_flush": 25 }
        },
        "fantasyland": {
            "pairCards": { "Q": 14, "K": 15, "A": 16 },
            "tripsCards": 17,
            "stayTopTrips": true,
            "stayBottomQuadsOrBetter": true
        }
    })
}

#[test]
fn ルールセットをjsonから読み込みcompileできる() {
    let ruleset: RuleSet = serde_json::from_value(standard_json()).unwrap();
    let compiled = ruleset.compile().unwrap();

    assert_eq!(compiled.players, 2);
    assert_eq!(compiled.jokers, 1);
    assert_eq!(compiled.scoring.row_point, 1);
    assert_eq!(compiled.scoring.scoop_bonus, 3);
}

#[test]
fn compileしたロイヤリティ表は標準プリセットと一致する() {
    let ruleset: RuleSet = serde_json::from_value(standard_json()).unwrap();
    let compiled = ruleset.compile().unwrap();
    assert_eq!(
        compiled.royalty,
        ofc_engine::royalty::RoyaltyTable::standard_american()
    );
}

#[test]
fn compileしたflルールは標準プリセットと一致する() {
    let ruleset: RuleSet = serde_json::from_value(standard_json()).unwrap();
    let compiled = ruleset.compile().unwrap();
    assert_eq!(
        compiled.fantasyland,
        ofc_engine::fantasyland::FantasylandRules::standard_progressive()
    );
}

#[test]
fn 標準プリセットはjsonなしでも得られる() {
    let compiled = RuleSet::standard_pineapple().compile().unwrap();
    assert_eq!(compiled.jokers, 0); // 標準は Joker なし
    assert_eq!(
        compiled.royalty,
        ofc_engine::royalty::RoyaltyTable::standard_american()
    );
}

#[test]
fn ローカルルールの部分的な差し替えがjsonで表現できる() {
    let mut json = standard_json();
    // bottom のトリップスに 1 点、FL は一律 14 枚のローカルルール
    json["royalties"]["bottom"]["trips"] = serde_json::json!(1);
    json["fantasyland"]["pairCards"] = serde_json::json!({ "Q": 14, "K": 14, "A": 14 });
    json["fantasyland"]["tripsCards"] = serde_json::json!(14);

    let compiled = serde_json::from_value::<RuleSet>(json)
        .unwrap()
        .compile()
        .unwrap();

    let trips = ofc_engine::hand::HandRank {
        category: Category::Trips,
        tiebreak: vec![ofc_engine::Rank::Eight],
    };
    assert_eq!(royalty_points(Row::Bottom, &trips, &compiled.royalty), 1);
}

#[test]
fn jokerが3枚以上のルールはcompileで拒否される() {
    let mut json = standard_json();
    json["deck"]["jokers"] = serde_json::json!(3);
    let ruleset: RuleSet = serde_json::from_value(json).unwrap();
    assert!(ruleset.compile().is_err());
}

#[test]
fn 未知のキーはデシリアライズで拒否される() {
    // キー名の打ち間違い(rowPoints 等)を沈黙させないための deny_unknown_fields
    let mut json = standard_json();
    json["scoring"] = serde_json::json!({ "rowPoints": 1, "scoopBonus": 3 });
    assert!(serde_json::from_value::<RuleSet>(json).is_err());
}
