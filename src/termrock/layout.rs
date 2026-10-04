//! Terminal geometry, constraints, responsive helpers, and pure layout algorithms.
//!
//! Owns cell-exact [`Rect`] and [`Size`] arithmetic, track allocation with gaps,
//! responsive breakpoint splitting, and safe degenerate handling.

use ratatui::layout::Position as RatatuiPosition;
use ratatui::layout::Rect as RatatuiRect;

/// A 2D point in terminal cell coordinates.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Position {
    pub x: u16,
    pub y: u16,
}

impl Position {
    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }
}

impl From<RatatuiPosition> for Position {
    fn from(p: RatatuiPosition) -> Self {
        Self { x: p.x, y: p.y }
    }
}

impl From<Position> for RatatuiPosition {
    fn from(p: Position) -> Self {
        Self { x: p.x, y: p.y }
    }
}

/// A 2D size in terminal cells.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Size {
    pub width: u16,
    pub height: u16,
}

impl Size {
    pub const fn new(width: u16, height: u16) -> Self {
        Self { width, height }
    }

    pub const fn zero() -> Self {
        Self {
            width: 0,
            height: 0,
        }
    }
}

/// A rectangular area of terminal cells.
#[derive(Copy, Clone, PartialEq, Eq, Hash, Debug, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

impl Rect {
    pub const fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn zero() -> Self {
        Self {
            x: 0,
            y: 0,
            width: 0,
            height: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn contains(&self, pos: Position) -> bool {
        pos.x >= self.x
            && pos.x < self.x.saturating_add(self.width)
            && pos.y >= self.y
            && pos.y < self.y.saturating_add(self.height)
    }

    pub fn intersect(&self, other: Rect) -> Rect {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self
            .x
            .saturating_add(self.width)
            .min(other.x.saturating_add(other.width));
        let y2 = self
            .y
            .saturating_add(self.height)
            .min(other.y.saturating_add(other.height));
        if x2 > x1 && y2 > y1 {
            Rect::new(x1, y1, x2 - x1, y2 - y1)
        } else {
            Rect::new(x1, y1, 0, 0)
        }
    }

    pub fn union(&self, other: Rect) -> Rect {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }
        let x1 = self.x.min(other.x);
        let y1 = self.y.min(other.y);
        let x2 = self
            .x
            .saturating_add(self.width)
            .max(other.x.saturating_add(other.width));
        let y2 = self
            .y
            .saturating_add(self.height)
            .max(other.y.saturating_add(other.height));
        Rect::new(x1, y1, x2 - x1, y2 - y1)
    }

    pub fn inset(&self, dx: u16, dy: u16) -> Rect {
        let double_dx = dx.saturating_mul(2);
        let double_dy = dy.saturating_mul(2);
        if self.width <= double_dx || self.height <= double_dy {
            Rect::new(self.x.saturating_add(dx), self.y.saturating_add(dy), 0, 0)
        } else {
            Rect::new(
                self.x.saturating_add(dx),
                self.y.saturating_add(dy),
                self.width - double_dx,
                self.height - double_dy,
            )
        }
    }
}

impl From<RatatuiRect> for Rect {
    fn from(r: RatatuiRect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

impl From<Rect> for RatatuiRect {
    fn from(r: Rect) -> Self {
        Self {
            x: r.x,
            y: r.y,
            width: r.width,
            height: r.height,
        }
    }
}

/// Bounded min/max sizing constraints.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Constraints {
    pub min: Size,
    pub max: Size,
}

impl Constraints {
    /// Construct constraints, ensuring `min <= max` component-wise.
    pub fn new(min: Size, max: Size) -> Self {
        let min_w = min.width.min(max.width);
        let min_h = min.height.min(max.height);
        let max_w = min.width.max(max.width);
        let max_h = min.height.max(max.height);
        Self {
            min: Size::new(min_w, min_h),
            max: Size::new(max_w, max_h),
        }
    }

    pub const fn tight(size: Size) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    pub const fn loose(max: Size) -> Self {
        Self {
            min: Size::zero(),
            max,
        }
    }

    pub const fn unbounded() -> Self {
        Self {
            min: Size::zero(),
            max: Size::new(u16::MAX, u16::MAX),
        }
    }

    pub fn clamp(&self, size: Size) -> Size {
        Size::new(
            size.width.clamp(self.min.width, self.max.width),
            size.height.clamp(self.min.height, self.max.height),
        )
    }
}

/// Sizing track specification.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Track {
    Fixed(u16),
    Flex(u16),
    Auto,
}

/// Content alignment policy along an axis.
#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
pub enum Alignment {
    #[default]
    Start,
    Center,
    End,
}

/// Layout axis.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Axis {
    Horizontal,
    Vertical,
}

/// Measure trait for components.
pub trait Measure {
    fn measure(&self, constraints: Constraints) -> Size;
}

fn distribute_tracks(length: u16, tracks: &[Track], gap: u16) -> Vec<(u16, u16)> {
    if tracks.is_empty() {
        return Vec::new();
    }

    let n = tracks.len();
    let total_gap = (n.saturating_sub(1) as u16).saturating_mul(gap);
    let usable_length = length.saturating_sub(total_gap);

    let mut fixed_sum: u16 = 0;
    let mut total_flex: u32 = 0;

    for &track in tracks {
        match track {
            Track::Fixed(w) => fixed_sum = fixed_sum.saturating_add(w),
            Track::Flex(w) => total_flex = total_flex.saturating_add(w.max(1) as u32),
            Track::Auto => total_flex = total_flex.saturating_add(1),
        }
    }

    let mut lengths = vec![0u16; n];
    let mut remaining = usable_length;

    // First allocate fixed
    if fixed_sum > usable_length {
        // Not enough room for fixed tracks: allocate proportionally or sequentially
        for (i, &track) in tracks.iter().enumerate() {
            if let Track::Fixed(w) = track {
                let alloc = w.min(remaining);
                lengths[i] = alloc;
                remaining = remaining.saturating_sub(alloc);
            }
        }
    } else {
        for (i, &track) in tracks.iter().enumerate() {
            if let Track::Fixed(w) = track {
                lengths[i] = w;
                remaining = remaining.saturating_sub(w);
            }
        }

        // Allocate remaining to flex/auto
        if total_flex > 0 && remaining > 0 {
            let flex_avail = remaining as u32;
            let mut allocated_flex = 0u32;
            for (i, &track) in tracks.iter().enumerate() {
                let weight = match track {
                    Track::Flex(w) => w.max(1) as u32,
                    Track::Auto => 1,
                    Track::Fixed(_) => continue,
                };
                let share = (flex_avail * weight) / total_flex;
                lengths[i] = share as u16;
                allocated_flex += share;
            }
            // Remainder distribution
            let mut remainder = (flex_avail - allocated_flex) as usize;
            for (i, &track) in tracks.iter().enumerate() {
                if remainder == 0 {
                    break;
                }
                if matches!(track, Track::Flex(_) | Track::Auto) {
                    lengths[i] = lengths[i].saturating_add(1);
                    remainder -= 1;
                }
            }
        }
    }

    // Compute offsets
    let mut result = Vec::with_capacity(n);
    let mut current_offset: u16 = 0;
    for len in lengths {
        result.push((current_offset, len));
        current_offset = current_offset.saturating_add(len).saturating_add(gap);
    }
    result
}

/// Divide area vertically into rows.
pub fn rows(area: Rect, tracks: &[Track], gap: u16) -> Vec<Rect> {
    if area.is_empty() || tracks.is_empty() {
        return tracks
            .iter()
            .map(|_| Rect::new(area.x, area.y, area.width, 0))
            .collect();
    }

    let allocations = distribute_tracks(area.height, tracks, gap);
    allocations
        .into_iter()
        .map(|(offset, h)| {
            let y = area.y.saturating_add(offset);
            let bounded_h = h.min(area.y.saturating_add(area.height).saturating_sub(y));
            Rect::new(area.x, y, area.width, bounded_h)
        })
        .collect()
}

/// Divide area horizontally into columns.
pub fn columns(area: Rect, tracks: &[Track], gap: u16) -> Vec<Rect> {
    if area.is_empty() || tracks.is_empty() {
        return tracks
            .iter()
            .map(|_| Rect::new(area.x, area.y, 0, area.height))
            .collect();
    }

    let allocations = distribute_tracks(area.width, tracks, gap);
    allocations
        .into_iter()
        .map(|(offset, w)| {
            let x = area.x.saturating_add(offset);
            let bounded_w = w.min(area.x.saturating_add(area.width).saturating_sub(x));
            Rect::new(x, area.y, bounded_w, area.height)
        })
        .collect()
}

/// Layout a row of action items (e.g. buttons) with specified alignment and gap.
pub fn action_row(area: Rect, sizes: &[Size], align: Alignment, gap: u16) -> Vec<Rect> {
    if sizes.is_empty() {
        return Vec::new();
    }
    if area.is_empty() {
        return sizes.iter().map(|_| Rect::zero()).collect();
    }

    let n = sizes.len();
    let total_gap = (n.saturating_sub(1) as u16).saturating_mul(gap);
    let total_item_width: u16 = sizes
        .iter()
        .map(|s| s.width)
        .fold(0, |acc, w| acc.saturating_add(w));
    let required_width = total_item_width.saturating_add(total_gap);

    let start_x = if required_width <= area.width {
        let slack = area.width - required_width;
        match align {
            Alignment::Start => area.x,
            Alignment::Center => area.x.saturating_add(slack / 2),
            Alignment::End => area.x.saturating_add(slack),
        }
    } else {
        area.x
    };

    let mut curr_x = start_x;
    let right_limit = area.x.saturating_add(area.width);
    let mut rects = Vec::with_capacity(n);

    for s in sizes {
        let max_w = right_limit.saturating_sub(curr_x);
        let w = s.width.min(max_w);
        let h = s.height.min(area.height);
        rects.push(Rect::new(curr_x, area.y, w, h));
        curr_x = curr_x
            .saturating_add(w)
            .saturating_add(gap)
            .min(right_limit);
    }

    rects
}

/// Responsive column specification with breakpoint and track rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsiveSpec {
    pub breakpoint: u16,
    pub left_track: Track,
    pub right_track: Track,
    pub gap: u16,
}

/// Allocation facts for responsive layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResponsiveAreas {
    pub side_by_side: bool,
    pub primary: Rect,
    pub secondary: Option<Rect>,
}

/// Compute responsive column areas based on width breakpoint.
pub fn responsive_columns(area: Rect, spec: ResponsiveSpec) -> ResponsiveAreas {
    if area.width < spec.breakpoint {
        ResponsiveAreas {
            side_by_side: false,
            primary: area,
            secondary: None,
        }
    } else {
        let cols = columns(area, &[spec.left_track, spec.right_track], spec.gap);
        ResponsiveAreas {
            side_by_side: true,
            primary: cols.first().copied().unwrap_or(area),
            secondary: cols.get(1).copied(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_operations_and_containment() {
        let r1 = Rect::new(10, 10, 20, 20);
        assert!(!r1.is_empty());
        assert!(r1.contains(Position::new(10, 10)));
        assert!(r1.contains(Position::new(29, 29)));
        assert!(!r1.contains(Position::new(30, 30)));
        assert!(!r1.contains(Position::new(9, 10)));

        let r2 = Rect::new(20, 20, 20, 20);
        let inter = r1.intersect(r2);
        assert_eq!(inter, Rect::new(20, 20, 10, 10));

        let union = r1.union(r2);
        assert_eq!(union, Rect::new(10, 10, 30, 30));

        let inset = r1.inset(2, 3);
        assert_eq!(inset, Rect::new(12, 13, 16, 14));

        let zero = Rect::zero();
        assert!(zero.is_empty());
    }

    #[test]
    fn degenerate_rectangles() {
        let empty = Rect::new(5, 5, 0, 0);
        assert!(empty.is_empty());

        let one_by_one = Rect::new(10, 10, 1, 1);
        assert!(!one_by_one.is_empty());
        assert!(one_by_one.contains(Position::new(10, 10)));
        assert!(!one_by_one.contains(Position::new(11, 10)));

        let inset_exhausted = one_by_one.inset(2, 2);
        assert!(inset_exhausted.is_empty());

        let rows_empty = rows(empty, &[Track::Fixed(10), Track::Flex(1)], 1);
        assert_eq!(rows_empty.len(), 2);
        assert!(rows_empty[0].is_empty());
        assert!(rows_empty[1].is_empty());
    }

    #[test]
    fn track_distribution_and_gaps() {
        let area = Rect::new(0, 0, 100, 50);
        let cols = columns(area, &[Track::Fixed(20), Track::Flex(1), Track::Flex(2)], 2);
        assert_eq!(cols.len(), 3);
        assert_eq!(cols[0].width, 20);
        // Usable = 100 - 4 = 96. Fixed = 20. Remaining = 76.
        // Flex total = 3. 76 / 3 = 25 remainder 1.
        // First flex gets 25 + 1 = 26. Second flex gets 50.
        assert_eq!(cols[1].width, 26);
        assert_eq!(cols[2].width, 50);
        assert_eq!(cols[0].x, 0);
        assert_eq!(cols[1].x, 22);
        assert_eq!(cols[2].x, 50);
    }

    #[test]
    fn action_row_alignment() {
        let area = Rect::new(10, 10, 50, 5);
        let sizes = vec![Size::new(10, 2), Size::new(10, 2)];
        // Total = 20 + 2 = 22. Slack = 28.
        let left = action_row(area, &sizes, Alignment::Start, 2);
        assert_eq!(left[0].x, 10);
        assert_eq!(left[1].x, 22);

        let right = action_row(area, &sizes, Alignment::End, 2);
        assert_eq!(right[0].x, 38);
        assert_eq!(right[1].x, 50);

        let center = action_row(area, &sizes, Alignment::Center, 2);
        assert_eq!(center[0].x, 24);
        assert_eq!(center[1].x, 36);
    }

    #[test]
    fn responsive_columns_breakpoint() {
        let spec = ResponsiveSpec {
            breakpoint: 80,
            left_track: Track::Fixed(20),
            right_track: Track::Flex(1),
            gap: 1,
        };

        let narrow = Rect::new(0, 0, 70, 20);
        let r_narrow = responsive_columns(narrow, spec);
        assert!(!r_narrow.side_by_side);
        assert_eq!(r_narrow.primary, narrow);
        assert_eq!(r_narrow.secondary, None);

        let wide = Rect::new(0, 0, 100, 20);
        let r_wide = responsive_columns(wide, spec);
        assert!(r_wide.side_by_side);
        assert_eq!(r_wide.primary.width, 20);
        assert!(r_wide.secondary.is_some());
        assert_eq!(r_wide.secondary.unwrap().width, 79);
    }

    #[test]
    fn constraints_clamping() {
        let c = Constraints::new(Size::new(10, 10), Size::new(50, 50));
        assert_eq!(c.clamp(Size::new(5, 5)), Size::new(10, 10));
        assert_eq!(c.clamp(Size::new(100, 100)), Size::new(50, 50));
        assert_eq!(c.clamp(Size::new(30, 30)), Size::new(30, 30));
    }
}
