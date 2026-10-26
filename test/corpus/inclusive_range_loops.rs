// A loop over an inclusive range, `a..=b`, counts up to its end, its end
// included: one to a type's largest value too, which JS's count passes only
// at the test that ends the loop.

fn sum_to(n: u32) -> u64 {
    let mut sum = 0;
    for i in 1..=n {
        sum += u64::from(i);
    }
    sum
}

fn bytes_from(start: u8) -> Vec<u8> {
    let mut seen = Vec::new();
    for b in start..=u8::MAX {
        seen.push(b);
    }
    seen
}

fn last_few(n: u32) -> u32 {
    let mut count = 0;
    for _ in n..=u32::MAX {
        count += 1;
    }
    count
}

fn odd_squares(limit: usize) -> Vec<usize> {
    let mut found = Vec::new();
    for i in 0..=limit {
        if i % 2 == 0 {
            continue;
        }
        if i * i > 50 {
            break;
        }
        found.push(i * i);
    }
    found
}

fn counters(n: i32) -> Vec<Box<dyn Fn() -> i32>> {
    let mut made: Vec<Box<dyn Fn() -> i32>> = Vec::new();
    for i in -2..=n {
        made.push(Box::new(move || i * 10));
    }
    made
}

fn grown(mut seen: Vec<usize>) -> Vec<usize> {
    for i in 0..=seen.len() {
        seen.push(i * 10);
    }
    seen
}

fn pairs(n: u32) -> Vec<(u32, u32)> {
    let mut found = Vec::new();
    'outer: for a in 1..=n {
        for b in a..=n {
            if a * b > 6 {
                continue 'outer;
            }
            found.push((a, b));
        }
    }
    found
}

fn main() {
    println!("{} {} {}", sum_to(0), sum_to(1), sum_to(100));
    println!("{:?}", bytes_from(250));
    println!("{:?}", bytes_from(255));
    println!("{} {}", last_few(u32::MAX - 3), last_few(u32::MAX));
    println!("{:?}", odd_squares(20));
    println!("{:?}", odd_squares(0));
    let made = counters(1);
    println!("{:?}", made.iter().map(|f| f()).collect::<Vec<_>>());
    println!("{}", counters(-5).len());
    println!("{:?}", grown(vec![7, 8]));
    println!("{:?}", pairs(4));
}
