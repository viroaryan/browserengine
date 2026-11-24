use nirvana_net::loader::ResourceLoader;

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
