// `by_ref()`, and `&mut it`: a `for` loop takes items from an iterator,
// and stops, and what's left is still there, as num-traits' float parser reads a number's digits
// and then its exponent.

fn split_number(text: &str) -> (String, String) {
    let mut cs = text.chars().enumerate();
    let mut digits = String::new();
    for (_, c) in cs.by_ref() {
        if c == 'e' {
            break;
        }
        digits.push(c);
    }
    let rest: String = cs.map(|(i, c)| format!("{i}{c}")).collect();
    (digits, rest)
}

fn split_mut(text: &str) -> (String, usize) {
    let mut cs = text.chars();
    let mut head = String::new();
    for c in &mut cs {
        if c == ':' {
            break;
        }
        head.push(c);
    }
    (head, cs.count())
}

fn main() {
    println!("{:?} {:?}", split_number("12e34"), split_number("7"));
    println!("{:?} {:?}", split_mut("key:value"), split_mut("none"));
}
