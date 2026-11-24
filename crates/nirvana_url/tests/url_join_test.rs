use nirvana_url::Url;

#[test]
fn test_url_display_and_as_str() {
    let url = Url::parse("https://example.com:8080/path/doc.html?query=1#frag").unwrap();
    assert_eq!(
        url.as_str(),
        "https://example.com:8080/path/doc.html?query=1#frag"
    );
    assert_eq!(
        format!("{}", url),
        "https://example.com:8080/path/doc.html?query=1#frag"
    );
}

#[test]
fn test_url_join_relative_paths() {
    let base = Url::parse("https://example.com/blog/article.html").unwrap();

    // Sibling relative path
    let sibling = base.join("style.css").unwrap();
    assert_eq!(sibling.as_str(), "https://example.com/blog/style.css");

    // Parent relative path
    let parent = base.join("../assets/main.css").unwrap();
    assert_eq!(parent.as_str(), "https://example.com/assets/main.css");

    // Root-relative path
    let root = base.join("/app.css").unwrap();
    assert_eq!(root.as_str(), "https://example.com/app.css");

    // Scheme-relative path
    let scheme_rel = base.join("//cdn.example.org/lib.js").unwrap();
    assert_eq!(scheme_rel.as_str(), "https://cdn.example.org/lib.js");

    // Absolute URL override
    let abs = base.join("http://other.com/index.html").unwrap();
    assert_eq!(abs.as_str(), "http://other.com/index.html");
}
