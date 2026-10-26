// A `bool`'s `&`, `|` and `^`, and their `op=`, given a `&bool`: std's
// forwarding impls, the same as with a value, as SHA-3's sponge absorbs a
// chunk's bits, `*s ^= c`.
fn absorb(state: &mut [bool], chunk: &[bool]) {
    for (s, c) in state.iter_mut().zip(chunk) {
        *s ^= c;
    }
}

fn main() {
    let mut state = [false, true, true, false];
    absorb(&mut state, &[true, true, false, false]);
    let chunk = [true, true, false];
    let mut all = true;
    let mut any = false;
    for c in &chunk {
        all &= c;
        any |= c;
    }
    let (a, b) = (&true, &false);
    println!("{state:?} {all} {any} {} {} {}", a ^ b, a & b, a | b);
}
