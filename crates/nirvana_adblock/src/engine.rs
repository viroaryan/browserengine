//! Unified AdBlock engine orchestrating network filtering, cosmetic element hiding, and telemetry.

use crate::cosmetic::generate_cosmetic_stylesheet;
use crate::filter::{ContentType, CosmeticRule, FilterAction, FilterRule, NetworkRule};
use crate::matcher::rule_matches;
use crate::parser::{parse_filter_list, parse_line};
use crate::rulesets::BUILTIN_RULESET;
use crate::scriptlets::generate_defuser_script;
use nirvana_url::Url;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug)]
pub struct AdBlockEngine {
    network_rules: Vec<NetworkRule>,
    whitelist_rules: Vec<NetworkRule>,
    cosmetic_rules: Vec<CosmeticRule>,
    blocked_count: AtomicU64,
    enabled: bool,
}

impl AdBlockEngine {
    /// Creates a new `AdBlockEngine` pre-loaded with the sovereign built-in ruleset.
    pub fn new() -> Self {
        let mut engine = Self::empty();
        engine.load_rules(BUILTIN_RULESET);
        engine
    }

    /// Creates an empty `AdBlockEngine` with no pre-loaded rules.
    pub fn empty() -> Self {
        Self {
            network_rules: Vec::new(),
            whitelist_rules: Vec::new(),
            cosmetic_rules: Vec::new(),
            blocked_count: AtomicU64::new(0),
            enabled: true,
        }
    }

    /// Loads and parses rules from multi-line text (e.g. EasyList / uBlock filter lists).
    pub fn load_rules(&mut self, content: &str) {
        let rules = parse_filter_list(content);
        for rule in rules {
            self.add_filter_rule(rule);
        }
    }

    /// Parses and adds a single filter rule string.
    pub fn add_rule(&mut self, line: &str) {
        if let Some(rule) = parse_line(line) {
            self.add_filter_rule(rule);
        }
    }

    fn add_filter_rule(&mut self, rule: FilterRule) {
        match rule {
            FilterRule::Network(nr) => {
                if nr.action == FilterAction::Allow {
                    self.whitelist_rules.push(nr);
                } else {
                    self.network_rules.push(nr);
                }
            }
            FilterRule::Cosmetic(cr) => {
                self.cosmetic_rules.push(cr);
            }
        }
    }

    /// Evaluates whether an outgoing resource request should be blocked.
    /// If blocked, increments the internal `blocked_count` counter.
    pub fn should_block(
        &self,
        request_url: &Url,
        doc_origin: Option<&str>,
        content_type: ContentType,
    ) -> bool {
        if !self.enabled {
            return false;
        }

        // 1. Whitelist rules take precedence
        for rule in &self.whitelist_rules {
            if rule_matches(rule, request_url, doc_origin, content_type) {
                return false;
            }
        }

        // 2. Block rules check
        for rule in &self.network_rules {
            if rule_matches(rule, request_url, doc_origin, content_type) {
                self.blocked_count.fetch_add(1, Ordering::Relaxed);
                return true;
            }
        }

        false
    }

    /// Returns generated cosmetic stylesheet hiding ad elements on the given document URL.
    pub fn cosmetic_stylesheet(&self, document_url: Option<&str>) -> Option<String> {
        if !self.enabled {
            return None;
        }
        generate_cosmetic_stylesheet(&self.cosmetic_rules, document_url)
    }

    /// Returns the standard scriptlet defuser code injected at document_start.
    #[inline]
    pub fn defuser_script(&self) -> &'static str {
        generate_defuser_script()
    }

    /// Returns the total number of blocked ad and tracker requests.
    #[inline]
    pub fn blocked_count(&self) -> u64 {
        self.blocked_count.load(Ordering::Relaxed)
    }

    /// Resets the blocked requests counter to zero.
    #[inline]
    pub fn reset_blocked_count(&self) {
        self.blocked_count.store(0, Ordering::Relaxed);
    }

    /// Adds blocked request delta recorded during off-thread prefetching.
    #[inline]
    pub fn add_blocked_count(&self, delta: u64) {
        if delta > 0 {
            self.blocked_count.fetch_add(delta, Ordering::Relaxed);
        }
    }

    /// Enables or disables ad-blocking globally.
    #[inline]
    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    /// Returns true if ad-blocking is currently enabled.
    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Returns the total number of active rules.
    pub fn rule_count(&self) -> (usize, usize, usize) {
        (
            self.network_rules.len(),
            self.whitelist_rules.len(),
            self.cosmetic_rules.len(),
        )
    }
}

impl Default for AdBlockEngine {
    fn default() -> Self {
        Self::new()
    }
}
