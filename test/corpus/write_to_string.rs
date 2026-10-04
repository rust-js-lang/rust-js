// `write!` and `writeln!` into a `String` add to it, as `push_str` does;
// writing to a string can't fail, so the `fmt::Result` is always `Ok`.
use std::fmt::Write;

fn render(items: &[(&str, u32)]) -> String {
    let mut out = String::new();
    for (name, n) in items {
        writeln!(out, "{name:<6}{n:>3}").unwrap();
    }
    write!(&mut out, "total {}", items.iter().map(|(_, n)| n).sum::<u32>()).unwrap();
    out
}

fn table(out: &mut String, rows: u32) -> std::fmt::Result {
    for i in 0..rows {
        writeln!(out, "row {i}")?;
    }
    out.write_str("end")?;
    out.write_char('.')?;
    Ok(())
}

fn main() {
    println!("{}", render(&[("pen", 3), ("ink", 12)]));
    let mut s = String::new();
    table(&mut s, 2).unwrap();
    println!("{s}");
    let mut t = String::from("x");
    let _ = write!(t, "{}", 1);
    let ok = write!(t, "!").is_ok();
    println!("{t} {ok}");
}
