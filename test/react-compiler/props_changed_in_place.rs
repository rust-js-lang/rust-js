//! A component that changes its props in place, as Rust may of what it
//! owns: copied first (#1723), which React Compiler takes as a local of
//! its own, where a write to React's props it refuses ("This value cannot
//! be modified").
#![allow(non_snake_case)]
use react::{JSX, jsx};

pub struct Point {
    pub x: f64,
}

pub struct DotProps {
    pub at: Point,
}

pub fn Changed(mut props: DotProps) -> JSX::Element {
    props.at.x += 1.0;
    jsx! { <i>{props.at.x}</i> }
}

pub fn Apart(DotProps { mut at }: DotProps) -> JSX::Element {
    at.x += 1.0;
    jsx! { <i>{at.x}</i> }
}
