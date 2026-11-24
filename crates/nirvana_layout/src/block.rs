//! Normal flow block layout algorithm according to W3C CSS 2.1 §10.3 and §10.6.

use crate::box_type::LayoutBox;
use crate::dimensions::{Dimensions, EdgeSizes};
use nirvana_core::geometry::{Point, Rect};
use nirvana_style::compute::StyleTree;
use nirvana_style::values::{ComputedValues, Length};

impl LayoutBox {
    /// Computes the layout geometry for this box and its children against a containing block.
    pub fn layout(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        if self.is_block_level() {
            self.layout_block(containing_block, style_tree);
        } else {
            self.layout_inline(containing_block, style_tree);
        }
    }

    fn layout_block(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        let style = self
            .node_id()
            .and_then(|id| style_tree.get_style(id).cloned())
            .unwrap_or_default();

        // 1. Calculate width and horizontal margins/padding/borders
        self.calculate_block_width(containing_block, &style);

        // 2. Calculate vertical position (X, Y)
        self.calculate_block_position(containing_block, &style);

        // 3. Layout children and accumulate vertical height
        self.layout_block_children(style_tree);

        // 4. Calculate final height
        self.calculate_block_height(&style);
    }

    fn calculate_block_width(&mut self, containing_block: &Dimensions, style: &ComputedValues) {
        let p_top = length_to_px(&style.padding_top);
        let p_right = length_to_px(&style.padding_right);
        let p_bottom = length_to_px(&style.padding_bottom);
        let p_left = length_to_px(&style.padding_left);

        self.dimensions.padding = EdgeSizes::new(p_top, p_right, p_bottom, p_left);

        let b = style.border_width;
        self.dimensions.border = EdgeSizes::new(b, b, b, b);

        let m_top = length_to_px(&style.margin_top);
        let m_bottom = length_to_px(&style.margin_bottom);

        let container_width = containing_block.content.width();

        match style.width {
            Length::Px(w) => {
                let total_fixed = w
                    + self.dimensions.padding.left
                    + self.dimensions.padding.right
                    + self.dimensions.border.left
                    + self.dimensions.border.right;

                let underflow = container_width - total_fixed;

                match (style.margin_left, style.margin_right) {
                    (Length::Auto, Length::Auto) => {
                        let half = (underflow / 2.0).max(0.0);
                        self.dimensions.margin = EdgeSizes::new(m_top, half, m_bottom, half);
                    }
                    (Length::Auto, Length::Px(mr)) => {
                        let ml = underflow - mr;
                        self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);
                    }
                    (Length::Px(ml), Length::Auto) => {
                        let mr = underflow - ml;
                        self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);
                    }
                    (Length::Px(ml), Length::Px(_mr)) => {
                        // Over-constrained: margin-right absorbs underflow in LTR
                        let mr_adjusted = underflow - ml;
                        self.dimensions.margin =
                            EdgeSizes::new(m_top, mr_adjusted, m_bottom, ml);
                    }
                }

                self.dimensions.content.size.width = w.max(0.0);
            }
            Length::Auto => {
                let ml = match style.margin_left {
                    Length::Px(val) => val,
                    Length::Auto => 0.0,
                };
                let mr = match style.margin_right {
                    Length::Px(val) => val,
                    Length::Auto => 0.0,
                };

                self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);

                let w = if let Some(intrinsic) = self.intrinsic_size {
                    if let Length::Px(h) = style.height {
                        if intrinsic.height > 0.0 {
                            h * (intrinsic.width / intrinsic.height)
                        } else {
                            intrinsic.width
                        }
                    } else {
                        intrinsic.width
                    }
                } else {
                    let total_non_content = self.dimensions.padding.left
                        + self.dimensions.padding.right
                        + self.dimensions.border.left
                        + self.dimensions.border.right
                        + self.dimensions.margin.left
                        + self.dimensions.margin.right;

                    (container_width - total_non_content).max(0.0)
                };

                self.dimensions.content.size.width = w.max(0.0);
            }
        }
    }

    fn calculate_block_position(&mut self, containing_block: &Dimensions, _style: &ComputedValues) {
        let x = containing_block.content.x()
            + self.dimensions.margin.left
            + self.dimensions.border.left
            + self.dimensions.padding.left;

        let y = containing_block.content.y()
            + self.dimensions.margin.top
            + self.dimensions.border.top
            + self.dimensions.padding.top;

        self.dimensions.content.origin = Point::new(x, y);
    }

    fn layout_block_children(&mut self, style_tree: &StyleTree) {
        let mut cursor_y = self.dimensions.content.y();

        for child in &mut self.children {
            // Setup child's containing block
            let child_cb = Dimensions {
                content: Rect::new(
                    self.dimensions.content.x(),
                    cursor_y,
                    self.dimensions.content.width(),
                    0.0,
                ),
                ..Default::default()
            };

            child.layout(&child_cb, style_tree);

            // Advance vertical cursor by child's margin box height
            cursor_y += child.dimensions.margin_box().height();
        }
    }

    fn calculate_block_height(&mut self, style: &ComputedValues) {
        match style.height {
            Length::Px(h) => {
                self.dimensions.content.size.height = h.max(0.0);
            }
            Length::Auto => {
                if let Some(intrinsic) = self.intrinsic_size {
                    if let Length::Px(w) = style.width {
                        if intrinsic.width > 0.0 {
                            self.dimensions.content.size.height =
                                (w * (intrinsic.height / intrinsic.width)).max(0.0);
                        } else {
                            self.dimensions.content.size.height = intrinsic.height;
                        }
                    } else if self.dimensions.content.size.width > 0.0 && intrinsic.width > 0.0 {
                        self.dimensions.content.size.height = (self.dimensions.content.size.width
                            * (intrinsic.height / intrinsic.width))
                            .max(0.0);
                    } else {
                        self.dimensions.content.size.height = intrinsic.height;
                    }
                } else {
                    // Sum of children margin box heights
                    let total_children_height: f32 = self
                        .children
                        .iter()
                        .map(|c| c.dimensions.margin_box().height())
                        .sum();
                    self.dimensions.content.size.height = total_children_height;
                }
            }
        }
    }

    fn layout_inline(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        let style = self
            .node_id()
            .and_then(|id| style_tree.get_style(id).cloned())
            .unwrap_or_default();

        let p_top = length_to_px(&style.padding_top);
        let p_right = length_to_px(&style.padding_right);
        let p_bottom = length_to_px(&style.padding_bottom);
        let p_left = length_to_px(&style.padding_left);
        self.dimensions.padding = EdgeSizes::new(p_top, p_right, p_bottom, p_left);

        let b = style.border_width;
        self.dimensions.border = EdgeSizes::new(b, b, b, b);

        let m_top = length_to_px(&style.margin_top);
        let m_right = length_to_px(&style.margin_right);
        let m_bottom = length_to_px(&style.margin_bottom);
        let m_left = length_to_px(&style.margin_left);
        self.dimensions.margin = EdgeSizes::new(m_top, m_right, m_bottom, m_left);

        self.dimensions.content.origin = containing_block.content.origin;

        if let Some(intrinsic) = self.intrinsic_size {
            let width = match style.width {
                Length::Px(w) => w,
                Length::Auto => {
                    if let Length::Px(h) = style.height {
                        if intrinsic.height > 0.0 {
                            h * (intrinsic.width / intrinsic.height)
                        } else {
                            intrinsic.width
                        }
                    } else {
                        intrinsic.width
                    }
                }
            };

            let height = match style.height {
                Length::Px(h) => h,
                Length::Auto => {
                    if let Length::Px(w) = style.width {
                        if intrinsic.width > 0.0 {
                            w * (intrinsic.height / intrinsic.width)
                        } else {
                            intrinsic.height
                        }
                    } else {
                        intrinsic.height
                    }
                }
            };

            self.dimensions.content.size.width = width.max(0.0);
            self.dimensions.content.size.height = height.max(0.0);
            return;
        }

        for child in &mut self.children {
            child.layout(containing_block, style_tree);
        }

        if !self.children.is_empty() {
            let mut max_w: f32 = 0.0;
            let mut max_h: f32 = 0.0;
            for child in &self.children {
                let mb = child.dimensions.margin_box();
                max_w = max_w.max(mb.right() - self.dimensions.content.origin.x);
                max_h = max_h.max(mb.bottom() - self.dimensions.content.origin.y);
            }
            self.dimensions.content.size.width = self.dimensions.content.size.width.max(max_w);
            self.dimensions.content.size.height = self.dimensions.content.size.height.max(max_h);
        }
    }
}

fn length_to_px(len: &Length) -> f32 {
    match len {
        Length::Px(p) => *p,
        Length::Auto => 0.0,
    }
}
