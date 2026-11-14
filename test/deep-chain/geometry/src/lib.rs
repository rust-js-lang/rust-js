//! Where a floating element goes beside the element it's anchored to, as
//! Floating UI places it: a side and an alignment, then middleware that
//! moves it, offsets it from its anchor, flips it to the other side where
//! it doesn't fit, and shifts it back inside the boundary.

use std::error::Error;
use std::fmt;
use std::str::FromStr;

/// A box on the page, in CSS pixels.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Rect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

impl Rect {
    pub fn new(x: f64, y: f64, width: f64, height: f64) -> Self {
        Rect { x, y, width, height }
    }

    pub fn right(&self) -> f64 {
        self.x + self.width
    }

    pub fn bottom(&self) -> f64 {
        self.y + self.height
    }
}

/// Where a floating element's top left corner goes.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Point {
    pub x: f64,
    pub y: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    Top,
    Right,
    Bottom,
    Left,
}

impl Side {
    pub fn opposite(self) -> Side {
        match self {
            Side::Top => Side::Bottom,
            Side::Right => Side::Left,
            Side::Bottom => Side::Top,
            Side::Left => Side::Right,
        }
    }

    fn is_vertical(self) -> bool {
        matches!(self, Side::Top | Side::Bottom)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Align {
    Start,
    #[default]
    Center,
    End,
}

/// A side and an alignment along it, `bottom-start`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Placement {
    pub side: Side,
    pub align: Align,
}

impl Default for Placement {
    fn default() -> Self {
        Placement {
            side: Side::Bottom,
            align: Align::Center,
        }
    }
}

impl fmt::Display for Placement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let side = match self.side {
            Side::Top => "top",
            Side::Right => "right",
            Side::Bottom => "bottom",
            Side::Left => "left",
        };
        match self.align {
            Align::Center => f.write_str(side),
            Align::Start => write!(f, "{side}-start"),
            Align::End => write!(f, "{side}-end"),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct ParsePlacementError(String);

impl fmt::Display for ParsePlacementError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown placement `{}`", self.0)
    }
}

impl Error for ParsePlacementError {}

impl FromStr for Placement {
    type Err = ParsePlacementError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let unknown = || ParsePlacementError(s.to_string());
        let (side, align) = match s.split_once('-') {
            Some((side, align)) => (side, Some(align)),
            None => (s, None),
        };
        let side = match side {
            "top" => Side::Top,
            "right" => Side::Right,
            "bottom" => Side::Bottom,
            "left" => Side::Left,
            _ => return Err(unknown()),
        };
        let align = match align {
            None => Align::Center,
            Some("start") => Align::Start,
            Some("end") => Align::End,
            Some(_) => return Err(unknown()),
        };
        Ok(Placement { side, align })
    }
}

/// What each middleware is given and gives back: where the floating
/// element is so far, and on which side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct State {
    pub placement: Placement,
    pub point: Point,
    pub reference: Rect,
    pub floating: Rect,
    pub boundary: Rect,
}

/// A step that moves the floating element after it's placed.
pub trait Middleware {
    fn name(&self) -> &'static str;

    fn apply(&self, state: State) -> State;

    fn describe(&self) -> String {
        self.name().to_string()
    }
}

/// Moves it away from its anchor, along its side.
pub struct Offset(pub f64);

impl Middleware for Offset {
    fn name(&self) -> &'static str {
        "offset"
    }

    fn apply(&self, mut state: State) -> State {
        match state.placement.side {
            Side::Top => state.point.y -= self.0,
            Side::Bottom => state.point.y += self.0,
            Side::Left => state.point.x -= self.0,
            Side::Right => state.point.x += self.0,
        }
        state
    }

    fn describe(&self) -> String {
        format!("offset({})", self.0)
    }
}

/// Puts it on the other side where it overflows the boundary on its own.
pub struct Flip;

impl Middleware for Flip {
    fn name(&self) -> &'static str {
        "flip"
    }

    fn apply(&self, state: State) -> State {
        if overflow(&state) <= 0.0 {
            return state;
        }
        let placement = Placement {
            side: state.placement.side.opposite(),
            ..state.placement
        };
        let flipped = State {
            placement,
            point: place(state.reference, state.floating, placement),
            ..state
        };
        if overflow(&flipped) < overflow(&state) { flipped } else { state }
    }
}

/// Keeps it inside the boundary, less `padding`, across its side.
pub struct Shift {
    pub padding: f64,
}

impl Middleware for Shift {
    fn name(&self) -> &'static str {
        "shift"
    }

    fn apply(&self, mut state: State) -> State {
        let State { boundary, floating, .. } = state;
        if state.placement.side.is_vertical() {
            let max = boundary.right() - floating.width - self.padding;
            state.point.x = clamp(state.point.x, boundary.x + self.padding, max);
        } else {
            let max = boundary.bottom() - floating.height - self.padding;
            state.point.y = clamp(state.point.y, boundary.y + self.padding, max);
        }
        state
    }
}

/// `value`, or `min` or `max` where it's past one.
pub fn clamp<T: PartialOrd>(value: T, min: T, max: T) -> T {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

/// How far it reaches past the boundary on its own side.
fn overflow(state: &State) -> f64 {
    let State { point, floating, boundary, .. } = state;
    match state.placement.side {
        Side::Top => boundary.y - point.y,
        Side::Bottom => point.y + floating.height - boundary.bottom(),
        Side::Left => boundary.x - point.x,
        Side::Right => point.x + floating.width - boundary.right(),
    }
}

/// Where `floating` goes on `placement` of `reference`, before middleware.
pub fn place(reference: Rect, floating: Rect, placement: Placement) -> Point {
    let along = |start: f64, length: f64, size: f64| match placement.align {
        Align::Start => start,
        Align::Center => start + (length - size) / 2.0,
        Align::End => start + length - size,
    };
    match placement.side {
        Side::Top => Point {
            x: along(reference.x, reference.width, floating.width),
            y: reference.y - floating.height,
        },
        Side::Bottom => Point {
            x: along(reference.x, reference.width, floating.width),
            y: reference.bottom(),
        },
        Side::Left => Point {
            x: reference.x - floating.width,
            y: along(reference.y, reference.height, floating.height),
        },
        Side::Right => Point {
            x: reference.right(),
            y: along(reference.y, reference.height, floating.height),
        },
    }
}

/// Where `floating` ends up, placed then moved by each middleware in turn.
pub fn compute_position(
    reference: Rect,
    floating: Rect,
    boundary: Rect,
    placement: Placement,
    middleware: &[Box<dyn Middleware>],
) -> State {
    let placed = State {
        placement,
        point: place(reference, floating, placement),
        reference,
        floating,
        boundary,
    };
    middleware.iter().fold(placed, |state, step| step.apply(state))
}

/// What's run when it's dropped, as Floating UI's `autoUpdate` gives back a
/// function that stops it.
pub struct Cleanup<F: FnMut()> {
    run: Option<F>,
}

impl<F: FnMut()> Cleanup<F> {
    pub fn new(run: F) -> Self {
        Cleanup { run: Some(run) }
    }
}

impl<F: FnMut()> Drop for Cleanup<F> {
    fn drop(&mut self) {
        if let Some(mut run) = self.run.take() {
            run();
        }
    }
}
