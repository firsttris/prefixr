use super::*;

#[test]
fn steam_app_ids_come_from_umu_ids() {
    assert_eq!(steam_app_id("umu-1091500"), Some(1091500));
    assert_eq!(steam_app_id("umu-default"), None);
    assert_eq!(steam_app_id("1091500"), None);
}

#[test]
fn protondb_summaries_give_their_tier() {
    let summary = r#"{"bestReportedTier":"platinum","confidence":"strong","score":0.79,"tier":"Gold","total":1234,"trendingTier":"platinum"}"#;
    assert_eq!(parse_tier(summary).as_deref(), Some("gold"));
    assert_eq!(parse_tier(r#"{"tier":"pending"}"#), None);
    assert_eq!(parse_tier("not json"), None);
}

#[test]
fn cached_tiers_expire_sooner_when_protondb_had_none() {
    let rated = CachedTier {
        tier: Some("gold".to_string()),
        fetched_at: 0,
    };
    let unrated = CachedTier {
        tier: None,
        fetched_at: 0,
    };
    assert!(fresh(&rated, 2 * 24 * 60 * 60));
    assert!(!fresh(&unrated, 2 * 24 * 60 * 60));
    assert!(!fresh(&rated, 8 * 24 * 60 * 60));
}

#[test]
fn store_keys_split_into_store_and_app_name() {
    assert_eq!(
        parse_store_key("gog:1207658924"),
        Some((StoreKind::Gog, "1207658924".to_string()))
    );
    assert_eq!(
        parse_store_key("epic:Sugar:Plum"),
        Some((StoreKind::Epic, "Sugar:Plum".to_string()))
    );
    assert_eq!(
        parse_store_key("3f1c2a52-9a43-4f53-9a8e-1b2c3d4e5f60"),
        None
    );
}
