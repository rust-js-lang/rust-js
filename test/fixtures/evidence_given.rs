use std::panic::Location;

pub mod shape {
    pub trait Shape {
        fn area(&self) -> f64;
    }

    pub struct Square {
        pub side: f64,
    }

    impl Shape for Square {
        fn area(&self) -> f64 {
            self.side * self.side
        }
    }
}

use shape::Shape;

#[track_caller]
pub fn here() -> &'static Location<'static> {
    Location::caller()
}

#[track_caller]
pub fn checked(n: u32, why: Option<&str>) -> u32 {
    if n == 0 {
        panic!("{} at {}", why.unwrap_or("zero"), Location::caller());
    }
    n
}

#[track_caller]
pub fn unwrapped(n: Option<u32>) -> u32 {
    n.unwrap()
}

pub fn zeros<const N: usize>() -> [u8; N] {
    [0; N]
}

pub fn big<const N: u64>() -> u64 {
    N
}

#[track_caller]
pub fn sized<const N: usize, S: Shape>(shapes: [S; N]) -> String {
    format!(
        "{N} of area {} at {}",
        shapes.iter().map(|s| s.area()).sum::<f64>(),
        Location::caller().line()
    )
}

pub struct Grid<const W: usize> {
    pub cells: Vec<u8>,
}

impl<const W: usize> Grid<W> {
    pub fn width(&self) -> usize {
        W
    }
}

pub fn calls() -> usize {
    let grid: Grid<4> = Grid { cells: Vec::new() };
    zeros::<3>().len()
        + here().line() as usize
        + checked(1, None) as usize
        + big::<4>() as usize
        + unwrapped(Some(1)) as usize
        + sized([shape::Square { side: 1.0 }]).len()
        + grid.width()
}
