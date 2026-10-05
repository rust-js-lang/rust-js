// A module in a block is one of its own, though another block of the same
// module has one of its name, as bitflags' macro writes one in a function and
// another in a constant: and a module the parent declares keeps its name.

fn first() -> &'static str {
    mod names {
        pub const NAME: &str = "first";
    }
    names::NAME
}

fn second() -> &'static str {
    mod names {
        pub fn name() -> &'static str {
            "second"
        }
    }
    names::name()
}

const ALL: &[&str] = {
    mod names {
        pub const NAME: &str = "constant";
    }
    &[names::NAME, "and more"]
};

// Declared after the blocks' modules, it keeps its name.
mod names {
    pub const NAME: &str = "outer";
}

fn main() {
    println!("{} {} {} {:?}", names::NAME, first(), second(), ALL);
}
