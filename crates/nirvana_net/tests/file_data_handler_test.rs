use nirvana_net::handler::{DataUriHandler, FileHandler, ProtocolHandler};
use nirvana_url::Url;

#[test]
fn test_data_uri_handler_plain_and_base64() {
    let handler = DataUriHandler;
    assert!(handler.can_handle("data"));

    // Plain text data URI
    let plain_url = Url::parse("data:text/html,<h1>Hello</h1>").unwrap();
    let resp = handler.fetch(&plain_url).unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.mime_type.as_deref(), Some("text/html"));
    assert_eq!(resp.text().unwrap(), "<h1>Hello</h1>");

    // Base64 encoded data URI ("body { color: red; }")
    let b64_url = Url::parse("data:text/css;base64,Ym9keSB7IGNvbG9yOiByZWQ7IH0=").unwrap();
    let resp_b64 = handler.fetch(&b64_url).unwrap();
    assert_eq!(resp_b64.status, 200);
    assert_eq!(resp_b64.mime_type.as_deref(), Some("text/css"));
    assert_eq!(resp_b64.text().unwrap(), "body { color: red; }");
}

#[test]
fn test_file_handler_reads_local_file() {
    let handler = FileHandler;
    assert!(handler.can_handle("file"));

    let cargo_path = std::env::current_dir().unwrap().join("Cargo.toml");
    let file_url_str = format!(
        "file:///{}",
        cargo_path.to_str().unwrap().replace('\\', "/")
    );
    let url = Url::parse(&file_url_str).unwrap();

    let resp = handler.fetch(&url).unwrap();
    assert_eq!(resp.status, 200);
    assert!(resp.text().unwrap().contains("name = \"nirvana_net\""));
}
