// An enum JS gives, an exported function's argument, never one the crate
// makes: its tag a getter may answer each read anew, so a `match` of it
// reads the tag once, as Rust reads its discriminant once, not once for
// each arm's test, which picked another arm where the getter's second
// answer differs (#2208). One the crate makes is its own data, read as it
// is (ADR 0262's difference 60).
//@ call: kindOf [{"TAG": {"$reads": ["Two", "Neither"]}, "_0": 7, "_1": 8}]
//@ gives: "two 7"
//@ logs: ["get call0.arg0.TAG", "get call0.arg0._0"]
//@ call: kindOf [{"TAG": {"$reads": ["One", "Two"]}, "_0": 6}]
//@ gives: "one 6"
//@ logs: ["get call1.arg0.TAG", "get call1.arg0._0"]
pub enum Given {
    One(u32),
    Two(u32, u32),
    Neither,
}

pub fn kind_of(given: Given) -> String {
    match given {
        Given::One(n) => format!("one {n}"),
        Given::Two(n, _) => format!("two {n}"),
        Given::Neither => String::from("neither"),
    }
}
