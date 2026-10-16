// `env::var` and `env::vars` are the process's environment, Node's, which a
// browser hasn't (ADR 0391): a variable missing is `Err(NotPresent)`, shown
// as std shows it; JS's are text, so a `NotUnicode` is only one made so.
// Their `_os` forms are the same text (ADR 0173): `var_os` an `Option`.
use std::env::{self, VarError};
use std::ffi::OsString;

fn main() {
    println!("{}", env::var("PATH").is_ok());
    let missing = env::var("RUST_JS_NO_SUCH_VARIABLE");
    println!("{missing:?}");
    if let Err(e) = &missing {
        println!("{e}");
    }
    let made = VarError::NotUnicode(OsString::from("bytes"));
    println!("{made} {made:?}");
    println!("{}", env::vars().any(|(key, _)| key == "PATH"));
    let path = env::vars().find(|(key, _)| key == "PATH").map(|(_, value)| value);
    println!("{}", path == env::var("PATH").ok());
    let all: Vec<(String, String)> = env::vars().collect();
    println!("{}", all.len() == env::vars().count());
    let mut each = env::vars();
    let first = each.next();
    println!("{}", first.is_some() && each.count() + 1 == all.len());
    println!("{:?} {}", env::var_os("RUST_JS_NO_SUCH_VARIABLE"), env::var_os("PATH").is_some());
    let os: Vec<(OsString, OsString)> = env::vars_os().collect();
    println!("{}", os.len() == all.len());
    println!("{}", env::var_os("PATH").is_some() == env::var("PATH").is_ok());
}
