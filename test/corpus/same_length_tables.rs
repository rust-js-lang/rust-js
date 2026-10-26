// A table made as long as another, `vec![0.0; terraces.len()]`, or as a
// count, `vec![0; n]`, and never resized is indexed as the other is
// (#362): javascript-algorithms' `dpRainTerraces`. One resized, in a
// branch too, or whose other grows keeps its check. What Rust does with
// it is what it does natively.

fn rain_water(terraces: &[f64]) -> f64 {
    let mut left_max_levels = vec![0.0; terraces.len()];
    let mut right_max_levels = vec![0.0; terraces.len()];
    let (Some(&first), Some(&last)) = (terraces.first(), terraces.last()) else {
        return 0.0;
    };
    left_max_levels[0] = first;
    for i in 1..terraces.len() {
        left_max_levels[i] = terraces[i].max(left_max_levels[i - 1]);
    }
    right_max_levels[terraces.len() - 1] = last;
    for i in (0..terraces.len() - 1).rev() {
        right_max_levels[i] = terraces[i].max(right_max_levels[i + 1]);
    }
    let mut water = 0.0;
    for (i, &terrace) in terraces.iter().enumerate() {
        let boundary = left_max_levels[i].min(right_max_levels[i]);
        if boundary > terrace {
            water += boundary - terrace;
        }
    }
    water
}

fn prefix_sums(count: usize) -> Vec<u32> {
    let mut sums = vec![0; count];
    let mut running = 0;
    for i in 0..count {
        running += i as u32;
        sums[i] = running;
    }
    sums
}

fn shrunk_sum(values: &[u32], shrink: bool) -> Option<u32> {
    let mut copy = vec![1; values.len()];
    if shrink {
        copy.pop();
    }
    let mut sum = 0;
    for i in 0..values.len() {
        sum += copy.get(i).copied()?;
    }
    Some(sum)
}

fn staircase(stairs: i32) -> f64 {
    let Ok(stairs) = usize::try_from(stairs) else {
        return 0.0;
    };
    let mut steps = vec![0.0; stairs.max(2) + 1];
    steps[1] = 1.0;
    steps[2] = 2.0;
    if stairs <= 2 {
        return steps[stairs];
    }
    for step in 3..=stairs {
        steps[step] = steps[step - 1] + steps[step - 2];
    }
    steps[stairs]
}

fn last_at_most(values: &[u32], i: usize) -> u32 {
    if values.is_empty() {
        return 0;
    }
    values[i.min(values.len() - 1)]
}

fn main() {
    for terraces in [
        &[][..],
        &[1.0],
        &[2.0, 0.0, 2.0],
        &[3.0, 0.0, 0.0, 2.0, 0.0, 4.0],
        &[0.0, 1.0, 0.0, 2.0, 1.0, 0.0, 1.0, 3.0, 2.0, 1.0, 2.0, 1.0],
    ] {
        print!("{} ", rain_water(terraces));
    }
    println!();
    println!("{:?} {:?}", prefix_sums(5), prefix_sums(0));
    println!("{:?} {:?}", shrunk_sum(&[1, 2, 3], false), shrunk_sum(&[1, 2, 3], true));
    println!(
        "{} {} {} {}",
        staircase(-1),
        staircase(2),
        staircase(10),
        last_at_most(&[4, 5], 9)
    );
}
