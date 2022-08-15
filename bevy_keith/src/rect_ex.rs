use bevy::{math::Vec2, sprite::Rect};

/// Polyfill for #5686 - Move `sprite::Rect` into `bevy_math`.
pub trait RectEx {
    fn from_corners(p0: Vec2, p1: Vec2) -> Self;
    fn contains(&self, point: Vec2) -> bool;
    fn is_empty(&self) -> bool;
    fn intersect(&self, other: Rect) -> Rect;
}

impl RectEx for Rect {
    #[inline]
    fn from_corners(p0: Vec2, p1: Vec2) -> Self {
        Rect {
            min: p0.min(p1),
            max: p0.max(p1),
        }
    }

    #[inline]
    fn contains(&self, point: Vec2) -> bool {
        point.cmpge(self.min).all() && point.cmple(self.max).all()
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.min.cmpge(self.max).any()
    }

    #[inline]
    fn intersect(&self, other: Rect) -> Rect {
        let mut r = Rect {
            min: self.min.max(other.min),
            max: self.max.min(other.max),
        };
        // Collapse min over max to enforce invariants and ensure e.g. width() or
        // height() never return a negative value.
        r.min = r.min.min(r.max);
        r
    }
}
