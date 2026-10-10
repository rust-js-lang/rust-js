// A width or a precision another argument gives: `{:>w$}`, `{:.*}`, `{:<1$}`
// and `{:^width$.prec$}`, each read where its placeholder is.
fn main() {
    let w = 6;
    let p = 2;
    println!(
        "[{:>w$}] [{:.*}] [{:<1$}] [{:^width$.prec$}]",
        "ab",
        p,
        1.23456,
        7,
        3.14159,
        width = w + 1,
        prec = p
    );
}
