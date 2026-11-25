//! CSS Abstract Syntax Tree (AST) definitions and Specificity calculation.

use nirvana_core::color::Rgba;
use std::cmp::Ordering;

/// Attribute comparison operators for CSS `[attr...]` selectors.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AttrOp {
    Exists,
    Exact,
    Includes,
    DashMatch,
    Prefix,
    Suffix,
    Contains,
}

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
    /// Matches an element attribute (e.g. `[type="hidden"]`).
    Attribute {
        name: String,
        op: AttrOp,
        value: Option<String>,
    },
    /// Matches a CSS pseudo-class or pseudo-element (e.g. `:hover`, `:first-child`, `::after`).
    PseudoClass(String),
}

/// Combinator linking a selector segment to the next segment.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Combinator {
    Descendant,
    Child,
    NextSibling,
    SubsequentSibling,
}

/// An ancestor or sibling compound selector constraint evaluated right-to-left.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SelectorSegment {
    pub combinator: Combinator,
    pub simple_selectors: Vec<SimpleSelector>,
}

/// CSS Selector Specificity tuple: (a, b, c)
/// - a: ID selectors count
/// - b: Class, attribute, and pseudo-class selectors count
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

/// A CSS complex selector consisting of a subject compound selector (`simple_selectors`)
/// and optional right-to-left combinator chain (`ancestors`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Selector {
    pub simple_selectors: Vec<SimpleSelector>,
    pub ancestors: Vec<SelectorSegment>,
}

impl Selector {
    /// Creates a new simple/compound `Selector` with no ancestor combinators.
    pub fn new(simple_selectors: Vec<SimpleSelector>) -> Self {
        Self {
            simple_selectors,
            ancestors: Vec::new(),
        }
    }

    /// Creates a complex `Selector` with right-to-left combinator segments.
    pub fn with_ancestors(
        simple_selectors: Vec<SimpleSelector>,
        ancestors: Vec<SelectorSegment>,
    ) -> Self {
        Self {
            simple_selectors,
            ancestors,
        }
    }

    /// Calculates specificity according to W3C Selectors Level 3 across all segments.
    pub fn specificity(&self) -> Specificity {
        let mut ids = 0;
        let mut classes = 0;
        let mut tags = 0;

        let mut count_simples = |simples: &[SimpleSelector]| {
            for s in simples {
                match s {
                    SimpleSelector::Id(_) => ids += 1,
                    SimpleSelector::Class(_)
                    | SimpleSelector::Attribute { .. }
                    | SimpleSelector::PseudoClass(_) => classes += 1,
                    SimpleSelector::Tag(_) => tags += 1,
                    SimpleSelector::Universal => {}
                }
            }
        };

        count_simples(&self.simple_selectors);
        for seg in &self.ancestors {
            count_simples(&seg.simple_selectors);
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
    /// Unitless numeric value (e.g. `font-weight: 700`, `line-height: 1.5`).
    Number(f32),
    /// Explicit pixel dimension.
    Px(f32),
    /// Relative percentage dimension.
    Percent(f32),
    /// Relative `em` font-size dimension.
    Em(f32),
    /// Relative `rem` root font-size dimension.
    Rem(f32),
    /// Typographic point dimension (`1pt = 4/3 px`).
    Pt(f32),
    /// Viewport width percentage (`vw`).
    Vw(f32),
    /// Viewport height percentage (`vh`).
    Vh(f32),
    /// Automatic sizing (`auto`).
    Auto,
}

/// A single CSS property declaration.
#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: PropertyValue,
    pub important: bool,
}

impl Declaration {
    /// Creates a normal (non-important) CSS property declaration.
    pub fn new(name: impl Into<String>, value: PropertyValue) -> Self {
        Self {
            name: name.into(),
            value,
            important: false,
        }
    }

    /// Creates a declaration with explicit `!important` flag.
    pub fn with_importance(
        name: impl Into<String>,
        value: PropertyValue,
        important: bool,
    ) -> Self {
        Self {
            name: name.into(),
            value,
            important,
        }
    }
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
