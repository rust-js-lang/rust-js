// A variable's value a chain of arms gives, one of whose arms a std call
// makes, `main.then_some(root_file)`, as the playground's `prepare` once
// chose what it runs: that arm's `undefined` is placed at the call, before
// `root_file`, so its arms are set where the chain is, in the order it's
// written, `else if (main) { runnable = rootFile; } else { runnable =
// undefined; }`, not turned by where each is placed, `else if (!main)`
// (ADR 0969, amended).
fn runnable<'a>(test: bool, main: bool, tests: Option<&'a str>, root_file: &'a str) -> &'a str {
    let runnable = match test {
        true => tests,
        false => main.then_some(root_file),
    };
    let Some(entry) = runnable else {
        return "nothing";
    };
    entry
}

fn main() {
    for test in [true, false] {
        for main in [true, false] {
            for tests in [Some("lib.test.js"), None] {
                println!("{test} {main} {tests:?}: {}", runnable(test, main, tests, "main.js"));
            }
        }
    }
}
