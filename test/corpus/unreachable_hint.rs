// `hint::unreachable_unchecked()`, which a program that's right never
// reaches, is what MIR's unreachable code is, a throw (ADR 0399).
fn digit(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

fn main() {
    let total: u32 = b"2025".iter().map(|&c| digit(c)).sum();
    println!("{}", total);
}
