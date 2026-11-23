use nirvana_html::token::{Token, TagData, DoctypeData};

#[test]
fn test_token_creation_and_attributes() {
    let mut tag = TagData::new("div", false);
    tag.add_attribute("class", "container");
    tag.add_attribute("ID", "main");

    assert_eq!(tag.name, "div");
    assert_eq!(tag.get_attribute("class"), Some("container"));
    assert_eq!(tag.get_attribute("id"), Some("main")); // Case-insensitive check

    let token = Token::StartTag(tag);
    assert!(matches!(token, Token::StartTag(_)));
}

#[test]
fn test_doctype_token() {
    let doctype = DoctypeData::new("html");
    let token = Token::Doctype(doctype);
    if let Token::Doctype(d) = token {
        assert_eq!(d.name, "html");
    } else {
        panic!("Expected Doctype token");
    }
}
