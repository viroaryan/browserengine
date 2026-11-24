use nirvana_css::token::{DimensionUnit, Token};
use nirvana_css::tokenizer::CssTokenizer;

#[test]
fn test_tokenize_ruleset_tokens() {
    let css = "h1.title#header { color: #ff0000; margin: 10px; }";
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "h1"));
    assert!(matches!(&tokens[1], Token::Dot));
    assert!(matches!(&tokens[2], Token::Ident(s) if s == "title"));
    assert!(matches!(&tokens[3], Token::Hash(s) if s == "header"));
    assert!(matches!(&tokens[4], Token::OpenBrace));
    assert!(matches!(&tokens[5], Token::Ident(s) if s == "color"));
    assert!(matches!(&tokens[6], Token::Colon));
    assert!(matches!(&tokens[7], Token::Hash(s) if s == "ff0000"));
    assert!(matches!(&tokens[8], Token::Semicolon));
    assert!(matches!(&tokens[9], Token::Ident(s) if s == "margin"));
    assert!(matches!(&tokens[10], Token::Colon));
    assert!(matches!(&tokens[11], Token::Dimension(val, DimensionUnit::Px) if (*val - 10.0).abs() < f32::EPSILON));
    assert!(matches!(&tokens[12], Token::Semicolon));
    assert!(matches!(&tokens[13], Token::CloseBrace));
    assert!(matches!(&tokens[14], Token::Eof));
}

#[test]
fn test_tokenize_comments_and_whitespace_skip() {
    let css = "/* a comment */ div { /* nested */ width: 100px; }";
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "div"));
    assert!(matches!(&tokens[1], Token::OpenBrace));
    assert!(matches!(&tokens[2], Token::Ident(s) if s == "width"));
    assert!(matches!(&tokens[3], Token::Colon));
    assert!(matches!(&tokens[4], Token::Dimension(val, DimensionUnit::Px) if (*val - 100.0).abs() < f32::EPSILON));
    assert!(matches!(&tokens[5], Token::Semicolon));
    assert!(matches!(&tokens[6], Token::CloseBrace));
    assert!(matches!(&tokens[7], Token::Eof));
}

#[test]
fn test_tokenize_numbers_and_dimensions() {
    let css = "opacity: 0.5; width: 50%; font-size: 1.5em; z-index: -1;";
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "opacity"));
    assert!(matches!(&tokens[1], Token::Colon));
    assert!(matches!(&tokens[2], Token::Number(val) if (*val - 0.5).abs() < f32::EPSILON));
    assert!(matches!(&tokens[3], Token::Semicolon));

    assert!(matches!(&tokens[4], Token::Ident(s) if s == "width"));
    assert!(matches!(&tokens[5], Token::Colon));
    assert!(matches!(&tokens[6], Token::Dimension(val, DimensionUnit::Percent) if (*val - 50.0).abs() < f32::EPSILON));
    assert!(matches!(&tokens[7], Token::Semicolon));

    assert!(matches!(&tokens[8], Token::Ident(s) if s == "font-size"));
    assert!(matches!(&tokens[9], Token::Colon));
    assert!(matches!(&tokens[10], Token::Dimension(val, DimensionUnit::Em) if (*val - 1.5).abs() < f32::EPSILON));
    assert!(matches!(&tokens[11], Token::Semicolon));

    assert!(matches!(&tokens[12], Token::Ident(s) if s == "z-index"));
    assert!(matches!(&tokens[13], Token::Colon));
    assert!(matches!(&tokens[14], Token::Number(val) if (*val - (-1.0)).abs() < f32::EPSILON));
}

#[test]
fn test_tokenize_strings_and_commas() {
    let css = r#"font-family: "Helvetica Neue", 'Arial', sans-serif;"#;
    let tokens = CssTokenizer::new(css).tokenize_all();

    assert!(matches!(&tokens[0], Token::Ident(s) if s == "font-family"));
    assert!(matches!(&tokens[1], Token::Colon));
    assert!(matches!(&tokens[2], Token::String(s) if s == "Helvetica Neue"));
    assert!(matches!(&tokens[3], Token::Comma));
    assert!(matches!(&tokens[4], Token::String(s) if s == "Arial"));
    assert!(matches!(&tokens[5], Token::Comma));
    assert!(matches!(&tokens[6], Token::Ident(s) if s == "sans-serif"));
    assert!(matches!(&tokens[7], Token::Semicolon));
}
