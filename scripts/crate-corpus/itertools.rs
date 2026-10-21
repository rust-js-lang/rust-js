use itertools::{EitherOrBoth, Itertools, MinMaxResult, Position};

pub fn report() -> String {
    let mut lines = Vec::new();
    let v = vec![3, 1, 4, 1, 5, 9, 2, 6];
    lines.push(format!(
        "positions {:?}",
        v.iter().positions(|&x| x % 2 == 0).collect_vec()
    ));
    lines.push(format!(
        "positions back {:?}",
        v.iter().positions(|&x| x > 3).rev().collect_vec()
    ));
    lines.push(format!(
        "with_position {:?}",
        ["a", "b", "c"]
            .iter()
            .with_position()
            .map(|(p, x)| (p == Position::First, p == Position::Last, *x))
            .collect_vec()
    ));
    lines.push(format!(
        "pad_using {:?}",
        [1, 2].iter().copied().pad_using(4, |i| i * 10).collect_vec()
    ));
    lines.push(format!(
        "pad_using back {:?}",
        [1, 2].iter().copied().pad_using(4, |i| i * 10).rev().collect_vec()
    ));
    lines.push(format!("join {}", v.iter().join(", ")));
    lines.push(format!("format {}", v.iter().format("-")));
    lines.push(format!(
        "format_with {}",
        v.iter().format_with("|", |x, f| f(&format_args!("<{x}>")))
    ));
    lines.push(format!(
        "interleave {:?}",
        [1, 3, 5].iter().interleave([2, 4].iter()).collect_vec()
    ));
    lines.push(format!(
        "intersperse {:?}",
        ["a", "b", "c"].iter().intersperse(&"+").collect_vec()
    ));
    lines.push(format!(
        "merge {:?}",
        [1, 4, 7].iter().merge([2, 3, 8].iter()).collect_vec()
    ));
    lines.push(format!(
        "kmerge {:?}",
        vec![vec![1, 5], vec![2, 3], vec![4]].into_iter().kmerge().collect_vec()
    ));
    lines.push(format!("unique {:?}", v.iter().unique().collect_vec()));
    lines.push(format!(
        "dedup {:?}",
        [1, 1, 2, 2, 2, 3, 1].iter().dedup().collect_vec()
    ));
    lines.push(format!(
        "chunk_by {:?}",
        [1, 3, 2, 4, 5]
            .iter()
            .chunk_by(|x| *x % 2)
            .into_iter()
            .map(|(k, g)| (k, g.copied().collect_vec()))
            .collect_vec()
    ));
    lines.push(format!(
        "tuple_windows {:?}",
        (1..5).tuple_windows::<(_, _)>().collect_vec()
    ));
    lines.push(format!(
        "circular {:?}",
        (1..4).circular_tuple_windows::<(_, _)>().collect_vec()
    ));
    lines.push(format!("tuples {:?}", (1..8).tuples::<(_, _, _)>().collect_vec()));
    lines.push(format!("combinations {:?}", (1..5).combinations(2).collect_vec()));
    lines.push(format!(
        "with_replacement {:?}",
        (1..4).combinations_with_replacement(2).collect_vec()
    ));
    lines.push(format!("permutations {:?}", (1..4).permutations(2).collect_vec()));
    lines.push(format!(
        "cartesian {:?}",
        (1..3).cartesian_product("ab".chars()).collect_vec()
    ));
    lines.push(format!("sorted {:?}", v.iter().sorted().collect_vec()));
    lines.push(format!(
        "sorted_by_key {:?}",
        ["ccc", "a", "bb"].iter().sorted_by_key(|s| s.len()).collect_vec()
    ));
    lines.push(format!(
        "minmax {:?}",
        match v.iter().minmax() {
            MinMaxResult::MinMax(a, b) => (Some(*a), Some(*b)),
            _ => (None, None),
        }
    ));
    lines.push(format!("position_minmax {:?}", v.iter().position_minmax()));
    lines.push(format!(
        "exactly_one {:?} {}",
        [7].iter().exactly_one().ok(),
        [1, 2].iter().exactly_one().is_err()
    ));
    lines.push(format!("collect_tuple {:?}", (1..3).collect_tuple::<(_, _)>()));
    lines.push(format!(
        "counts {:?}",
        "abca".chars().counts().into_iter().sorted().collect_vec()
    ));
    lines.push(format!(
        "all_equal {} {}",
        [1, 1].iter().all_equal(),
        [1, 2].iter().all_equal()
    ));
    lines.push(format!(
        "zip_longest {:?}",
        [1, 2, 3]
            .iter()
            .zip_longest(["x"].iter())
            .map(|e| match e {
                EitherOrBoth::Both(a, b) => format!("{a}{b}"),
                EitherOrBoth::Left(a) => format!("{a}"),
                EitherOrBoth::Right(b) => b.to_string(),
            })
            .collect_vec()
    ));
    let (a, b) = (1..4).tee();
    lines.push(format!("tee {:?} {:?}", a.collect_vec(), b.collect_vec()));
    lines.push(format!("fold1 {:?}", (1..5).reduce(|a, b| a * b)));
    lines.push(format!(
        "into_group_map {:?}",
        vec![(1, 'a'), (2, 'b'), (1, 'c')]
            .into_iter()
            .into_group_map()
            .into_iter()
            .sorted()
            .collect_vec()
    ));
    lines.push(format!(
        "multi_cartesian {:?}",
        vec![0..2, 0..2].into_iter().multi_cartesian_product().collect_vec()
    ));
    lines.push(format!("step_by {:?}", (0..10).step_by(3).collect_vec()));
    lines.push(format!(
        "rev chunks {:?}",
        v.chunks(3).rev().map(|c| c.iter().sum::<i32>()).collect_vec()
    ));
    lines.join("\n")
}
