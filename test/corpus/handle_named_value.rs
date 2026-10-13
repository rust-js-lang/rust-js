// A `&mut` chosen between two places is a handle on the one chosen (ADR
// 0099), `{ get value() { .. }, set value(newValue) { .. } }`: its setter's
// parameter a name the place doesn't read, which here is `value` itself.
fn main() {
    let mut value = (1u32, 2u32);
    for flag in [true, false] {
        let r = if flag { &mut value.0 } else { &mut value.1 };
        *r += 10;
    }
    println!("{} {}", value.0, value.1);
}
