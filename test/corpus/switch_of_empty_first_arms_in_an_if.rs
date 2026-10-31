// rd133: a `match` of strings in an `if`, whose first arm does nothing, is
// its `switch`, React DOM's updateProperties' `switch (propKey) { case
// 'checked': break; case 'value': // .. break; case 'defaultValue': .. }`,
// not that arm's test joined to the `if`'s, `&& key !== "checked"`, and
// the rest an `if` chain. Where no `switch` is made, a guarded arm's,
// the tests are one `if`'s still.

fn updated(keys: &[&str], next: &[&str], log: &mut Vec<String>) {
    for &key in keys {
        if !key.is_empty() && !key.starts_with('_') {
            match key {
                "checked" => {}
                "value" => {
                    // Handled below.
                }
                "defaultValue" => {
                    log.push("default".to_string());
                    if !next.contains(&key) {
                        log.push(key.to_string());
                    }
                }
                _ => {
                    if !next.contains(&key) {
                        log.push(key.to_string());
                    }
                }
            }
        }
    }
}

fn extra(names: &[&str], transitions: bool) -> Vec<String> {
    let mut found = Vec::new();
    for &name in names {
        match name {
            "value" => {}
            "checked" => {}
            "vt-name" | "vt-update" if transitions => {
                // Expected from the server.
            }
            _ => found.push(name.to_string()),
        }
    }
    found
}

fn main() {
    let mut log = Vec::new();
    updated(&["checked", "value", "defaultValue", "title", "", "_x", "id"], &["id"], &mut log);
    println!("{log:?}");
    let names = ["value", "checked", "vt-name", "vt-update", "class"];
    println!("{:?} {:?}", extra(&names, true), extra(&names, false));
}
