//! Normal flow block, inline line-box flow, flexbox, and table layout engine.

use crate::box_type::{LayoutBox, TextRun};
use crate::dimensions::{Dimensions, EdgeSizes};
use crate::tree_builder::split_overlong_word;
use nirvana_core::geometry::{Point, Rect};
use nirvana_style::compute::StyleTree;
use nirvana_style::values::{ComputedValues, Display, Length, TextAlign};

impl LayoutBox {
    /// Computes layout geometry for this box and its descendants inside a containing block.
    pub fn layout(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        if self.is_block_level() {
            self.layout_block(containing_block, style_tree);
        } else {
            self.layout_inline_standalone(containing_block, style_tree);
        }
    }

    fn layout_block(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        let default_style = ComputedValues::default();
        let style = self
            .node_id()
            .and_then(|id| style_tree.get_style(id))
            .unwrap_or(&default_style);

        // 1. Resolve width, margins, padding, borders
        self.calculate_block_width(containing_block, style);

        // 2. Resolve top-left content origin
        self.calculate_block_position(containing_block);

        // 3. Layout descendants based on display mode (Table, TableRow, Flex, Block, Inline Flow)
        match self.display_mode {
            Display::Table => self.layout_table(style_tree),
            Display::TableRow => self.layout_standalone_table_row(style_tree),
            Display::Flex => self.layout_flex(style, style_tree),
            _ => self.layout_block_or_inline_children(style, style_tree),
        }

        // 4. Final height resolution
        self.calculate_block_height(style);
    }

    fn apply_box_edges(&mut self, style: &ComputedValues, container_w: f32) {
        self.dimensions.padding = EdgeSizes::new(
            resolve_px_nonneg(&style.padding_top, container_w),
            resolve_px_nonneg(&style.padding_right, container_w),
            resolve_px_nonneg(&style.padding_bottom, container_w),
            resolve_px_nonneg(&style.padding_left, container_w),
        );
        self.dimensions.border = EdgeSizes::new(
            style.border_top_width,
            style.border_right_width,
            style.border_bottom_width,
            style.border_left_width,
        );
    }

    fn calculate_block_width(&mut self, containing_block: &Dimensions, style: &ComputedValues) {
        let container_width = containing_block.content.width();
        self.apply_box_edges(style, container_width);

        let m_top = length_to_px_or_pct(&style.margin_top, container_width);
        let m_bottom = length_to_px_or_pct(&style.margin_bottom, container_width);

        let min_w = resolve_optional_length(&style.min_width, container_width).unwrap_or(0.0);
        let max_w = resolve_optional_length(&style.max_width, container_width).unwrap_or(f32::INFINITY);

        let resolved_w = resolve_optional_length(&style.width, container_width);

        match resolved_w {
            Some(mut w) => {
                w = css_clamp_size(w, min_w, max_w);
                self.solve_horizontal_margins(w, m_top, m_bottom, container_width, style);
                self.dimensions.content.size.width = w;
            }
            None => {
                if let Some(intrinsic) = self.intrinsic_size {
                    let mut w = if let Some(h) = resolve_optional_length(&style.height, containing_block.content.height()) {
                        if intrinsic.height > 0.0 {
                            h * (intrinsic.width / intrinsic.height)
                        } else {
                            intrinsic.width
                        }
                    } else {
                        intrinsic.width
                    };
                    w = css_clamp_size(w, min_w, max_w);
                    self.solve_horizontal_margins(w, m_top, m_bottom, container_width, style);
                    self.dimensions.content.size.width = w;
                } else {
                    let ml = length_to_px_or_pct(&style.margin_left, container_width);
                    let mr = length_to_px_or_pct(&style.margin_right, container_width);
                    self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);

                    let total_non_content = self.dimensions.padding.left
                        + self.dimensions.padding.right
                        + self.dimensions.border.left
                        + self.dimensions.border.right
                        + ml
                        + mr;

                    let auto_w = css_clamp_size(container_width - total_non_content, min_w, max_w);
                    // Support margin: auto centering when max-width shrinks an auto-width block
                    if auto_w < (container_width - total_non_content).max(0.0)
                        && matches!(style.margin_left, Length::Auto)
                        && matches!(style.margin_right, Length::Auto)
                    {
                        self.solve_horizontal_margins(auto_w, m_top, m_bottom, container_width, style);
                    }
                    self.dimensions.content.size.width = auto_w;
                }
            }
        }
    }

    fn solve_horizontal_margins(
        &mut self,
        w: f32,
        m_top: f32,
        m_bottom: f32,
        container_width: f32,
        style: &ComputedValues,
    ) {
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
            (Length::Auto, mr_len) => {
                let mr = length_to_px_or_pct(&mr_len, container_width);
                let ml = underflow - mr;
                self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);
            }
            (ml_len, Length::Auto) => {
                let ml = length_to_px_or_pct(&ml_len, container_width);
                let mr = underflow - ml;
                self.dimensions.margin = EdgeSizes::new(m_top, mr, m_bottom, ml);
            }
            (ml_len, _mr_len) => {
                let ml = length_to_px_or_pct(&ml_len, container_width);
                let mr_adjusted = underflow - ml;
                self.dimensions.margin = EdgeSizes::new(m_top, mr_adjusted, m_bottom, ml);
            }
        }
    }

    fn calculate_block_position(&mut self, containing_block: &Dimensions) {
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

    fn layout_block_or_inline_children(&mut self, style: &ComputedValues, style_tree: &StyleTree) {
        if self.children.is_empty() {
            return;
        }

        let all_inline = self.children.iter().all(|c| c.is_inline_level());
        if all_inline {
            let max_w = self.dimensions.content.width().max(40.0);
            let origin_x = self.dimensions.content.x();
            let start_y = self.dimensions.content.y();
            let mut cursor = InlineFlowCursor::new(origin_x, start_y, max_w, style.text_align);

            let mut children = std::mem::take(&mut self.children);
            for child in &mut children {
                flow_inline_box(child, &mut cursor, style, style_tree);
            }
            cursor.finish_line();
            align_inline_children(&mut children, origin_x, max_w, style.text_align);
            self.children = children;
            return;
        }

        let mut cursor_y = self.dimensions.content.y();
        for child in &mut self.children {
            if child.is_out_of_flow() {
                let child_cb = Dimensions {
                    content: Rect::new(
                        self.dimensions.content.x(),
                        self.dimensions.content.y(),
                        self.dimensions.content.width(),
                        0.0,
                    ),
                    ..Default::default()
                };
                child.layout(&child_cb, style_tree);
                continue;
            }
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
            cursor_y += child.dimensions.margin_box().height();
        }
    }

    fn layout_flex(&mut self, style: &ComputedValues, style_tree: &StyleTree) {
        if self.children.is_empty() {
            return;
        }
        let container_w = self.dimensions.content.width().max(40.0);
        let origin_x = self.dimensions.content.x();
        let origin_y = self.dimensions.content.y();
        let gap = style.gap;

        if style.flex_direction_column || (style.is_grid && style.grid_cols <= 1) {
            let mut cursor_y = origin_y;
            let mut in_flow_seen = 0usize;
            for child in &mut self.children {
                if child.is_out_of_flow() {
                    let child_cb = Dimensions {
                        content: Rect::new(origin_x, origin_y, container_w, 0.0),
                        ..Default::default()
                    };
                    if child.is_inline_level() {
                        child.layout_block(&child_cb, style_tree);
                    } else {
                        child.layout(&child_cb, style_tree);
                    }
                    continue;
                }
                if in_flow_seen > 0 {
                    cursor_y += gap;
                }
                let child_cb = Dimensions {
                    content: Rect::new(origin_x, cursor_y, container_w, 0.0),
                    ..Default::default()
                };
                if child.is_inline_level() {
                    child.layout_block(&child_cb, style_tree);
                } else {
                    child.layout(&child_cb, style_tree);
                }
                cursor_y += child.dimensions.margin_box().height();
                in_flow_seen += 1;
            }
            return;
        }

        // Row flex / multi-column CSS grid layout: distribute width and wrap when overflowing mobile viewport
        let in_flow_count = self.children.iter().filter(|c| !c.is_out_of_flow()).count().max(1);
        let effective_cols = if style.is_grid {
            style.grid_cols.max(2).min(in_flow_count)
        } else {
            in_flow_count
        };
        let total_gaps = gap * (effective_cols.saturating_sub(1) as f32);
        let raw_item_w = (container_w - total_gaps).max(20.0) / effective_cols as f32;
        let force_wrap = style.flex_wrap || style.is_grid || (raw_item_w < 56.0 && in_flow_count > 3);
        let default_item_w = if force_wrap && raw_item_w < 88.0 && !style.is_grid {
            (container_w / 2.0 - gap * 0.5).max(88.0).min(container_w)
        } else {
            raw_item_w.max(24.0)
        };

        let mut cursor_x = origin_x;
        let mut cursor_y = origin_y;
        let mut row_h: f32 = 0.0;
        let mut row_items = 0usize;

        for child in &mut self.children {
            if child.is_out_of_flow() {
                let item_cb = Dimensions {
                    content: Rect::new(origin_x, origin_y, container_w, 0.0),
                    ..Default::default()
                };
                child.layout_block(&item_cb, style_tree);
                continue;
            }

            let child_style = child
                .node_id()
                .and_then(|id| style_tree.get_style(id).cloned())
                .unwrap_or_default();

            let item_avail_w = css_clamp_size(
                resolve_optional_length(&child_style.width, container_w).unwrap_or_else(|| {
                    if let Some(intr) = child.intrinsic_size {
                        intr.width
                    } else {
                        default_item_w
                    }
                }),
                20.0,
                container_w.max(20.0),
            );

            let wraps_here = row_items > 0
                && cursor_x > origin_x
                && ((force_wrap && cursor_x + item_avail_w > origin_x + container_w + 1.0)
                    || item_avail_w >= container_w * 0.6);
            if wraps_here {
                cursor_x = origin_x;
                cursor_y += row_h + gap;
                row_h = 0.0;
                row_items = 0;
            }

            let item_cb = Dimensions {
                content: Rect::new(cursor_x, cursor_y, item_avail_w, 0.0),
                ..Default::default()
            };
            child.layout_block(&item_cb, style_tree);

            let mb = child.dimensions.margin_box();
            cursor_x += mb.width() + gap;
            row_h = row_h.max(mb.height());
            row_items += 1;
        }
    }

    fn layout_standalone_table_row(&mut self, style_tree: &StyleTree) {
        let row_w = self.dimensions.content.width().max(40.0);
        let origin_x = self.dimensions.content.x();
        let origin_y = self.dimensions.content.y();
        let cols = self.children.len().max(1);
        let col_widths: Vec<f32> = self
            .children
            .iter()
            .map(|c| {
                c.node_id()
                    .and_then(|id| style_tree.get_style(id))
                    .and_then(|s| resolve_optional_length(&s.width, row_w))
                    .unwrap_or(row_w / cols as f32)
            })
            .collect();
        let sum: f32 = col_widths.iter().sum();
        let normalized: Vec<f32> = if sum > 0.0 {
            col_widths.into_iter().map(|w| w * (row_w / sum)).collect()
        } else {
            vec![row_w / cols as f32; cols]
        };
        self.layout_table_row(origin_x, origin_y, row_w, &normalized, style_tree);
    }

    fn layout_table(&mut self, style_tree: &StyleTree) {
        let table_w = self.dimensions.content.width().max(40.0);
        let origin_x = self.dimensions.content.x();
        let mut cursor_y = self.dimensions.content.y();

        // Count max columns across rows directly or inside row-groups
        let mut max_cols = 0usize;
        for child in &self.children {
            match child.display_mode {
                Display::TableRow => max_cols = max_cols.max(child.children.len()),
                Display::TableRowGroup => {
                    for row in &child.children {
                        if row.display_mode == Display::TableRow {
                            max_cols = max_cols.max(row.children.len());
                        }
                    }
                }
                _ => {}
            }
        }
        let cols = max_cols.max(1);
        let col_widths = compute_table_col_widths(&self.children, cols, table_w, style_tree);

        for child in &mut self.children {
            match child.display_mode {
                Display::TableRow => {
                    child.layout_table_row(origin_x, cursor_y, table_w, &col_widths, style_tree);
                    cursor_y += child.dimensions.margin_box().height();
                }
                Display::TableRowGroup => {
                    let group_start_y = cursor_y;
                    child.dimensions.content.origin = Point::new(origin_x, cursor_y);
                    child.dimensions.content.size.width = table_w;
                    for row in &mut child.children {
                        if row.display_mode == Display::TableRow {
                            row.layout_table_row(origin_x, cursor_y, table_w, &col_widths, style_tree);
                            cursor_y += row.dimensions.margin_box().height();
                        } else {
                            let row_cb = Dimensions {
                                content: Rect::new(origin_x, cursor_y, table_w, 0.0),
                                ..Default::default()
                            };
                            row.layout(&row_cb, style_tree);
                            cursor_y += row.dimensions.margin_box().height();
                        }
                    }
                    child.dimensions.content.size.height = (cursor_y - group_start_y).max(0.0);
                }
                _ => {
                    let child_cb = Dimensions {
                        content: Rect::new(origin_x, cursor_y, table_w, 0.0),
                        ..Default::default()
                    };
                    child.layout(&child_cb, style_tree);
                    cursor_y += child.dimensions.margin_box().height();
                }
            }
        }
    }

    fn layout_table_row(
        &mut self,
        origin_x: f32,
        origin_y: f32,
        row_w: f32,
        col_widths: &[f32],
        style_tree: &StyleTree,
    ) {
        self.dimensions.content.origin = Point::new(origin_x, origin_y);
        self.dimensions.content.size.width = row_w;

        let mut cell_x = origin_x;
        let mut max_row_h: f32 = 0.0;
        let cell_count = self.children.len().max(1);

        for (i, cell) in self.children.iter_mut().enumerate() {
            let cw = col_widths
                .get(i)
                .copied()
                .unwrap_or_else(|| (row_w / cell_count as f32).max(20.0));
            let cell_cb = Dimensions {
                content: Rect::new(cell_x, origin_y, cw, 0.0),
                ..Default::default()
            };
            cell.layout_block(&cell_cb, style_tree);
            max_row_h = max_row_h.max(cell.dimensions.margin_box().height());
            cell_x += cw;
        }

        // Uniform row cell heights for border-box parity
        for cell in &mut self.children {
            let overhead = cell.dimensions.margin.top
                + cell.dimensions.margin.bottom
                + cell.dimensions.border.top
                + cell.dimensions.border.bottom
                + cell.dimensions.padding.top
                + cell.dimensions.padding.bottom;
            cell.dimensions.content.size.height = cell
                .dimensions
                .content
                .size
                .height
                .max((max_row_h - overhead).max(0.0));
        }

        self.dimensions.content.size.height = max_row_h;
    }

    fn calculate_block_height(&mut self, style: &ComputedValues) {
        let min_h = match style.min_height {
            Length::Px(h) => h.max(0.0),
            _ => 0.0,
        };
        match style.height {
            Length::Px(h) => {
                self.dimensions.content.size.height = h.max(min_h).max(0.0);
            }
            _ => {
                if let Some(intrinsic) = self.intrinsic_size {
                    let h = if let Length::Px(w) = style.width {
                        if intrinsic.width > 0.0 {
                            (w * (intrinsic.height / intrinsic.width)).max(0.0)
                        } else {
                            intrinsic.height
                        }
                    } else if self.dimensions.content.size.width > 0.0 && intrinsic.width > 0.0 {
                        (self.dimensions.content.size.width * (intrinsic.height / intrinsic.width)).max(0.0)
                    } else {
                        intrinsic.height
                    };
                    self.dimensions.content.size.height = h.max(min_h);
                } else {
                    let origin_y = self.dimensions.content.y();
                    let max_bottom = self
                        .children
                        .iter()
                        .filter(|c| !c.is_out_of_flow())
                        .map(|c| {
                            let mb = c.dimensions.margin_box();
                            mb.y() + mb.height()
                        })
                        .fold(origin_y, f32::max);
                    self.dimensions.content.size.height = (max_bottom - origin_y).max(min_h).max(0.0);
                }
            }
        }
    }

    fn layout_inline_standalone(&mut self, containing_block: &Dimensions, style_tree: &StyleTree) {
        let style = self
            .node_id()
            .and_then(|id| style_tree.get_style(id).cloned())
            .unwrap_or_default();
        let available_w = containing_block.content.width().max(40.0);
        let mut cursor = InlineFlowCursor::new(
            containing_block.content.x(),
            containing_block.content.y(),
            available_w,
            style.text_align,
        );
        flow_inline_box(self, &mut cursor, &style, style_tree);
    }
}

struct InlineFlowCursor {
    origin_x: f32,
    max_width: f32,
    max_right: f32,
    cursor_x: f32,
    cursor_y: f32,
    line_height: f32,
    _text_align: TextAlign,
}

impl InlineFlowCursor {
    fn new(origin_x: f32, start_y: f32, max_width: f32, text_align: TextAlign) -> Self {
        Self {
            origin_x,
            max_width,
            max_right: origin_x + max_width,
            cursor_x: origin_x,
            cursor_y: start_y,
            line_height: 0.0,
            _text_align: text_align,
        }
    }

    fn new_line(&mut self, min_h: f32) {
        let step = self.line_height.max(min_h).max(16.0);
        self.cursor_y += step;
        self.cursor_x = self.origin_x;
        self.line_height = 0.0;
    }

    fn finish_line(&mut self) {
        if self.line_height > 0.0 {
            self.cursor_y += self.line_height;
            self.cursor_x = self.origin_x;
            self.line_height = 0.0;
        }
    }
}

fn flow_inline_box(
    lbox: &mut LayoutBox,
    cursor: &mut InlineFlowCursor,
    parent_style: &ComputedValues,
    style_tree: &StyleTree,
) {
    let style = lbox
        .node_id()
        .and_then(|id| style_tree.get_style(id).cloned())
        .unwrap_or_else(|| parent_style.clone());

    if lbox.is_line_break {
        lbox.dimensions.content = Rect::new(cursor.cursor_x, cursor.cursor_y, 0.0, 0.0);
        cursor.new_line(style.font_size * 1.25);
        return;
    }

    // 1. Leaf Text node -> tokenize into flowing TextRuns wrapped to container edge
    if let Some(ref text) = lbox.text_content
        && lbox.children.is_empty()
        && !lbox.is_replaced()
    {
        flow_text_node(lbox, text.clone(), cursor, &style);
        return;
    }

    lbox.dimensions.padding = EdgeSizes::new(
        resolve_px_nonneg(&style.padding_top, cursor.max_width),
        resolve_px_nonneg(&style.padding_right, cursor.max_width),
        resolve_px_nonneg(&style.padding_bottom, cursor.max_width),
        resolve_px_nonneg(&style.padding_left, cursor.max_width),
    );
    lbox.dimensions.border = EdgeSizes::new(
        style.border_top_width,
        style.border_right_width,
        style.border_bottom_width,
        style.border_left_width,
    );
    lbox.dimensions.margin = EdgeSizes::new(
        length_to_px_or_pct(&style.margin_top, cursor.max_width),
        length_to_px_or_pct(&style.margin_right, cursor.max_width),
        length_to_px_or_pct(&style.margin_bottom, cursor.max_width),
        length_to_px_or_pct(&style.margin_left, cursor.max_width),
    );

    // 2. Replaced inline elements (<img>, <input>, <button>, <textarea>, <select>)
    if let Some(intrinsic) = lbox.intrinsic_size {
        let mut width = match resolve_optional_length(&style.width, cursor.max_width) {
            Some(w) => w,
            None => match resolve_optional_length(&style.height, cursor.max_width) {
                Some(h) if intrinsic.height > 0.0 => h * (intrinsic.width / intrinsic.height),
                _ => intrinsic.width,
            },
        };
        let mut height = match resolve_optional_length(&style.height, cursor.max_width) {
            Some(h) => h,
            None => match resolve_optional_length(&style.width, cursor.max_width) {
                Some(w) if intrinsic.width > 0.0 => w * (intrinsic.height / intrinsic.width),
                _ => intrinsic.height,
            },
        };

        // Clamp unstyled oversized images smoothly to fit inside mobile container width
        if style.width == Length::Auto && style.height == Length::Auto && width > cursor.max_width && cursor.max_width > 20.0 {
            let ratio = cursor.max_width / width;
            width = cursor.max_width;
            height = (height * ratio).max(1.0);
        }

        width = width.max(0.0);
        height = height.max(0.0);

        let total_w = lbox.dimensions.margin.left
            + lbox.dimensions.border.left
            + lbox.dimensions.padding.left
            + width
            + lbox.dimensions.padding.right
            + lbox.dimensions.border.right
            + lbox.dimensions.margin.right;
        let total_h = lbox.dimensions.margin.top
            + lbox.dimensions.border.top
            + lbox.dimensions.padding.top
            + height
            + lbox.dimensions.padding.bottom
            + lbox.dimensions.border.bottom
            + lbox.dimensions.margin.bottom;

        if cursor.cursor_x > cursor.origin_x && cursor.cursor_x + total_w > cursor.max_right {
            cursor.new_line(height);
        }

        let x = cursor.cursor_x
            + lbox.dimensions.margin.left
            + lbox.dimensions.border.left
            + lbox.dimensions.padding.left;
        let y = cursor.cursor_y
            + lbox.dimensions.margin.top
            + lbox.dimensions.border.top
            + lbox.dimensions.padding.top;

        lbox.dimensions.content = Rect::new(x, y, width, height);
        cursor.cursor_x += total_w;
        cursor.line_height = cursor.line_height.max(total_h);
        return;
    }

    // 3. InlineBlock container: lay out its internal block children at (0, 0), wrap if needed, shift
    if lbox.display_mode == Display::InlineBlock {
        let explicit_w = resolve_optional_length(&style.width, cursor.max_width);
        let pad_border = lbox.dimensions.padding.left
            + lbox.dimensions.padding.right
            + lbox.dimensions.border.left
            + lbox.dimensions.border.right;
        let avail_inner = explicit_w.unwrap_or((cursor.max_width - pad_border).max(20.0));

        let temp_cb = Dimensions {
            content: Rect::new(0.0, 0.0, avail_inner, 0.0),
            ..Default::default()
        };
        lbox.dimensions.content.origin = Point::new(0.0, 0.0);
        lbox.dimensions.content.size.width = avail_inner;
        lbox.layout_block_or_inline_children(&style, style_tree);
        lbox.calculate_block_height(&style);

        // Shrink-to-fit if width was auto
        if explicit_w.is_none() {
            let intrinsic_child_w = compute_children_max_right(&lbox.children, 0.0);
            if intrinsic_child_w > 0.0 {
                lbox.dimensions.content.size.width = intrinsic_child_w.min(avail_inner);
            }
        }
        let _ = temp_cb;

        let total_w = lbox.dimensions.margin_box().width();
        let total_h = lbox.dimensions.margin_box().height();
        if cursor.cursor_x > cursor.origin_x && cursor.cursor_x + total_w > cursor.max_right {
            cursor.new_line(total_h);
        }

        let target_x = cursor.cursor_x
            + lbox.dimensions.margin.left
            + lbox.dimensions.border.left
            + lbox.dimensions.padding.left;
        let target_y = cursor.cursor_y
            + lbox.dimensions.margin.top
            + lbox.dimensions.border.top
            + lbox.dimensions.padding.top;
        lbox.shift_by(target_x, target_y);

        cursor.cursor_x += total_w;
        cursor.line_height = cursor.line_height.max(total_h);
        return;
    }

    // 4. Non-replaced Inline wrapper (<a>, <span>, <b>, etc.)
    let lead_x = lbox.dimensions.margin.left + lbox.dimensions.border.left + lbox.dimensions.padding.left;
    let trail_x = lbox.dimensions.padding.right + lbox.dimensions.border.right + lbox.dimensions.margin.right;

    if cursor.cursor_x + lead_x > cursor.max_right && cursor.cursor_x > cursor.origin_x {
        cursor.new_line(style.font_size * 1.25);
    }

    let entry_y = cursor.cursor_y + lbox.dimensions.margin.top + lbox.dimensions.border.top + lbox.dimensions.padding.top;
    cursor.cursor_x += lead_x;

    let mut children = std::mem::take(&mut lbox.children);
    for child in &mut children {
        flow_inline_box(child, cursor, &style, style_tree);
    }
    cursor.cursor_x += trail_x;

    // Tightly bound all flowed inline descendants so background/borders & hit-test enclose children cleanly
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;

    for child in &children {
        collect_visual_bounds(child, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
    }

    if min_x.is_finite() && min_y.is_finite() {
        lbox.dimensions.content = Rect::new(min_x, min_y, (max_x - min_x).max(0.0), (max_y - min_y).max(0.0));
    } else {
        let w = resolve_optional_length(&style.width, cursor.max_width).unwrap_or(0.0);
        let h = resolve_optional_length(&style.height, cursor.max_width).unwrap_or(0.0);
        lbox.dimensions.content = Rect::new(cursor.cursor_x - trail_x, entry_y, w, h);
    }

    lbox.children = children;
    let mb_h = lbox.dimensions.margin_box().height();
    cursor.line_height = cursor.line_height.max(mb_h);
}

fn flow_text_node(
    lbox: &mut LayoutBox,
    text: String,
    cursor: &mut InlineFlowCursor,
    style: &ComputedValues,
) {
    let font = nirvana_font::Font::default_font();
    let font_size = style.font_size;
    let line_h = if style.line_height > 0.0 {
        style.line_height.max(font_size * 1.1)
    } else {
        font.measure_text("Ay", font_size)
            .line_height
            .max(font_size * 1.28)
    };

    lbox.text_runs.clear();

    if text == " " {
        let space_w = font.measure_text(" ", font_size).width;
        if cursor.cursor_x > cursor.origin_x && cursor.cursor_x + space_w <= cursor.max_right {
            lbox.dimensions.content = Rect::new(cursor.cursor_x, cursor.cursor_y, space_w, line_h);
            cursor.cursor_x += space_w;
            cursor.line_height = cursor.line_height.max(line_h);
        } else {
            lbox.dimensions.content = Rect::new(cursor.cursor_x, cursor.cursor_y, 0.0, line_h);
        }
        return;
    }

    let leading_space = text.starts_with(' ');
    let trailing_space = text.ends_with(' ');
    let words: Vec<&str> = text.split_whitespace().collect();
    if words.is_empty() {
        lbox.dimensions.content = Rect::new(cursor.cursor_x, cursor.cursor_y, 0.0, 0.0);
        return;
    }

    let mut line_buf = String::new();
    let mut line_start_x = cursor.cursor_x;
    let mut line_start_y = cursor.cursor_y;

    let flush_run = |buf: &mut String, sx: f32, sy: f32, runs: &mut Vec<TextRun>| {
        if !buf.is_empty() {
            let w = font.measure_text(buf, font_size).width;
            runs.push(TextRun {
                text: std::mem::take(buf),
                rect: Rect::new(sx, sy, w, line_h),
                font_size,
            });
        }
    };

    for (idx, word) in words.iter().enumerate() {
        let need_space = if idx == 0 {
            leading_space && cursor.cursor_x > cursor.origin_x
        } else {
            true
        };

        let mut token_str = if need_space {
            format!(" {word}")
        } else {
            (*word).to_string()
        };
        let mut token_w = font.measure_text(&token_str, font_size).width;

        // Wrap to next line if token won't fit remaining horizontal line space
        if cursor.cursor_x > cursor.origin_x && cursor.cursor_x + token_w > cursor.max_right {
            flush_run(&mut line_buf, line_start_x, line_start_y, &mut lbox.text_runs);
            cursor.new_line(line_h);
            line_start_x = cursor.cursor_x;
            line_start_y = cursor.cursor_y;
            token_str = (*word).to_string();
            token_w = font.measure_text(&token_str, font_size).width;
        }

        // If a single word alone exceeds the entire container width, split it gracefully
        if token_w > cursor.max_width && cursor.max_width > 24.0 {
            let trimmed_word = token_str.trim_start();
            let parts = split_overlong_word(trimmed_word, font, font_size, cursor.max_width);
            for (p_idx, part) in parts.into_iter().enumerate() {
                if p_idx > 0 {
                    flush_run(&mut line_buf, line_start_x, line_start_y, &mut lbox.text_runs);
                    cursor.new_line(line_h);
                    line_start_x = cursor.cursor_x;
                    line_start_y = cursor.cursor_y;
                }
                let pw = font.measure_text(&part, font_size).width;
                line_buf.push_str(&part);
                cursor.cursor_x += pw;
                cursor.line_height = cursor.line_height.max(line_h);
            }
        } else {
            line_buf.push_str(&token_str);
            cursor.cursor_x += token_w;
            cursor.line_height = cursor.line_height.max(line_h);
        }
    }

    flush_run(&mut line_buf, line_start_x, line_start_y, &mut lbox.text_runs);

    if trailing_space && cursor.cursor_x > cursor.origin_x {
        let sp_w = font.measure_text(" ", font_size).width;
        if cursor.cursor_x + sp_w <= cursor.max_right {
            cursor.cursor_x += sp_w;
        }
    }

    if let Some(first) = lbox.text_runs.first() {
        let mut min_x = first.rect.x();
        let mut min_y = first.rect.y();
        let mut max_x = first.rect.right();
        let mut max_y = first.rect.bottom();
        for run in &lbox.text_runs[1..] {
            min_x = min_x.min(run.rect.x());
            min_y = min_y.min(run.rect.y());
            max_x = max_x.max(run.rect.right());
            max_y = max_y.max(run.rect.bottom());
        }
        lbox.dimensions.content = Rect::new(min_x, min_y, (max_x - min_x).max(0.0), (max_y - min_y).max(line_h));
    } else {
        lbox.dimensions.content = Rect::new(cursor.cursor_x, cursor.cursor_y, 0.0, line_h);
    }
}

fn align_inline_children(
    children: &mut [LayoutBox],
    origin_x: f32,
    container_w: f32,
    align: TextAlign,
) {
    if align == TextAlign::Left || children.is_empty() {
        return;
    }
    let mut min_x = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for child in children.iter() {
        collect_visual_bounds(child, &mut min_x, &mut min_y, &mut max_x, &mut max_y);
    }
    if !min_x.is_finite() || !max_x.is_finite() {
        return;
    }
    let content_w = max_x - min_x;
    if content_w >= container_w {
        return;
    }
    let target_start = match align {
        TextAlign::Center => origin_x + ((container_w - content_w) / 2.0),
        TextAlign::Right => origin_x + (container_w - content_w),
        TextAlign::Left => origin_x,
    };
    let dx = target_start - min_x;
    if dx.abs() > 0.5 {
        for child in children.iter_mut() {
            child.shift_by(dx, 0.0);
        }
    }
}

fn collect_visual_bounds(
    lbox: &LayoutBox,
    min_x: &mut f32,
    min_y: &mut f32,
    max_x: &mut f32,
    max_y: &mut f32,
) {
    if !lbox.text_runs.is_empty() {
        for run in &lbox.text_runs {
            if run.rect.width() > 0.0 || run.rect.height() > 0.0 {
                *min_x = (*min_x).min(run.rect.x());
                *min_y = (*min_y).min(run.rect.y());
                *max_x = (*max_x).max(run.rect.right());
                *max_y = (*max_y).max(run.rect.bottom());
            }
        }
        return;
    }
    if lbox.children.is_empty() {
        let bb = lbox.dimensions.border_box();
        if bb.width() > 0.0 || bb.height() > 0.0 {
            *min_x = (*min_x).min(bb.x());
            *min_y = (*min_y).min(bb.y());
            *max_x = (*max_x).max(bb.right());
            *max_y = (*max_y).max(bb.bottom());
        }
        return;
    }
    for child in &lbox.children {
        collect_visual_bounds(child, min_x, min_y, max_x, max_y);
    }
}

fn compute_children_max_right(children: &[LayoutBox], origin_x: f32) -> f32 {
    children
        .iter()
        .map(|c| (c.dimensions.margin_box().right() - origin_x).max(0.0))
        .fold(0.0, f32::max)
}

fn compute_table_col_widths(
    table_children: &[LayoutBox],
    cols: usize,
    table_w: f32,
    style_tree: &StyleTree,
) -> Vec<f32> {
    let mut desired = vec![24.0f32; cols];

    let mut inspect_row = |row: &LayoutBox| {
        for (i, cell) in row.children.iter().enumerate() {
            if i >= cols {
                break;
            }
            if let Some(id) = cell.node_id()
                && let Some(style) = style_tree.get_style(id)
                && let Some(w) = resolve_optional_length(&style.width, table_w)
            {
                desired[i] = desired[i].max(w);
                continue;
            }
            let est = css_clamp_size(estimate_subtree_width(cell), 24.0, table_w.max(24.0));
            desired[i] = desired[i].max(est);
        }
    };

    for child in table_children {
        match child.display_mode {
            Display::TableRow => inspect_row(child),
            Display::TableRowGroup => {
                for row in &child.children {
                    if row.display_mode == Display::TableRow {
                        inspect_row(row);
                    }
                }
            }
            _ => {}
        }
    }

    let sum: f32 = desired.iter().sum();
    if sum <= table_w && sum > 0.0 {
        let extra = (table_w - sum) / cols as f32;
        return desired.into_iter().map(|w| w + extra).collect();
    }
    if sum > 0.0 {
        // Compress wide columns proportionally while preserving narrow index columns cleanly
        let min_col = 22.0f32.min(table_w / cols as f32);
        let scale = table_w / sum;
        let scaled: Vec<f32> = desired.iter().map(|w| (w * scale).max(min_col)).collect();
        let scaled_sum: f32 = scaled.iter().sum();
        if scaled_sum > 0.0 {
            let norm = table_w / scaled_sum;
            return scaled.into_iter().map(|w| w * norm).collect();
        }
    }
    vec![table_w / cols as f32; cols]
}

fn estimate_subtree_width(lbox: &LayoutBox) -> f32 {
    if let Some(ref txt) = lbox.text_content {
        return nirvana_font::Font::default_font()
            .measure_text(txt.trim(), 16.0)
            .width
            + 12.0;
    }
    if let Some(intr) = lbox.intrinsic_size {
        return intr.width + 8.0;
    }
    let mut total = 0.0f32;
    for child in &lbox.children {
        total += estimate_subtree_width(child);
    }
    total.max(24.0)
}

fn length_to_px_or_pct(len: &Length, container_w: f32) -> f32 {
    match len {
        Length::Px(p) => *p,
        Length::Percent(pct) => (pct / 100.0) * container_w,
        Length::Auto => 0.0,
    }
}

fn resolve_px_nonneg(len: &Length, container_w: f32) -> f32 {
    length_to_px_or_pct(len, container_w).max(0.0)
}

fn resolve_optional_length(len: &Length, container_w: f32) -> Option<f32> {
    match len {
        Length::Px(p) => Some(*p),
        Length::Percent(pct) => Some((pct / 100.0) * container_w),
        Length::Auto => None,
    }
}

/// Safely clamps a CSS size so `min_val` takes precedence over `max_val` per CSS Sizing and NaN values never panic.
#[inline]
fn css_clamp_size(val: f32, min_val: f32, max_val: f32) -> f32 {
    let lo = if min_val.is_finite() { min_val.max(0.0) } else { 0.0 };
    let hi = if max_val.is_finite() { max_val.max(0.0) } else { f32::MAX };
    if !val.is_finite() {
        return lo;
    }
    val.min(hi).max(lo).max(0.0)
}
