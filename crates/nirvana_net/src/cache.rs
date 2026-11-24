//! In-memory resource caching mechanism.

use crate::response::Response;
use std::collections::HashMap;
use std::sync::RwLock;

/// Thread-safe in-memory cache for fetched web resources (documents, stylesheets, fonts).
#[derive(Debug)]
pub struct ResourceCache {
    entries: RwLock<HashMap<String, Response>>,
    max_entries: usize,
}

impl Default for ResourceCache {
    fn default() -> Self {
        Self::new(128)
    }
}

impl ResourceCache {
    /// Creates a new cache with specified maximum capacity.
    pub fn new(max_entries: usize) -> Self {
        Self {
            entries: RwLock::new(HashMap::new()),
            max_entries: max_entries.max(1),
        }
    }

    /// Retrieves a cached response by canonical URL string.
    pub fn get(&self, url: &str) -> Option<Response> {
        self.entries.read().ok()?.get(url).cloned()
    }

    /// Inserts a response into the cache. If capacity is exceeded, clears the cache.
    pub fn insert(&self, url: String, response: Response) {
        if let Ok(mut lock) = self.entries.write() {
            if lock.len() >= self.max_entries {
                lock.clear();
            }
            lock.insert(url, response);
        }
    }

    /// Clears all entries from the cache.
    pub fn clear(&self) {
        if let Ok(mut lock) = self.entries.write() {
            lock.clear();
        }
    }

    /// Returns the number of cached items.
    pub fn len(&self) -> usize {
        self.entries.read().map(|l| l.len()).unwrap_or(0)
    }

    /// Returns true if the cache contains no items.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}
