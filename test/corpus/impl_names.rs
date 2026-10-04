// Impls of one trait whose names would be the same (ADR 0133): for one type
// with other arguments, for a type with other trait arguments, and for two
// types of one name, local to two functions. Each is named apart.

trait Describe {
    fn describe(&self) -> String;
}

impl Describe for Vec<i32> {
    fn describe(&self) -> String {
        format!("{} ints", self.len())
    }
}

impl Describe for Vec<String> {
    fn describe(&self) -> String {
        format!("{} strings", self.len())
    }
}

struct Wrapper<T>(T);

impl Describe for Wrapper<&str> {
    fn describe(&self) -> String {
        format!("a str {}", self.0)
    }
}

impl Describe for Wrapper<u8> {
    fn describe(&self) -> String {
        format!("a byte {}", self.0)
    }
}

trait Convert<T> {
    fn convert(&self) -> String;
}

impl Convert<u8> for () {
    fn convert(&self) -> String {
        "to u8".to_string()
    }
}

impl Convert<&u8> for () {
    fn convert(&self) -> String {
        "to &u8".to_string()
    }
}

fn first() -> String {
    struct Local;
    impl Describe for Local {
        fn describe(&self) -> String {
            "the first local".to_string()
        }
    }
    Local.describe()
}

fn second() -> String {
    struct Local;
    impl Describe for Local {
        fn describe(&self) -> String {
            "the second local".to_string()
        }
    }
    Local.describe()
}

// Two traits of one name for one type, as serde's `de::Error` and
// `ser::Error` are: each named with its trait's module too.
mod de {
    pub trait Error {
        fn custom(msg: &str) -> Self;
    }
}

mod ser {
    pub trait Error {
        fn custom(msg: &str) -> Self;
    }
}

struct Error(String);

impl de::Error for Error {
    fn custom(msg: &str) -> Self {
        Error(format!("de: {msg}"))
    }
}

impl ser::Error for Error {
    fn custom(msg: &str) -> Self {
        Error(format!("ser: {msg}"))
    }
}

fn fail_de<E: de::Error>() -> E {
    E::custom("eof")
}

fn fail_ser<E: ser::Error>() -> E {
    E::custom("key")
}

fn main() {
    println!("{} | {}", vec![1, 2].describe(), vec!["a".to_string()].describe());
    println!("{} | {}", Wrapper("x").describe(), Wrapper(7u8).describe());
    println!("{} | {}", <() as Convert<u8>>::convert(&()), <() as Convert<&u8>>::convert(&()));
    println!("{} | {}", first(), second());
    println!("{} | {}", fail_de::<Error>().0, fail_ser::<Error>().0);
}
