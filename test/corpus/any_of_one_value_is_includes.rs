// `any` whose closure only compares its item with one value, as React's
// `addTransitionType` asks of a `Vec<String>` and a `&str`, is JS's search
// of the array, `includes`, where what's compared is never a NaN, as
// Rust's types tell: text, integers, `char`s and `bool`s, whose `===`
// `includes` finds alike (rd353, ADR 1016, amended). A float's stays the
// `some` it is: `includes` finds a NaN, which `==` never does.

fn has_type(types: &[String], ty: &str) -> bool {
    types.iter().any(|t| t == ty)
}

fn has_level(levels: &[u32], level: u32) -> bool {
    levels.iter().any(|&l| l == level)
}

fn has_char(chars: &[char], c: char) -> bool {
    chars.iter().any(|&each| each == c)
}

fn has_flag(flags: &[bool], flag: bool) -> bool {
    flags.iter().any(|&f| f == flag)
}

fn has_float(xs: &[f64], x: f64) -> bool {
    xs.iter().any(|&each| each == x)
}

fn main() {
    let types = vec![String::from("navigation"), String::from("reload")];
    let ty = "reload";
    println!("{} {}", has_type(&types, ty), !has_type(&types, "push"));
    println!("{} {}", has_level(&[1, 2, 3], 2), has_level(&[], 2));
    println!("{} {}", has_char(&['a', 'é'], 'é'), has_char(&['a'], 'b'));
    println!("{} {}", has_flag(&[false], true), has_flag(&[true, false], false));
    let nan = f64::NAN;
    println!("{} {}", has_float(&[1.0, nan], nan), has_float(&[0.0], -0.0));
}
