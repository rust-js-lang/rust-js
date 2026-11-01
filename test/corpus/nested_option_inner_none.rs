// What a `Some(None)` of an `Option` of an `Option` gives of its `Some`
// is a `None`, as any other `None` is: kept, compared, looked for and
// shown as one, of numbers, text and structs. The inner `None` given on
// is JS's `undefined`, never the `null` an `Option` held as its payload
// keeps for it (ADR 1017, amended).

#[derive(Debug, PartialEq, Clone)]
struct Point {
    x: i32,
}

fn chosen<T: Copy>(v: Option<T>) -> Option<Option<T>> {
    if v.is_some() { Some(v) } else { Some(None) }
}

fn kept_numbers(values: &[Option<u32>]) -> Vec<Option<u32>> {
    let mut kept = Vec::new();
    for value in values {
        let o = if value.is_some() { Some(*value) } else { Some(None) };
        if let Some(v) = o {
            kept.push(v);
        }
    }
    kept
}

fn kept_text(values: &[Option<&'static str>]) -> Vec<Option<&'static str>> {
    let mut kept = Vec::new();
    for value in values {
        let o = if value.is_some() { Some(*value) } else { Some(None) };
        if let Some(v) = o {
            kept.push(v);
        }
    }
    kept
}

fn kept_points(values: &[Option<Point>]) -> Vec<Option<Point>> {
    let mut kept = Vec::new();
    for value in values {
        let o = if value.is_some() { Some(value) } else { Some(&None) };
        if let Some(v) = o {
            kept.push(v.clone());
        }
    }
    kept
}

struct Chunk {
    fulfilled: bool,
    value: Option<u32>,
}

fn settled(chunks: &[Chunk]) -> Vec<Option<u32>> {
    let mut out = Vec::new();
    for chunk in chunks {
        let value = if chunk.fulfilled { Some(chunk.value) } else { None };
        if let Some(value) = value {
            out.push(value);
        }
    }
    out
}

fn last_seen(values: &[Option<u32>]) -> Option<u32> {
    let mut last = None;
    for value in values {
        last = Some(*value);
    }
    match last {
        Some(inner) => inner,
        None => Some(0),
    }
}

fn main() {
    let numbers = kept_numbers(&[Some(1), None, Some(3)]);
    println!("{numbers:?} {} {}", numbers.contains(&None), numbers.iter().filter(|v| v.is_none()).count());
    let text = kept_text(&[None, Some("a")]);
    println!("{text:?} {} {:?}", text.contains(&None), text.iter().position(|v| v.is_none()));
    let points = kept_points(&[Some(Point { x: 1 }), None]);
    println!("{points:?} {}", points.contains(&None));
    let chunks = [
        Chunk { fulfilled: true, value: None },
        Chunk { fulfilled: false, value: Some(2) },
        Chunk { fulfilled: true, value: Some(4) },
    ];
    let settled = settled(&chunks);
    println!("{settled:?} {}", settled.contains(&None));
    let last = last_seen(&[Some(5), None]);
    println!("{last:?} {}", [last].contains(&None));
    println!("{:?} {:?}", chosen(Some(1)), chosen::<u32>(None));
}
