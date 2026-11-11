//! A consumer collecting into the library's `Summed`, whose `from_iter`
//! holds the iterator it's given in each of six places (#1366).
pub fn main() {
    let _: dep::Summed<0> = dep::collected(dep::count("cell").filter(|_| true));
    let _: dep::Summed<1> = dep::collected(dep::count("box").filter(|_| true));
    let _: dep::Summed<2> = dep::collected(dep::count("option").filter(|_| true));
    let _: dep::Summed<3> = dep::collected(dep::count("field").filter(|_| true));
    let _: dep::Summed<4> = dep::collected(dep::count("array").filter(|_| true));
    let _: dep::Summed<5> = dep::collected(dep::count("tuple").filter(|_| true));
}
