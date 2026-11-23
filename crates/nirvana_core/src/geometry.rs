//! Geometry primitives for 2D layout and rendering.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Point {
    pub x: f32,
    pub y: f32,
}

impl Point {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    #[inline]
    pub fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Size {
    pub width: f32,
    pub height: f32,
}

impl Size {
    pub const ZERO: Self = Self { width: 0.0, height: 0.0 };

    #[inline]
    pub fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Rect {
    pub origin: Point,
    pub size: Size,
}

impl Rect {
    pub const ZERO: Self = Self {
        origin: Point::ZERO,
        size: Size::ZERO,
    };

    #[inline]
    pub fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            origin: Point::new(x, y),
            size: Size::new(width, height),
        }
    }

    #[inline]
    pub fn x(&self) -> f32 {
        self.origin.x
    }

    #[inline]
    pub fn y(&self) -> f32 {
        self.origin.y
    }

    #[inline]
    pub fn width(&self) -> f32 {
        self.size.width
    }

    #[inline]
    pub fn height(&self) -> f32 {
        self.size.height
    }

    #[inline]
    pub fn right(&self) -> f32 {
        self.origin.x + self.size.width
    }

    #[inline]
    pub fn bottom(&self) -> f32 {
        self.origin.y + self.size.height
    }

    pub fn contains_point(&self, p: Point) -> bool {
        p.x >= self.x() && p.x <= self.right() && p.y >= self.y() && p.y <= self.bottom()
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x() < other.right()
            && self.right() > other.x()
            && self.y() < other.bottom()
            && self.bottom() > other.y()
    }

    pub fn union(&self, other: &Rect) -> Rect {
        let x = self.x().min(other.x());
        let y = self.y().min(other.y());
        let right = self.right().max(other.right());
        let bottom = self.bottom().max(other.bottom());
        Rect::new(x, y, right - x, bottom - y)
    }
}
