//@ compile-fail: `{:?}` of an `ExitCode` yet: each platform's std writes its own
// `{:?}` of an `ExitCode` is each platform's std's own text,
// `ExitCode(unix_exit_status(3))` of a unix's, which no JS can say: refused
// (#2292; ADR 0262's last case), where its number would be wrong.
use std::process::ExitCode;

fn main() {
    println!("{:?}", ExitCode::from(3));
}
