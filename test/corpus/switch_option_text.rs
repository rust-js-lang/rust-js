// A `match` of an `Option`'s text, React's `resolveThenable`: a `switch` of
// its value, whose `None` no case takes, and a last `_`'s statements its
// `default`, what follows the `match` after it (ADR 0629, amended).
struct Thenable {
    status: Option<String>,
    log: Vec<String>,
}

impl Thenable {
    fn settle(&mut self) {
        if self.status.is_none() {
            self.status = Some("pending".to_string());
        } else {
            self.log.push("seen".to_string());
        }
    }
}

fn resolve(thenable: &mut Thenable) -> Result<u32, String> {
    match thenable.status.as_deref() {
        Some("fulfilled") => return Ok(1),
        Some("rejected") => return Err("rejected".to_string()),
        _ => {
            thenable.settle();

            // Check one more time.
            match thenable.status.as_deref() {
                Some("fulfilled") => return Ok(2),
                Some("rejected") => return Err("late".to_string()),
                _ => {}
            }
        }
    }
    thenable.log.push("thrown".to_string());
    Err("pending".to_string())
}

fn code(status: Option<&str>) -> u32 {
    match status {
        Some("fulfilled") => 1,
        Some("rejected") => 2,
        _ => {
            let unknown = status.map_or(0, str::len) as u32;
            unknown + 10
        }
    }
}

fn main() {
    for status in [None, Some("fulfilled"), Some("rejected"), Some("pending"), Some("other")] {
        let mut thenable = Thenable {
            status: status.map(str::to_string),
            log: Vec::new(),
        };
        let resolved = resolve(&mut thenable);
        println!("{status:?}: {resolved:?} {:?} {:?}", thenable.status, thenable.log);
        println!("{}", code(status));
    }
}
