//@ library-differs
// A type's associated constant whose value is a module's `const` declared
// after it, getrandom 0.1's `impl Error { pub const UNKNOWN: Error =
// UNSUPPORTED; }` with `const UNSUPPORTED: Error = ..` below: the type's
// object, which holds it, is made after what it reads. It was made first,
// and loading the module threw, "Cannot access 'UNSUPPORTED' before
// initialization", where Rust runs.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Code(u32);

impl Code {
    pub const UNKNOWN: Code = UNSUPPORTED;
    pub const BASE: u32 = 1 << 31;

    pub fn raw(self) -> u32 {
        self.0
    }
}

const UNSUPPORTED: Code = Code(Code::BASE + 4);

fn main() {
    println!("{:?} {}", Code::UNKNOWN, Code::UNKNOWN.raw());
    println!("{}", UNSUPPORTED == Code::UNKNOWN);
}
