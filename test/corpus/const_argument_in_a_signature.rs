// A const argument in a signature, `FixedOutput<'_, 10>`, and a type
// parameter's default after a const parameter: constants rustc evaluates,
// which JS never runs. Declared and never built, as in rustc's
// const-generics/defaults/simple-defaults.rs, `10` is left unlowered, which
// the analysis of what calls reach a function mustn't typecheck as a body
// (ADR 0577), so their dead code is allowed.
#[allow(dead_code)]
struct FixedOutput<'a, const N: usize, T = u32> {
    out: &'a [T; N],
}

#[allow(dead_code)]
trait FixedOutputter {
    fn out(&self) -> FixedOutput<'_, 10>;
}

fn main() {
    println!("{}", std::mem::size_of::<FixedOutput<'static, 3>>() > 0);
}
