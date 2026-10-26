// `enumerate()` then `fold`: the array's `reduce`, its callback given the
// accumulator, each item and its index, from 0, as `enumerate` counts.

fn weighted(values: &[f64]) -> f64 {
    values
        .iter()
        .enumerate()
        .fold(0.0, |sum, (index, value)| sum + value * index as f64)
}

fn indices_only(words: &[&str]) -> usize {
    words.iter().enumerate().fold(0, |total, (i, _)| total + i)
}

fn labels(words: &[&str]) -> String {
    words.iter().enumerate().fold(String::new(), |mut out, (i, word)| {
        if i > 0 {
            out.push_str(", ");
        }
        out.push_str(&format!("{i}:{word}"));
        out
    })
}

fn of_pairs(points: &[(i32, i32)]) -> i32 {
    points
        .iter()
        .enumerate()
        .fold(0, |acc, (i, &(x, y))| acc + (x - y) * i as i32)
}

fn of_kept(values: &[u32]) -> Vec<(usize, u32)> {
    values
        .iter()
        .filter(|v| **v % 2 == 0)
        .enumerate()
        .fold(Vec::new(), |mut found, (i, &v)| {
            found.push((i, v));
            found
        })
}

fn from_the_end(values: &[u32]) -> u32 {
    values
        .iter()
        .rev()
        .enumerate()
        .fold(0, |acc, (i, v)| acc * 10 + v * i as u32)
}

fn start() -> u32 {
    println!("start");
    100
}

fn started(values: &[u32]) -> u32 {
    values
        .iter()
        .map(|v| {
            println!("item {v}");
            v + 1
        })
        .enumerate()
        .fold(start(), |acc, (i, v)| acc + v * i as u32)
}

fn last_index(words: &[&str]) -> usize {
    words.iter().enumerate().fold(0, |_, (i, _)| i)
}

fn doubled_indices(words: &[&str]) -> Vec<usize> {
    words.iter().enumerate().map(|(i, _)| i * 2).collect()
}

fn main() {
    println!("{}", weighted(&[2.0, 4.0, 0.5]));
    println!("{}", weighted(&[]));
    println!("{}", indices_only(&["a", "b", "c", "d"]));
    println!("{}", labels(&["x", "y", "z"]));
    println!("{}", of_pairs(&[(1, 2), (5, 3), (10, 1)]));
    println!("{:?}", of_kept(&[1, 2, 3, 4, 6]));
    println!("{}", from_the_end(&[1, 2, 3]));
    println!("{}", started(&[1, 2]));
    println!("{} {}", last_index(&["a", "b", "c"]), last_index(&[]));
    println!("{:?}", doubled_indices(&["a", "b", "c"]));
}
