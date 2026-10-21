use either::{Either, Left, Right};

fn parse(text: &str) -> Either<u32, String> {
    match text.parse() {
        Ok(n) => Left(n),
        Err(_) => Right(text.to_uppercase()),
    }
}

fn evens_or_odds(evens: bool) -> Either<impl DoubleEndedIterator<Item = u32>, impl DoubleEndedIterator<Item = u32>> {
    match evens {
        true => Left((0..10).filter(|n| n % 2 == 0)),
        false => Right((0..10).filter(|n| n % 2 == 1)),
    }
}

pub fn report() -> String {
    let mut lines = Vec::new();
    for text in ["12", "abc", "", "7"] {
        let value = parse(text);
        lines.push(format!(
            "{text:?}: {value:?} left {} right {} {:?} {:?} {}",
            value.is_left(),
            value.is_right(),
            value.clone().left(),
            value.clone().right(),
            value.clone().either(|n| n * 2, |s| s.len() as u32),
        ));
        lines.push(format!(
            "  mapped {:?} flipped {:?} or {:?}",
            value.clone().map_left(|n| n + 1).map_right(|s| s + "!"),
            value.clone().flip(),
            value.clone().left_or(99),
        ));
        lines.push(format!(
            "  as_ref {:?} display {}",
            value.as_ref().map_right(|s| s.len()),
            value
        ));
    }
    lines.push(format!("evens {:?}", evens_or_odds(true).collect::<Vec<_>>()));
    lines.push(format!("odds {:?}", evens_or_odds(false).rev().collect::<Vec<_>>()));
    let both: Vec<Either<u8, char>> = vec![Left(1), Right('x'), Left(3)];
    let (ns, cs): (Vec<u8>, Vec<char>) = itertools_free_partition(both);
    lines.push(format!("partition {ns:?} {cs:?}"));
    let sum: Either<u32, u32> = Left(4);
    lines.push(format!(
        "into_inner {} {:?}",
        sum.into_inner(),
        Right::<u8, u8>(5).either_into::<u32>()
    ));
    lines.join("\n")
}

fn itertools_free_partition(items: Vec<Either<u8, char>>) -> (Vec<u8>, Vec<char>) {
    let mut ns = Vec::new();
    let mut cs = Vec::new();
    for item in items {
        match item {
            Left(n) => ns.push(n),
            Right(c) => cs.push(c),
        }
    }
    (ns, cs)
}
