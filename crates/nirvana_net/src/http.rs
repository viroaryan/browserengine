//! HTTP/HTTPS protocol handler using ureq with IPv4 fallback and subresource deadlines.

use crate::error::NetError;
use crate::handler::ProtocolHandler;
use crate::response::Response;
use nirvana_url::Url;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use ureq::config::IpFamily;
use ureq::ResponseExt;

const HTTP_TIMEOUT_SECS: u64 = 15;
const SUBRESOURCE_TIMEOUT_SECS: u64 = 5;

const DEFAULT_MOBILE_UA: &str = "Mozilla/5.0 (Android 14; Mobile; rv:132.0) Gecko/132.0 Firefox/132.0";

/// Protocol handler for `http://` and `https://` URLs.
#[derive(Debug, Clone)]
pub struct HttpHandler {
    agent: Arc<ureq::Agent>,
    ipv4_agent: Arc<ureq::Agent>,
    fast_agent: Arc<ureq::Agent>,
    fast_ipv4_agent: Arc<ureq::Agent>,
}

fn build_tls_config() -> ureq::tls::TlsConfig {
    #[cfg(windows)]
    return ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();

    #[cfg(not(windows))]
    return ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::Rustls)
        .root_certs(ureq::tls::RootCerts::WebPki)
        .build();
}

fn build_agent(
    ip_family: IpFamily,
    connect_secs: u64,
    recv_resp_secs: u64,
    global_secs: u64,
) -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .tls_config(build_tls_config())
        .ip_family(ip_family)
        .timeout_resolve(Some(Duration::from_secs(3)))
        .timeout_connect(Some(Duration::from_secs(connect_secs)))
        .timeout_recv_response(Some(Duration::from_secs(recv_resp_secs)))
        .timeout_recv_body(Some(Duration::from_secs(global_secs)))
        .timeout_global(Some(Duration::from_secs(global_secs)))
        .http_status_as_error(false)
        .max_redirects(10)
        .build();
    ureq::Agent::new_with_config(config)
}

impl Default for HttpHandler {
    fn default() -> Self {
        Self {
            agent: Arc::new(build_agent(IpFamily::Any, 4, 8, HTTP_TIMEOUT_SECS)),
            ipv4_agent: Arc::new(build_agent(IpFamily::Ipv4Only, 4, 8, 12)),
            fast_agent: Arc::new(build_agent(IpFamily::Any, 3, 4, SUBRESOURCE_TIMEOUT_SECS)),
            fast_ipv4_agent: Arc::new(build_agent(IpFamily::Ipv4Only, 3, 4, SUBRESOURCE_TIMEOUT_SECS)),
        }
    }
}

impl HttpHandler {
    fn execute_with_agent(
        agent: &ureq::Agent,
        url_str: &str,
        req_headers: &[(&str, &str)],
        body_limit: usize,
    ) -> Result<Response, ureq::Error> {
        let has_custom_ua = req_headers
            .iter()
            .any(|(name, _)| name.eq_ignore_ascii_case("user-agent"));

        let mut req = agent.get(url_str);
        if !has_custom_ua {
            req = req.header("User-Agent", DEFAULT_MOBILE_UA);
        }
        req = req
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/png,*/*;q=0.8",
            )
            .header("Accept-Language", "en-US,en;q=0.9")
            .header("Upgrade-Insecure-Requests", "1");

        for (name, val) in req_headers {
            req = req.header(*name, *val);
        }

        let resp = req.call()?;
        let status = resp.status().as_u16();
        let final_url = Some(resp.get_uri().to_string());

        let mut headers = HashMap::new();
        let mut raw_headers = Vec::new();
        let mut mime_type = None;

        for (name, val) in resp.headers() {
            let key = name.as_str().to_ascii_lowercase();
            if let Ok(val_str) = val.to_str() {
                if key == "content-type" {
                    let mime = val_str.split(';').next().unwrap_or(val_str).trim();
                    mime_type = Some(mime.to_string());
                }
                headers.insert(key.clone(), val_str.to_string());
                raw_headers.push((key, val_str.to_string()));
            }
        }

        let body = resp
            .into_body()
            .with_config()
            .limit(body_limit as u64)
            .read_to_vec()?;

        Ok(Response {
            status,
            headers,
            raw_headers,
            body,
            mime_type,
            final_url,
        })
    }

    fn fetch_internal(
        &self,
        url: &Url,
        req_headers: &[(&str, &str)],
        fast: bool,
    ) -> Result<Response, NetError> {
        let url_str = url.as_str();
        let (primary, fallback_v4, limit) = if fast {
            (&self.fast_agent, &self.fast_ipv4_agent, 8 * 1024 * 1024)
        } else {
            (&self.agent, &self.ipv4_agent, 25 * 1024 * 1024)
        };

        match Self::execute_with_agent(primary, &url_str, req_headers, limit) {
            Ok(resp) => Ok(resp),
            Err(err) if should_retry_ipv4(&err) => {
                Self::execute_with_agent(fallback_v4, &url_str, req_headers, limit)
                    .map_err(|e2| NetError::NetworkFailure(e2.to_string()))
            }
            Err(err) => Err(NetError::NetworkFailure(err.to_string())),
        }
    }
}

fn should_retry_ipv4(err: &ureq::Error) -> bool {
    matches!(
        err,
        ureq::Error::Timeout(_) | ureq::Error::Io(_) | ureq::Error::HostNotFound
    )
}

impl ProtocolHandler for HttpHandler {
    fn can_handle(&self, scheme: &str) -> bool {
        scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https")
    }

    fn fetch(&self, url: &Url) -> Result<Response, NetError> {
        self.fetch_internal(url, &[], false)
    }

    fn fetch_with_headers(&self, url: &Url, req_headers: &[(&str, &str)]) -> Result<Response, NetError> {
        self.fetch_internal(url, req_headers, false)
    }

    fn fetch_fast_with_headers(&self, url: &Url, req_headers: &[(&str, &str)]) -> Result<Response, NetError> {
        self.fetch_internal(url, req_headers, true)
    }
}
