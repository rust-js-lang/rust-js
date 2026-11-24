// `clamp` of `&mut` references, std's `Ord for &mut A`'s default `clamp`:
// what each points at compared, `min <= max` then `self < min` and `self >
// max`, and the chosen `&mut` itself given back, so a write through it
// writes that place (#2191), as `max` and `min` of `&mut`s do (#2188). Its
// bounds the wrong way round panic as the default does, `assertion failed:
// min <= max` at std's line, of a `&mut char` too, never a `char`'s message.
use std::panic;

fn clamped(x: i32, lo: i32, hi: i32) -> (i32, i32, i32) {
    let (mut x, mut lo, mut hi) = (x, lo, hi);
    *(&mut x).clamp(&mut lo, &mut hi) += 100;
    (x, lo, hi)
}

fn main() {
    println!("{:?} {:?} {:?}", clamped(5, 1, 9), clamped(0, 3, 9), clamped(20, 3, 9));
    let (mut s, mut t, mut u) = (String::from("m"), String::from("a"), String::from("z"));
    (&mut s).clamp(&mut t, &mut u).push('!');
    let (mut early, mut late) = (String::from("b"), String::from("y"));
    let mut before = String::from("a");
    (&mut before).clamp(&mut early, &mut late).push('?');
    println!("{s} {t} {u} {before} {early} {late}");
    let (mut c, mut lo, mut hi) = ('q', 'a', 'k');
    *(&mut c).clamp(&mut lo, &mut hi) = 'z';
    println!("{c} {lo} {hi}");
    let crossed = panic::catch_unwind(|| {
        let (mut x, mut lo, mut hi) = (1, 9, 3);
        *(&mut x).clamp(&mut lo, &mut hi)
    });
    let crossed_char = panic::catch_unwind(|| {
        let (mut x, mut lo, mut hi) = ('m', 'z', 'a');
        *(&mut x).clamp(&mut lo, &mut hi)
    });
    let said = |caught: Result<_, Box<dyn std::any::Any + Send>>| match caught {
        Ok(_) => "ok".to_string(),
        Err(payload) => payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_default(),
    };
    println!("{} | {}", said(crossed.map(|_| ())), said(crossed_char.map(|_| ())));
}
