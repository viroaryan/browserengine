//! CSS Abstract Syntax Tree (AST) definitions and Specificity calculation.

use nirvana_core::color::Rgba;
use std::cmp::Ordering;

/// Simple selector components.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SimpleSelector {
    /// Matches any element (`*`).
    Universal,
    /// Matches element tag name (e.g. `div`, `h1`).
    Tag(String),
    /// Matches element class name (e.g. `.container`).
    Class(String),
    /// Matches element id (e.g. `#main`).
    Id(String),
}

/// CSS Selector Specificity tuple: (a, b, c)
/// - a: ID selectors count
/// - b: Class selectors count
/// - c: Type (tag) selectors count
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Specificity {
    pub ids: u32,
    pub classes: u32,
    pub tags: u32,
}

impl Specificity {
    /// Creates a new `Specificity` tuple.
    pub const fn new(ids: u32, classes: u32, tags: u32) -> Self {
        Self { ids, classes, tags }
    }
}

impl Ord for Specificity {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.ids, self.classes, self.tags).cmp(&(other.ids, other.classes, other.tags))
    }
}

impl PartialOrd for Specificity {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

/// A compound selector consisting of one or more simple selectors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub simple_selectors: Vec<SimpleSelector>,
}

impl Selector {
    /// Creates a new `Selector`.
    pub fn new(simple_selectors: Vec<SimpleSelector>) -> Self {
        Self { simple_selectors }
    }

    /// Calculates the specificity according to W3C Selectors Level 3.
    pub fn specificity(&self) -> Specificity {
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;

        for s in &self.simple_selectors {
            match s {
                SimpleSelector::Id(_) => ids += 1,
                SimpleSelector::Class(_) => classes += 1,
                SimpleSelector::Tag(_) => tags += 1,
                SimpleSelector::Universal => {}
            }
        }

        Specificity::new(ids, classes, tags)
    }
}

/// Resolved property value representation in the CSS AST.
#[derive(Debug, Clone, PartialEq)]
pub enum PropertyValue {
    /// Identifier or keyword (e.g. `block`, `inline`, `none`).
    Keyword(String),
    /// RGBA Color value.
    Color(Rgba),
    /// Explicit pixel dimension.
    Px(f32),
    /// Automatic sizing (`auto`).
    Auto,
}

/// A single CSS property declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: PropertyValue,
}

/// A CSS rule set consisting of a list of selectors and property declarations.
#[derive(Debug, Clone, PartialEq)]
pub struct RuleSet {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

impl RuleSet {
    /// Looks up the declared value for a given property name (case-insensitive, last declaration wins).
    pub fn get_property(&self, name: &str) -> Option<&PropertyValue> {
        self.declarations
            .iter()
            .rev()
            .find(|d| d.name.eq_ignore_ascii_case(name))
            .map(|d| &d.value)
    }
}

/// A complete parsed CSS stylesheet.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct StyleSheet {
    pub rules: Vec<RuleSet>,
}
