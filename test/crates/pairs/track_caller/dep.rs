// A library's function that tracks its caller and reads where it was called
// takes it, as its manifest says, `takes_location`, which its consumer's
// calls give (ADR 0427); one that reads nothing of it takes none.
use std::panic::Location;

#[track_caller]
pub fn line_here() -> u32 {
    Location::caller().line()
}

#[track_caller]
pub fn column_here() -> u32 {
    Location::caller().column()
}

#[track_caller]
pub fn quiet(x: u32) -> u32 {
    x + 1
}
