// A struct an aggregate makes holding a `&mut` to a function a type
// parameter is, `Held(&mut f)`, called through its field: whether what the
// struct's parameter is given is only called is asked of a function alone,
// a struct having no signature, which rustc's `fn_sig` crashed on (#2061).
struct Held<T>(T);

fn applied<F: FnMut(u32) -> u32>(mut f: F) -> u32 {
    let held = Held(&mut f);
    (held.0)(1) + (held.0)(2)
}

fn main() {
    let mut calls = 0;
    let total = applied(|x| {
        calls += 1;
        x * 10
    });
    println!("{total} {calls}");
}
