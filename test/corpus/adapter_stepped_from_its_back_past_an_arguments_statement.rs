// An adapter made in place and stepped from its back, by `nth_back`,
// `rfold` or `rfind`, whose argument runs a statement of its own first:
// MIR holds the adapter in a variable across that statement, lent to its
// step or given whole, so the step can't read its form where it's made.
// It's made knowing its back there, `$mapEnds` say, as a counted adapter
// is. One its step reads in place, `next_back()` of a map, is as it was:
// its source's back, mapped. A map, a filter, an enumeration and a copy
// of numbers and of items with a destructor (#811).
struct D(u32);

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn ds() -> Vec<D> {
    (1..=5).map(D).collect()
}

fn main() {
    let mut k = 1;
    let u = vec![1, 2, 3];
    println!(
        "{:?}",
        u.into_iter().map(|x| x * 10).nth_back({
            k += 1;
            k - 2
        })
    );
    let found = ds().into_iter().map(|d| d.0 * 10).nth_back({
        k += 1;
        k - 2
    });
    println!("{found:?}");
    let folded = vec![1u32, 2, 3, 4].into_iter().map(|x| x + 1).rfold(
        {
            k += 1;
            k as u32
        },
        |n, x| n * 10 + x,
    );
    println!("{folded}");
    let odd = ds().into_iter().filter(|d| d.0 % 2 == 1).map(|d| d.0).nth_back({
        k -= 1;
        k - 2
    });
    println!("{odd:?}");
    let pair = vec![7u32, 8, 9].into_iter().enumerate().nth_back({
        k += 1;
        k - 4
    });
    println!("{pair:?}");
    let copy = [4u32, 5, 6].iter().copied().map(|x| x + 1).rfind(|x| {
        k += 1;
        *x < 7
    });
    println!("{copy:?}");
    let v = vec![1, 2, 3];
    println!("{:?}", v.iter().map(|x| x * 10).next_back());
    println!("{k}");
}
