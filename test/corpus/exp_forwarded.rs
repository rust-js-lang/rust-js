// A type that shows its number's `{:e}` and `{:E}`, `LowerExp::fmt(&self.0,
// f)`, as deranged's ranged integers do: the number's, padded by the
// options it's given.
use std::fmt;

struct Ranged(u8);

impl fmt::LowerExp for Ranged {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerExp::fmt(&self.0, f)
    }
}

impl fmt::UpperExp for Ranged {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::UpperExp::fmt(&self.0, f)
    }
}

struct Measure(f64);

impl fmt::LowerExp for Measure {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fmt::LowerExp::fmt(&self.0, f)
    }
}

fn main() {
    let r = Ranged(200);
    println!("{r:e} {r:E} [{r:>6e}] [{r:<6E}]");
    println!("{:e} {:e}", Ranged(0), Ranged(7));
    println!("{:e} [{:>10e}]", Measure(1234.5), Measure(0.0025));
}
