use nirvana_html::tokenizer::Tokenizer;
use nirvana_html::token::Token;

#[test]
fn test_tokenize_simple_tags_and_text() {
    let mut tokenizer = Tokenizer::new("<p class=\"intro\">Hello &amp; World</p>");
    let tokens = tokenizer.tokenize_all();

    assert!(matches!(&tokens[0], Token::StartTag(tag) if tag.name == "p" && tag.get_attribute("class") == Some("intro")));
    assert!(matches!(&tokens[tokens.len() - 2], Token::EndTag(tag) if tag.name == "p"));
    assert!(matches!(&tokens[tokens.len() - 1], Token::Eof));
}

#[test]
fn test_tokenize_self_closing_and_unquoted_attributes() {
    let mut tokenizer = Tokenizer::new("<img src=logo.png alt='Logo' />");
    let tokens = tokenizer.tokenize_all();

    if let Token::StartTag(tag) = &tokens[0] {
        assert_eq!(tag.name, "img");
        assert_eq!(tag.get_attribute("src"), Some("logo.png"));
        assert_eq!(tag.get_attribute("alt"), Some("Logo"));
        assert!(tag.self_closing);
    } else {
        panic!("Expected StartTag for img");
    }
}

#[test]
fn test_tokenize_comments_and_doctype() {
    let mut tokenizer = Tokenizer::new("<!DOCTYPE html><!-- a comment --><span>text</span>");
    let tokens = tokenizer.tokenize_all();

    assert!(matches!(&tokens[0], Token::Doctype(d) if d.name == "html"));
    assert!(matches!(&tokens[1], Token::Comment(c) if c == " a comment "));
    assert!(matches!(&tokens[2], Token::StartTag(tag) if tag.name == "span"));
}

#[test]
fn test_tag_open_non_alphabetic_character() {
    let mut tokenizer = Tokenizer::new("a < b and c <= d");
    let tokens = tokenizer.tokenize_all();

    let text: String = tokens
        .into_iter()
        .filter_map(|t| match t {
            Token::Character(c) => Some(c),
            _ => None,
        })
        .collect();

    assert_eq!(text, "a < b and c <= d");
}

#[test]
fn test_multi_hyphen_and_bogus_comments() {
    let mut tokenizer = Tokenizer::new("<!-- divider ---><p>content</p><?xml version=\"1.0\"?><div>xml</div>");
    let tokens = tokenizer.tokenize_all();

    assert!(matches!(&tokens[0], Token::Comment(c) if c == " divider -"));
    assert!(matches!(&tokens[1], Token::StartTag(tag) if tag.name == "p"));
    // Bogus comment for <?xml ...?>
    let has_div = tokens.iter().any(|t| matches!(t, Token::StartTag(tag) if tag.name == "div"));
    assert!(has_div);
}

#[test]
fn test_unquoted_attribute_adjacent_self_closing_slash() {
    let mut tokenizer = Tokenizer::new("<img src=logo.png/>");
    let tokens = tokenizer.tokenize_all();

    if let Token::StartTag(tag) = &tokens[0] {
        assert_eq!(tag.name, "img");
        assert_eq!(tag.get_attribute("src"), Some("logo.png"));
        assert!(tag.self_closing);
    } else {
        panic!("Expected StartTag for img");
    }
}
