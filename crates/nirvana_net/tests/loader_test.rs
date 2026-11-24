use nirvana_net::error::NetError;
use nirvana_net::handler::ProtocolHandler;
use nirvana_net::loader::ResourceLoader;
use nirvana_net::response::Response;
use nirvana_url::Url;

#[test]
fn test_resource_loader_dispatches_data_and_file() {
    let loader = ResourceLoader::new();

    // Data URI dispatch
    let resp = loader.fetch_string("data:text/plain,Nirvana").unwrap();
    assert_eq!(resp.status, 200);
    assert_eq!(resp.text().unwrap(), "Nirvana");

    // Cache hit verification
    let resp2 = loader.fetch_string("data:text/plain,Nirvana").unwrap();
    assert_eq!(resp2.text().unwrap(), "Nirvana");
}

#[test]
fn test_resource_loader_unsupported_scheme() {
    let loader = ResourceLoader::new();
    let err = loader.fetch_string("ftp://example.com/file").unwrap_err();
    assert!(err.to_string().contains("Unsupported scheme"));
}

#[derive(Debug)]
struct MockHttp404Handler;

impl ProtocolHandler for MockHttp404Handler {
    fn can_handle(&self, scheme: &str) -> bool {
        scheme == "mock"
    }

    fn fetch(&self, _url: &Url) -> Result<Response, NetError> {
        Ok(Response {
            status: 404,
            headers: std::collections::HashMap::new(),
            body: b"404 Not Found".to_vec(),
            mime_type: Some("text/html".to_string()),
        })
    }
}

#[test]
fn test_resource_loader_does_not_cache_error_responses() {
    let loader = ResourceLoader::with_handlers(vec![Box::new(MockHttp404Handler)]);
    let url = Url::parse("mock://example.com/missing").unwrap();

    let resp = loader.fetch(&url).unwrap();
    assert_eq!(resp.status, 404);

    // Cache should not contain this 404 URL
    assert!(loader.cache().get("mock://example.com/missing").is_none());
}
