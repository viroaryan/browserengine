//! ResourceLoader orchestrating protocol handlers and in-memory cache.

use crate::cache::ResourceCache;
use crate::error::NetError;
use crate::handler::{DataUriHandler, FileHandler, ProtocolHandler};
use crate::http::HttpHandler;
use crate::response::Response;
use nirvana_url::Url;
use std::sync::Arc;

/// Central coordinator for loading web resources across all supported protocols.
#[derive(Debug)]
pub struct ResourceLoader {
    handlers: Vec<Box<dyn ProtocolHandler>>,
    cache: Arc<ResourceCache>,
}

impl Default for ResourceLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ResourceLoader {
    /// Creates a new ResourceLoader pre-configured with HTTP, File, and Data URI handlers.
    pub fn new() -> Self {
        let handlers: Vec<Box<dyn ProtocolHandler>> = vec![
            Box::new(HttpHandler::default()),
            Box::new(FileHandler),
            Box::new(DataUriHandler),
        ];

        Self {
            handlers,
            cache: Arc::new(ResourceCache::default()),
        }
    }

    /// Creates a ResourceLoader with custom protocol handlers.
    pub fn with_handlers(handlers: Vec<Box<dyn ProtocolHandler>>) -> Self {
        Self {
            handlers,
            cache: Arc::new(ResourceCache::default()),
        }
    }

    /// Access the internal resource cache.
    pub fn cache(&self) -> &Arc<ResourceCache> {
        &self.cache
    }

    /// Fetches a resource from the given parsed `Url`.
    /// Checks in-memory cache first, then dispatches to the matching protocol handler.
    pub fn fetch(&self, url: &Url) -> Result<Response, NetError> {
        let url_str = url.as_str();

        // 1. Check cache
        if let Some(cached) = self.cache.get(&url_str) {
            return Ok(cached);
        }

        // 2. Find matching handler
        let handler = self
            .handlers
            .iter()
            .find(|h| h.can_handle(url.scheme()))
            .ok_or_else(|| NetError::UnsupportedScheme(url.scheme().to_string()))?;

        // 3. Fetch resource
        let response = handler.fetch(url)?;

        // 4. Store in cache on success (2xx HTTP / OK status)
        if response.status >= 200 && response.status < 300 {
            self.cache.insert(url_str, response.clone());
        }

        Ok(response)
    }

    /// Helper to parse a URL string and fetch the resource.
    pub fn fetch_string(&self, url_str: &str) -> Result<Response, NetError> {
        let url = Url::parse(url_str)?;
        self.fetch(&url)
    }

    /// Clears the internal resource cache.
    pub fn clear_cache(&self) {
        self.cache.clear();
    }
}
