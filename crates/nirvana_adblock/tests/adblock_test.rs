use nirvana_adblock::filter::ContentType;
use nirvana_adblock::{parse_line, AdBlockEngine, FilterAction, FilterRule};
use nirvana_url::Url;

#[test]
fn test_parse_network_rules_and_options() {
    let rule = parse_line("||doubleclick.net^$script,third-party").unwrap();
    if let FilterRule::Network(nr) = rule {
        assert_eq!(nr.pattern, "doubleclick.net^");
        assert!(nr.is_domain_anchor);
        assert_eq!(nr.content_types, ContentType::SCRIPT);
        assert_eq!(nr.third_party, Some(true));
        assert_eq!(nr.action, FilterAction::Block);
    } else {
        panic!("Expected network rule");
    }

    // Whitelist rule
    let wl = parse_line("@@||google.com/search$image").unwrap();
    if let FilterRule::Network(nr) = wl {
        assert_eq!(nr.pattern, "google.com/search");
        assert_eq!(nr.action, FilterAction::Allow);
        assert_eq!(nr.content_types, ContentType::IMAGE);
    } else {
        panic!("Expected whitelist rule");
    }
}

#[test]
fn test_parse_cosmetic_filters() {
    let global_cosmetic = parse_line("##.adsbygoogle").unwrap();
    if let FilterRule::Cosmetic(cr) = global_cosmetic {
        assert_eq!(cr.selector, ".adsbygoogle");
        assert!(cr.domains.is_empty());
    } else {
        panic!("Expected cosmetic rule");
    }

    let domain_cosmetic = parse_line("news.ycombinator.com,~sub.news.com##.sponsored-story").unwrap();
    if let FilterRule::Cosmetic(cr) = domain_cosmetic {
        assert_eq!(cr.selector, ".sponsored-story");
        assert_eq!(cr.domains, vec!["news.ycombinator.com"]);
        assert_eq!(cr.excluded_domains, vec!["sub.news.com"]);
    } else {
        panic!("Expected domain cosmetic rule");
    }
}

#[test]
fn test_network_blocking_and_whitelist_precedence() {
    let mut engine = AdBlockEngine::empty();
    engine.add_rule("||doubleclick.net^");
    engine.add_rule("@@||doubleclick.net/safe.js^");

    let ad_url = Url::parse("https://ad.doubleclick.net/banner.png").unwrap();
    let safe_url = Url::parse("https://doubleclick.net/safe.js").unwrap();
    let normal_url = Url::parse("https://wikipedia.org/wiki/Rust").unwrap();

    // Ad is blocked
    assert!(engine.should_block(&ad_url, Some("https://example.com"), ContentType(ContentType::IMAGE)));
    assert_eq!(engine.blocked_count(), 1);

    // Whitelisted is allowed
    assert!(!engine.should_block(&safe_url, Some("https://example.com"), ContentType(ContentType::SCRIPT)));
    assert_eq!(engine.blocked_count(), 1); // count unchanged

    // Normal URL is allowed
    assert!(!engine.should_block(&normal_url, Some("https://example.com"), ContentType(ContentType::IMAGE)));
}

#[test]
fn test_third_party_and_content_type_filtering() {
    let mut engine = AdBlockEngine::empty();
    engine.add_rule("||tracking.com^$third-party,script");

    let track_script = Url::parse("https://tracking.com/pixel.js").unwrap();

    // From third-party site (example.com -> tracking.com) => BLOCKED
    assert!(engine.should_block(
        &track_script,
        Some("https://example.com"),
        ContentType(ContentType::SCRIPT)
    ));

    // Same request but type is IMAGE instead of SCRIPT => ALLOWED
    assert!(!engine.should_block(
        &track_script,
        Some("https://example.com"),
        ContentType(ContentType::IMAGE)
    ));

    // First-party request (tracking.com -> tracking.com) => ALLOWED because of $third-party
    assert!(!engine.should_block(
        &track_script,
        Some("https://tracking.com"),
        ContentType(ContentType::SCRIPT)
    ));
}

#[test]
fn test_cosmetic_stylesheet_generation() {
    let mut engine = AdBlockEngine::empty();
    engine.add_rule("##.adsbygoogle");
    engine.add_rule("##div.banner-ad");
    engine.add_rule("techblog.com##.sponsored-link");

    // On techblog.com, both global and domain rules are included
    let css_tech = engine.cosmetic_stylesheet(Some("https://techblog.com/post/1")).unwrap();
    assert!(css_tech.contains(".adsbygoogle"));
    assert!(css_tech.contains("div.banner-ad"));
    assert!(css_tech.contains(".sponsored-link"));
    assert!(css_tech.contains("display: none !important"));

    // On another site, domain rule for techblog is NOT included
    let css_other = engine.cosmetic_stylesheet(Some("https://wikipedia.org")).unwrap();
    assert!(css_other.contains(".adsbygoogle"));
    assert!(css_other.contains("div.banner-ad"));
    assert!(!css_other.contains(".sponsored-link"));
}

#[test]
fn test_builtin_ruleset_blocks_known_trackers() {
    let engine = AdBlockEngine::new();
    let ga_url = Url::parse("https://www.google-analytics.com/analytics.js").unwrap();
    let criteo_url = Url::parse("https://static.criteo.net/js/ld/ld.js").unwrap();
    let taboola_url = Url::parse("https://cdn.taboola.com/libtrc/ad.js").unwrap();

    assert!(engine.should_block(&ga_url, Some("https://mysite.com"), ContentType(ContentType::SCRIPT)));
    assert!(engine.should_block(&criteo_url, Some("https://mysite.com"), ContentType(ContentType::SCRIPT)));
    assert!(engine.should_block(&taboola_url, Some("https://mysite.com"), ContentType(ContentType::SCRIPT)));
    assert_eq!(engine.blocked_count(), 3);
}
