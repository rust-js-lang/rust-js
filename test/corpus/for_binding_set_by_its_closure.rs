// A `for`'s binding set by a closure its body makes, or through a `&mut`
// of it given to a function: a `let` each turn, as Rust's `mut` binding
// is, not a `const` the closure's assignment throws at (#663).

fn bump(n: &mut i32) {
    *n += 100;
}

fn main() {
    let rows = vec![1, 2, 3];
    for mut row in rows {
        let mut add = || row += 10;
        add();
        println!("{row}");
    }
    let mut total = 0;
    for mut n in [4, 5] {
        let mut twice = || n *= 2;
        twice();
        twice();
        total += n;
    }
    println!("{total}");
    for mut item in [7, 8] {
        bump(&mut item);
        println!("{item}");
    }
}
