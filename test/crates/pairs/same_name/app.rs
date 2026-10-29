//! Its `main` calls the library's: found in review, the app's was the one
//! renamed, so JS calling `main` found none. Called by its path, the
//! library's is `Dep.main()`; a `use` of it in a body is its import, named
//! around the crate's own `main`.

pub fn main() {
    dep::main();
    greet();
    println!("app's main");
}

fn greet() {
    use dep::main;
    main();
}
