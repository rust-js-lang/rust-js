// A std function that's JS's `Math`'s, lent as a `&dyn Fn` (ADR 0887,
// #1812): `&f64::ln` is `Math.log`, as `f64::ln` taken as a `fn` is, read
// where it's lent. A scale whose mapping is chosen where it's made, as a
// port of vercel-ui's number scale keeps `Math.log`. One that isn't
// `Math`'s for every value, `&f64::round` or an `f32`'s `&f32::sqrt`, stays
// the arrow of its call. What a math library computes is shown to 12
// places: its last bit is the platform's, natively.

struct Scale {
    to: &'static dyn Fn(f64) -> f64,
    from: &'static dyn Fn(f64) -> f64,
}

thread_local! {
    static LOG: Scale = Scale { to: &f64::ln, from: &f64::exp };
    static LINEAR: Scale = Scale { to: &f64::abs, from: &f64::abs };
}

fn position(scale: &Scale, value: f64, min: f64, max: f64) -> f64 {
    let (to, from) = (scale.to, scale.from);
    let at = (to(value) - to(min)) / (to(max) - to(min));
    from(to(min) + at * (to(max) - to(min)))
}

fn shown(name: &str, f: &dyn Fn(f64) -> f64) {
    let shown: Vec<String> = [-2.5, -0.5, -0.0, 0.0, 0.5, 1.5, 7.0, f64::INFINITY, f64::NAN]
        .iter()
        .map(|&x| format!("{:?}", f(x)))
        .collect();
    println!("{name}: {}", shown.join(" "));
}

fn main() {
    for value in [1.0, 10.0, 100.0, 1000.0, 0.0, -1.0, f64::NAN] {
        let log = LOG.with(|scale| position(scale, value, 1.0, 1000.0));
        let linear = LINEAR.with(|scale| position(scale, value, 0.0, 1000.0));
        println!("{value:?}: {log:.12} {linear:?}");
    }

    // Exact, `Math`'s; not `Math`'s for every value, their arrows.
    shown("floor", &f64::floor);
    shown("trunc", &f64::trunc);
    shown("sqrt", &f64::sqrt);
    shown("round", &f64::round);
    shown("signum", &f64::signum);

    // Of two numbers, given in order; handed to an iterator, given no more
    // than Rust gives it.
    let pairs: [(&str, &dyn Fn(f64, f64) -> f64); 2] = [("hypot", &f64::hypot), ("atan2", &f64::atan2)];
    for (name, f) in pairs {
        let folded = [3.0, 4.0, 12.0].into_iter().fold(1.0, f);
        println!("{name}: {:.12} {folded:.12}", f(-0.0, 1.0));
    }

    // An integer's, and an `f32`'s, rounded to one.
    let leading: &dyn Fn(u32) -> u32 = &u32::leading_zeros;
    let narrow: &dyn Fn(u8) -> u32 = &u8::leading_zeros;
    let single: &dyn Fn(f32) -> f32 = &f32::sqrt;
    let exact: &dyn Fn(f32) -> f32 = &f32::floor;
    println!(
        "{} {} {} {} {:?} {:?}",
        leading(0),
        leading(1),
        narrow(1),
        narrow(0),
        f64::from(single(2.0)),
        f64::from(exact(-1.5))
    );
}
