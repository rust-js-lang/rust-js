// `Termination::report` of the types std writes it for, the status a
// `main` of each ends with (#2292): an `ExitCode`'s own, `()`'s `0`, and a
// `Result`'s, of an `Ok` its value's, of an `Err` `1`, `Error: {err:?}`
// written to stderr first; an `ExitCode` compared with another.
use std::process::{ExitCode, Termination};

#[derive(Debug)]
struct MyErr {
    code: u32,
    note: String,
}

fn status(fail: bool) -> Result<(), MyErr> {
    if fail {
        Err(MyErr { code: 3, note: String::from("a \"quoted\"\nline") })
    } else {
        Ok(())
    }
}

fn nested(code: u8) -> Result<Result<ExitCode, String>, u32> {
    match code {
        0 => Err(7),
        1 => Ok(Err(String::from("inner"))),
        n => Ok(Ok(ExitCode::from(n))),
    }
}

fn main() {
    for fail in [true, false] {
        let reported = status(fail).report();
        println!("failed {fail}: {}", reported == ExitCode::FAILURE);
    }
    println!("unit: {}", ().report() == ExitCode::SUCCESS);
    println!("own: {}", ExitCode::from(42).report() == ExitCode::from(42));
    for code in [0, 1, 9] {
        let reported = nested(code).report();
        println!("nested {code}: {} {}", reported == ExitCode::FAILURE, reported == ExitCode::from(9));
    }
    let codes = [ExitCode::SUCCESS, ExitCode::FAILURE, ExitCode::from(0)];
    println!("{} {}", codes[0] == codes[2], codes[0] == codes[1]);
}
