// A closure called where it's made, whose callback reads its parameter,
// `extension.and_then(|e| DAYS.iter().position(|&day| day == e))`: put in
// place, `$position(DAYS, (day) => day === extension)`, where what it's
// given is a variable nothing changes, so the callback reads that value
// whenever it runs (the vercel-ui SDK's port, G6). Not where the variable
// changes after, which a callback that runs later would read; nor where
// the callback's own parameter has the variable's word.

const DAYS: [&str; 7] = ["sun", "mon", "tue", "wed", "thu", "fri", "sat"];

fn first_day(tag: &str) -> Option<usize> {
    let extension = tag.split('-').nth(1).map(|part| part.to_lowercase());
    extension.and_then(|e| DAYS.iter().position(|&day| day == e))
}

fn later(mut given: Option<u32>) -> Vec<u32> {
    let read = given.map(|n| move |k: u32| n + k);
    given = Some(100);
    let mut out = Vec::new();
    if let Some(read) = read {
        out.push(read(1));
    }
    out.push(given.unwrap_or(0));
    out
}

fn shadowed(extension: Option<&str>) -> Option<usize> {
    extension.and_then(|e| DAYS.iter().position(|&extension| extension == e))
}

fn main() {
    for tag in ["en-SAT", "en-mon", "en", "en-xyz"] {
        println!("{tag}: {:?}", first_day(tag));
    }
    println!("{:?} {:?}", later(Some(5)), later(None));
    println!("{:?} {:?}", shadowed(Some("tue")), shadowed(None));
}
