use nirvana_net::security::{is_mixed_content, CspDirective, CspPolicy, Origin};
use nirvana_url::Url;

#[test]
fn test_origin_parsing_and_equality() {
    let u1 = Url::parse("https://example.com/path").unwrap();
    let o1 = Origin::from_url(&u1);
    assert_eq!(o1.scheme(), "https");
    assert_eq!(o1.host(), Some("example.com"));
    assert_eq!(o1.port(), Some(443));
    assert_eq!(o1.as_string(), "https://example.com");

    let u2 = Url::parse("https://example.com:443/other").unwrap();
    let o2 = Origin::from_url(&u2);
    assert!(o1.is_same_origin(&o2));

    // Different port
    let u3 = Url::parse("https://example.com:8443/").unwrap();
    let o3 = Origin::from_url(&u3);
    assert!(!o1.is_same_origin(&o3));
    assert_eq!(o3.as_string(), "https://example.com:8443");

    // Different scheme
    let u4 = Url::parse("http://example.com/").unwrap();
    let o4 = Origin::from_url(&u4);
    assert!(!o1.is_same_origin(&o4));

    // Different host
    let u5 = Url::parse("https://sub.example.com/").unwrap();
    let o5 = Origin::from_url(&u5);
    assert!(!o1.is_same_origin(&o5));
}

#[test]
fn test_opaque_origins() {
    let u_data = Url::parse("data:text/html,hello").unwrap();
    let o_data1 = Origin::from_url(&u_data);
    let o_data2 = Origin::from_url(&u_data);

    assert!(o_data1.is_opaque());
    // Opaque origins are never same-origin
    assert!(!o_data1.is_same_origin(&o_data2));
}

#[test]
fn test_mixed_content_detection() {
    let doc_https = Origin::from_url(&Url::parse("https://bank.com/account").unwrap());
    let doc_http = Origin::from_url(&Url::parse("http://insecure.com/page").unwrap());

    let sub_http = Url::parse("http://cdn.com/app.js").unwrap();
    let sub_https = Url::parse("https://cdn.com/app.js").unwrap();
    let sub_data = Url::parse("data:image/png;base64,abc").unwrap();

    // Insecure subresource on HTTPS document is mixed content
    assert!(is_mixed_content(&doc_https, &sub_http));

    // Secure subresource on HTTPS document is NOT mixed content
    assert!(!is_mixed_content(&doc_https, &sub_https));

    // Data URI subresource is safe
    assert!(!is_mixed_content(&doc_https, &sub_data));

    // HTTP document is already insecure, so not classified as mixed content
    assert!(!is_mixed_content(&doc_http, &sub_http));
}

#[test]
fn test_csp_policy_parsing_and_evaluation() {
    let header = "default-src 'self'; script-src https://trusted.com; img-src * data:";
    let policy = CspPolicy::parse(header).expect("csp parsed");

    let origin = Origin::from_url(&Url::parse("https://example.com/").unwrap());

    // Script checks:
    let script_trusted = Url::parse("https://trusted.com/lib.js").unwrap();
    assert!(policy.is_allowed(CspDirective::ScriptSrc, &script_trusted, &origin));

    let script_self = Url::parse("https://example.com/main.js").unwrap();
    // script-src explicitly specified https://trusted.com, so 'self' is not allowed for scripts
    assert!(!policy.is_allowed(CspDirective::ScriptSrc, &script_self, &origin));

    // Image checks (img-src * data:):
    let img_any = Url::parse("https://random.org/avatar.png").unwrap();
    assert!(policy.is_allowed(CspDirective::ImgSrc, &img_any, &origin));

    // Style checks (falls back to default-src 'self'):
    let style_self = Url::parse("https://example.com/style.css").unwrap();
    assert!(policy.is_allowed(CspDirective::StyleSrc, &style_self, &origin));

    let style_other = Url::parse("https://attacker.com/evil.css").unwrap();
    assert!(!policy.is_allowed(CspDirective::StyleSrc, &style_other, &origin));
}
