//! A library's flattened struct, which its consumer may lend whole, its
//! parent's object: the library's own derived `==`, `Hash` and clone of it
//! read its fields alone, as every struct a consumer can reach that a
//! flattened field holds is (#1820).
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Grid {
    pub cols: u32,
    pub rows: u32,
}

pub struct Board {
    #[cfg_attr(rust_js, rust_js::flatten)]
    pub grid: Grid,
    pub name: &'static str,
}

pub fn same(a: &Grid, b: &Grid) -> bool {
    a == b
}

pub fn seen(grids: &[&Grid]) -> usize {
    grids.iter().map(|grid| **grid).collect::<HashSet<Grid>>().len()
}

pub fn kept(grid: &Grid) -> Grid {
    grid.clone()
}
