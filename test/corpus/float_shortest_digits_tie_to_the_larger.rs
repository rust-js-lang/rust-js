// An `f64`'s shortest digits that read back as it, where two are equally
// near it: Rust's are the larger, `1888570120608320.3`, in each form that
// writes them, `{}`, `{:?}`, `{:e}` and `{:E}`, where JS's `toString()` and
// `toExponential()` take the even one (#1682).
fn main() {
    let ties = [
        1888570120608320.2_f64,
        -1888570120608320.2,
        2251799813685248.5,
        4503599627370495.5,
        1234567890123456.8,
    ];
    for x in ties {
        println!("{x} {x:?} {x:e} {x:E} {x:+e} {x:>30e}");
    }
    let shown: Vec<f64> = ties.iter().map(|x| x / 1e10).collect();
    println!("{shown:?}");
    for x in &shown {
        println!("{x:e}");
    }
}
