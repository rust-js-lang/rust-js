//! Lends a library's flattened struct whole, from parents of other names,
//! to the library's functions, which compare, hash and clone it (#1820).
pub fn main() {
    let a = dep::Board {
        grid: dep::Grid { cols: 2, rows: 3 },
        name: "a",
    };
    let b = dep::Board {
        grid: dep::Grid { cols: 2, rows: 3 },
        name: "b",
    };
    let kept = dep::kept(&a.grid);
    println!(
        "{} {} {:?} {} {}",
        dep::same(&a.grid, &b.grid),
        dep::seen(&[&a.grid, &b.grid]),
        kept,
        kept == b.grid,
        a.name
    );
}
