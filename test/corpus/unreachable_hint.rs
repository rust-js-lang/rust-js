// `hint::unreachable_unchecked()`, which a program that's right never
// reaches, is what MIR's unreachable code is: a way to it alone is left
// out, its test with it, and the last arm tested is what's left (ADR
// 0399, amended). `unreachable!()` is a panic, which throws.
enum Body {
    Text(String),
    Params(u32),
    Form(Vec<u8>),
}

fn size(body: &Body) -> usize {
    match body {
        Body::Text(text) => text.len(),
        Body::Params(_) => unsafe { std::hint::unreachable_unchecked() },
        Body::Form(bytes) => bytes.len() * 2,
    }
}

fn named(body: &Body) -> &'static str {
    match body {
        Body::Text(_) => "text",
        Body::Params(_) => unreachable!("no params"),
        Body::Form(_) => "form",
    }
}

fn digit(c: u8) -> u32 {
    match c {
        b'0'..=b'9' => (c - b'0') as u32,
        _ => unsafe { std::hint::unreachable_unchecked() },
    }
}

fn loud(n: u32) -> bool {
    println!("loud {n}");
    n > 100
}

// The test goes, but what it calls is still called.
fn checked(n: u32) -> u32 {
    if loud(n) {
        unsafe { std::hint::unreachable_unchecked() }
    }
    n + 1
}

// Where it isn't a way a test leaves out, it throws, as MIR's unreachable
// code does.
fn never() -> u32 {
    unsafe { std::hint::unreachable_unchecked() }
}

fn main() {
    let bodies = [Body::Text("abc".to_string()), Body::Form(vec![1, 2])];
    for body in &bodies {
        println!("{} {}", named(body), size(body));
    }
    let total: u32 = b"2025".iter().map(|&c| digit(c)).sum();
    println!("{} {}", total, checked(3));
    if total > 10_000 {
        println!("{}", never());
    }
}
