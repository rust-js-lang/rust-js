// `{:.2e}`, an exponent with a precision: that many digits after the first,
// of the exact value rounded half to even, as Rust rounds a float's and an
// integer's; through a type's own `LowerExp` too (ADRs 0058, 0165).
use std::fmt;

struct Ranged(u8);

impl fmt::LowerExp for Ranged {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerExp::fmt(&self.0, f)
    }
}

fn main() {
    println!("{:.1e} {:.1e} {:.1e} {:.0e} {:.2e} {:.3E}", 1.25f64, 1.35f64, 0.125f64, 2.5f64, 1234.5f64, -0.000123456f64);
    println!("{:.1e} {:.1e} {:.0e} {:.2e} {:.1e} {:.4e}", 125u32, 135u32, 25u8, 999u32, 0u8, 7i64);
    println!("{:.2e} {:.1e} {:.3e} [{:>12.2e}] {:.0e}", 9.995f64, 9.96f64, f64::MAX, 1.5f32, -0.0f64);
    println!("{:.3e} {:.2e}", u64::MAX, i64::MIN);
    let digits = 3;
    println!("{:.*e} {:.prec$E}", digits, 2.0f64 / 3.0, 1.0f32 / 3.0, prec = 1);
    let r = Ranged(250);
    println!("{r:.1e} [{r:>8.0e}] {r:e}");
}
