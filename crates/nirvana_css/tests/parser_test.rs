use nirvana_core::color::Rgba;
use nirvana_css::ast::{PropertyValue, SimpleSelector, Specificity};
use nirvana_css::parser::parse_css;

#[test]
fn test_parse_ruleset_and_specificity() {
    let css = r#"
        div.container#main {
            color: #ff0000;
            background-color: #00ff00;
            margin: 10px;
            display: block;
        }
    "#;
    let stylesheet = parse_css(css);
    assert_eq!(stylesheet.rules.len(), 1);

    let rule = &stylesheet.rules[0];
    let selector = &rule.selectors[0];
    assert_eq!(selector.simple_selectors.len(), 3);
    assert!(selector.simple_selectors.contains(&SimpleSelector::Tag("div".into())));
    assert!(selector.simple_selectors.contains(&SimpleSelector::Class("container".into())));
    assert!(selector.simple_selectors.contains(&SimpleSelector::Id("main".into())));

    // Specificity: 1 ID, 1 Class, 1 Tag -> (1, 1, 1)
    assert_eq!(selector.specificity(), Specificity::new(1, 1, 1));

    assert_eq!(rule.get_property("color"), Some(&PropertyValue::Color(Rgba::new(255, 0, 0, 255))));
    assert_eq!(rule.get_property("display"), Some(&PropertyValue::Keyword("block".into())));
    assert_eq!(rule.get_property("margin-top"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-right"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-bottom"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-left"), Some(&PropertyValue::Px(10.0)));
}

#[test]
fn test_parse_multiple_selectors_and_shorthand_margins() {
    let css = "h1, h2 { margin: 10px 20px; }";
    let stylesheet = parse_css(css);
    assert_eq!(stylesheet.rules.len(), 1);

    let rule = &stylesheet.rules[0];
    assert_eq!(rule.selectors.len(), 2);
    assert_eq!(rule.selectors[0].specificity(), Specificity::new(0, 0, 1));
    assert_eq!(rule.selectors[1].specificity(), Specificity::new(0, 0, 1));

    // Shorthand margin: 10px 20px -> top/bottom: 10px, left/right: 20px
    assert_eq!(rule.get_property("margin-top"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-bottom"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("margin-left"), Some(&PropertyValue::Px(20.0)));
    assert_eq!(rule.get_property("margin-right"), Some(&PropertyValue::Px(20.0)));
}

#[test]
fn test_parse_auto_and_shorthand_padding() {
    let css = "div { margin: 0 auto; padding: 5px 10px 15px 20px; }";
    let stylesheet = parse_css(css);
    let rule = &stylesheet.rules[0];

    assert_eq!(rule.get_property("margin-top"), Some(&PropertyValue::Px(0.0)));
    assert_eq!(rule.get_property("margin-bottom"), Some(&PropertyValue::Px(0.0)));
    assert_eq!(rule.get_property("margin-left"), Some(&PropertyValue::Auto));
    assert_eq!(rule.get_property("margin-right"), Some(&PropertyValue::Auto));

    assert_eq!(rule.get_property("padding-top"), Some(&PropertyValue::Px(5.0)));
    assert_eq!(rule.get_property("padding-right"), Some(&PropertyValue::Px(10.0)));
    assert_eq!(rule.get_property("padding-bottom"), Some(&PropertyValue::Px(15.0)));
    assert_eq!(rule.get_property("padding-left"), Some(&PropertyValue::Px(20.0)));
}

#[test]
fn test_specificity_ordering() {
    let id_spec = Specificity::new(1, 0, 0);
    let class_spec = Specificity::new(0, 2, 0);
    let tag_spec = Specificity::new(0, 0, 5);

    assert!(id_spec > class_spec);
    assert!(class_spec > tag_spec);
    assert!(Specificity::new(0, 1, 1) > Specificity::new(0, 1, 0));
}
