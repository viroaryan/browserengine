//! Filter rule definitions for network requests and cosmetic element hiding.

/// Bitflags representing network resource types.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContentType(pub u32);

impl ContentType {
    pub const SCRIPT: u32 = 1 << 0;
    pub const IMAGE: u32 = 1 << 1;
    pub const STYLESHEET: u32 = 1 << 2;
    pub const SUBDOCUMENT: u32 = 1 << 3;
    pub const XMLHTTPREQUEST: u32 = 1 << 4;
    pub const MEDIA: u32 = 1 << 5;
    pub const FONT: u32 = 1 << 6;
    pub const OTHER: u32 = 1 << 7;
    pub const ALL: u32 = 0xFFFFFFFF;

    #[inline]
    pub fn matches(&self, mask: u32) -> bool {
        if mask == 0 || mask == Self::ALL {
            true
        } else {
            (self.0 & mask) != 0
        }
    }
}

/// Action to take on matching network request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterAction {
    Block,
    Allow,
}

/// Network filter rule parsed from ABP/uBlock syntax.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NetworkRule {
    pub raw: String,
    pub pattern: String,
    pub is_domain_anchor: bool, // starts with ||
    pub is_left_anchor: bool,   // starts with |
    pub is_right_anchor: bool,  // ends with |
    pub domains: Vec<String>,
    pub excluded_domains: Vec<String>,
    pub content_types: u32,
    pub third_party: Option<bool>,
    pub action: FilterAction,
}

/// Cosmetic filter rule for hiding DOM elements via injected CSS.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CosmeticRule {
    pub raw: String,
    pub domains: Vec<String>,
    pub excluded_domains: Vec<String>,
    pub selector: String,
}

/// Top-level filter rule enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterRule {
    Network(NetworkRule),
    Cosmetic(CosmeticRule),
}
