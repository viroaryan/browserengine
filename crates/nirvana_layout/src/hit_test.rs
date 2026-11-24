//! Layout tree hit-testing implementation in reverse visual painting order.

use crate::box_type::LayoutBox;
use nirvana_core::arena::Id;
use nirvana_core::geometry::Point;
use nirvana_dom::node::Node;

impl LayoutBox {
    /// Performs front-to-back hit-testing on this layout box and its descendants.
    ///
    /// Traverses child boxes in reverse document order (last child first) to match
    /// CSS visual stacking order, returning the topmost DOM node at the specified point.
    pub fn hit_test(&self, point: Point) -> Option<Id<Node>> {
        // 1. Check children in reverse painting order (topmost first)
        for child in self.children.iter().rev() {
            if let Some(hit) = child.hit_test(point) {
                return Some(hit);
            }
        }

        // 2. If no children hit, check if point falls within this box's border area
        if self.dimensions.border_box().contains_point(point) {
            return self.node_id();
        }

        None
    }
}
