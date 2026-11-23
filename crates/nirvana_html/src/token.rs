//! HTML token types conforming to WHATWG HTML tokenization.

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagData {
    pub name: String,
    pub self_closing: bool,
    pub attributes: Vec<(String, String)>,
}

impl TagData {
    pub fn new(name: &str, self_closing: bool) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            self_closing,
            attributes: Vec::new(),
        }
    }

    pub fn add_attribute(&mut self, name: &str, value: &str) {
        let name_lower = name.to_ascii_lowercase();
        // WHATWG §13.2.5.33: When an attribute is already present, leave first value unchanged
        if !self
            .attributes
            .iter()
            .any(|(k, _)| k.to_ascii_lowercase() == name_lower)
        {
            self.attributes.push((name_lower, value.to_string()));
        }
    }

    pub fn get_attribute(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(name))
            .map(|(_, v)| v.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DoctypeData {
    pub name: String,
    pub public_identifier: Option<String>,
    pub system_identifier: Option<String>,
    pub force_quirks: bool,
}

impl DoctypeData {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_ascii_lowercase(),
            public_identifier: None,
            system_identifier: None,
            force_quirks: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Doctype(DoctypeData),
    StartTag(TagData),
    EndTag(TagData),
    Comment(String),
    Character(char),
    Eof,
}
