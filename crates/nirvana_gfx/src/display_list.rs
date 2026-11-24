//! Display list definitions and painting command abstractions.

use nirvana_core::color::Rgba;
use nirvana_core::geometry::{Point, Rect};
use nirvana_layout::dimensions::EdgeSizes;

/// Individual 2D drawing command in the display list.
#[derive(Debug, Clone, PartialEq)]
pub enum DisplayCommand {
    /// Fills a solid colored rectangle (e.g. background color).
    SolidColor {
        rect: Rect,
        color: Rgba,
    },
    /// Draws borders around a rectangle with specified edge widths and color.
    Border {
        rect: Rect,
        widths: EdgeSizes,
        color: Rgba,
    },
    /// Renders a text string at a given baseline origin with specified color.
    Text {
        text: String,
        origin: Point,
        color: Rgba,
    },
    /// Draws a decoded image scaled to fit inside the specified rectangle.
    DrawImage {
        rect: Rect,
        image: std::sync::Arc<crate::image::DecodedImage>,
    },
    /// Draws a text input caret cursor (vertical bar).
    Caret {
        rect: Rect,
        color: Rgba,
    },
    /// Draws a text selection highlight rectangle.
    TextSelection {
        rect: Rect,
        color: Rgba,
    },
}

/// An ordered sequence of drawing commands representing the visual appearance of a page.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DisplayList {
    pub commands: Vec<DisplayCommand>,
}

impl DisplayList {
    /// Creates an empty `DisplayList`.
    pub fn new() -> Self {
        Self {
            commands: Vec::new(),
        }
    }

    /// Appends a new command to the display list.
    pub fn push(&mut self, command: DisplayCommand) {
        self.commands.push(command);
    }

    /// Returns the number of commands in the display list.
    pub fn len(&self) -> usize {
        self.commands.len()
    }

    /// Returns `true` if the display list contains no commands.
    pub fn is_empty(&self) -> bool {
        self.commands.is_empty()
    }
}
