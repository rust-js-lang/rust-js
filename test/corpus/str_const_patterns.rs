// A `match` on text whose patterns are `&str` constants names them, as
// it's written (ADR 0590, amended): React Flight's `match status { Some(INITIALIZED)
// => .., }` is upstream's `case INITIALIZED:`, not its text, `case
// "fulfilled":`. A constant's text is never `null`, so its test says the
// `Option` is there, as a literal's does: the `default` stays in the
// `switch`, and no `status != null &&` is written before it. A `==` the
// program writes in an arm of the text keeps its literal, and a value
// written two ways in one `match` is its text. Each is its literal to
// tidy, so an or-pattern's tests join, `value === A || value === B`, and
// is named once tidy's done; another module's is imported.

const INITIALIZED: &str = "fulfilled";
const ERRORED: &str = "rejected";

mod states {
    pub const PENDING: &str = "pending";
}

fn plain(status: &str) -> u32 {
    match status {
        INITIALIZED => 1,
        ERRORED => 2,
        _ => 0,
    }
}

fn wrapped(status: Option<&str>) -> u32 {
    match status {
        Some(INITIALIZED) => 1,
        Some(ERRORED) => 2,
        _ => 0,
    }
}

fn owned(status: Option<String>) -> u32 {
    match status.as_deref() {
        Some(INITIALIZED) => 1,
        _ => 0,
    }
}

fn inner(status: &str, other: &str) -> u32 {
    match status {
        INITIALIZED => {
            if other == "fulfilled" {
                1
            } else {
                2
            }
        }
        _ => 0,
    }
}

fn mixed(status: Option<&str>) -> u32 {
    match status {
        Some(INITIALIZED) => 1,
        Some("rejected") | None => 2,
        _ => 0,
    }
}

fn twice(status: &str, first: bool) -> u32 {
    match status {
        "fulfilled" if first => 1,
        INITIALIZED => 2,
        _ => 0,
    }
}

fn tested(status: &str) -> bool {
    matches!(status, INITIALIZED | ERRORED)
}

fn waiting(status: Option<&str>) -> bool {
    matches!(status, Some(states::PENDING))
}

fn let_else(status: Option<&str>) -> u32 {
    let Some(INITIALIZED) = status else {
        return 0;
    };
    1
}

fn main() {
    for status in ["fulfilled", "rejected", "pending"] {
        println!(
            "{} {} {} {} {}",
            plain(status),
            wrapped(Some(status)),
            owned(Some(status.to_string())),
            inner(status, status),
            inner(status, "other"),
        );
        println!(
            "{} {} {} {} {}",
            mixed(Some(status)),
            tested(status),
            let_else(Some(status)),
            twice(status, true),
            twice(status, false),
        );
        println!("{}", waiting(Some(status)));
    }
    println!(
        "{} {} {} {} {}",
        wrapped(None),
        owned(None),
        mixed(None),
        let_else(None),
        waiting(None),
    );
}
