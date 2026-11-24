//! Complete document layout pipeline with viewport initial containing block.

use crate::box_type::LayoutBox;
use crate::dimensions::Dimensions;
use crate::tree_builder::{build_layout_tree_with_context, LayoutContext};
use nirvana_core::geometry::{Rect, Size};
use nirvana_dom::DomTree;
use nirvana_style::compute::StyleTree;

/// Performs full document layout given a DOM tree, computed style tree, and viewport size.
/// Returns the root `LayoutBox` with fully calculated coordinates and dimensions.
pub fn layout_document(
    dom: &DomTree,
    style_tree: &StyleTree,
    viewport: Size,
) -> Option<LayoutBox> {
    layout_document_with_context(dom, style_tree, viewport, &LayoutContext::default())
}

/// Performs full document layout with an explicit `LayoutContext` (e.g. intrinsic image sizes).
pub fn layout_document_with_context(
    dom: &DomTree,
    style_tree: &StyleTree,
    viewport: Size,
    context: &LayoutContext,
) -> Option<LayoutBox> {
    let mut root_box = build_layout_tree_with_context(dom, style_tree, context)?;

    let initial_containing_block = Dimensions {
        content: Rect::new(0.0, 0.0, viewport.width, viewport.height),
        ..Default::default()
    };

    root_box.layout(&initial_containing_block, style_tree);
    Some(root_box)
}
