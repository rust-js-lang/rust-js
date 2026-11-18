// A slice's `position` an `Option`'s `and_then` gives, its `Option` only
// tested and its index taken: JS's own search, `-1` its `None`,
// `extension != null ? DAYS.indexOf(extension) : -1` then `day !== -1`, as
// the vercel-ui SDK's first day of a week reads (#2074); `findIndex` of any
// other test. One that leaves its function stays an `Option`, `undefined`
// where it's `None`; one of an iterator that isn't the slice's own, too,
// `skip(1)`'s or `rev()`'s.

const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

fn by_name(extension: Option<String>) -> u32 {
    if let Some(day) = extension.and_then(|extension| DAYS.iter().position(|&day| day == extension)) {
        return day as u32;
    }
    1
}

fn by_prefix(prefix: Option<&str>) -> Option<u32> {
    let found = prefix.and_then(|prefix| DAYS.iter().position(|day| day.starts_with(prefix)));
    match found {
        Some(day) => Some(day as u32 * 10),
        None => None,
    }
}

fn given_on(extension: Option<&str>) -> Option<usize> {
    extension.and_then(|extension| DAYS.iter().position(|&day| day == extension))
}

fn after_the_first(extension: Option<&str>) -> Option<usize> {
    let found = extension.and_then(|extension| DAYS.iter().skip(1).position(|&day| day == extension));
    found.map(|day| day + 1)
}

fn from_the_end(extension: Option<&str>) -> usize {
    if let Some(back) = extension.and_then(|extension| DAYS.iter().rev().position(|&day| day == extension)) {
        return back + 100;
    }
    0
}

fn main() {
    for name in [Some("sat"), Some("xyz"), None, Some("sun")] {
        println!(
            "{:?} {} {:?} {:?} {:?} {}",
            name,
            by_name(name.map(str::to_string)),
            by_prefix(name.map(|n| &n[..1])),
            given_on(name),
            after_the_first(name),
            from_the_end(name)
        );
    }
}
