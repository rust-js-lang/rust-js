// A cache filled where it misses: `let info = match cache.get(id).cloned()
// { Some(info) => info, None => { ..; made } };` is upstream's `let ioInfo =
// chunkIOInfoCache.get(chunkId); if (ioInfo === undefined) { ..; ioInfo =
// {..}; }`, its one variable read, tested and set where it misses (rd548).
// What the program names, `let found = ..;`, stays its own.
use std::collections::HashMap;

#[derive(Clone, Debug)]
struct Info {
    name: String,
    size: u32,
}

fn info_of(cache: &mut HashMap<String, Info>, id: &str, made: &mut u32) -> Info {
    let info = match cache.get(id).cloned() {
        Some(info) => info,
        None => {
            *made += 1;
            let info = Info {
                name: format!("chunk {id}"),
                size: id.len() as u32,
            };
            cache.insert(id.to_string(), info.clone());
            info
        }
    };
    info
}

// A `match` of `None` first, the test the other way round.
fn size_of(sizes: &HashMap<String, u32>, id: &str) -> u32 {
    let size = match sizes.get(id).copied() {
        None => {
            let doubled = id.len() as u32 * 2;
            doubled + 1
        }
        Some(size) => size,
    };
    size
}

// A name the program wrote stays, `found`, read only by its `match`.
fn named_of(cache: &HashMap<String, Info>, id: &str) -> u32 {
    let found = cache.get(id).cloned();
    let info = match found {
        Some(info) => info,
        None => Info {
            name: id.to_string(),
            size: 3,
        },
    };
    info.size
}

struct Noisy(u32);

impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

// Of what has a destructor, which its `match` drops where it's made.
fn dropped_of(slots: &[Option<u32>], at: usize) -> u32 {
    let noisy = match slots.get(at).copied().flatten().map(Noisy) {
        Some(noisy) => noisy,
        None => Noisy(0),
    };
    noisy.0 + 1
}

// Of a guard's test too, which takes more than its being there: as written.
fn guarded_of(cache: &HashMap<String, Info>, id: &str) -> u32 {
    let info = match cache.get(id).cloned() {
        Some(info) if info.size > 1 => info,
        _ => Info {
            name: id.to_string(),
            size: 2,
        },
    };
    info.size
}

fn main() {
    let mut cache = HashMap::new();
    let mut made = 0;
    for id in ["a", "bb", "a", "ccc", "bb"] {
        let info = info_of(&mut cache, id, &mut made);
        println!("{} {} {}", info.name, info.size, made);
    }
    let sizes = HashMap::from([("a".to_string(), 7)]);
    println!("{} {}", size_of(&sizes, "a"), size_of(&sizes, "bcd"));
    println!("{} {}", named_of(&cache, "a"), named_of(&cache, "zz"));
    let slots = [Some(4), None];
    println!(
        "{} {} {}",
        dropped_of(&slots, 0),
        dropped_of(&slots, 1),
        dropped_of(&slots, 5)
    );
    println!("{} {}", guarded_of(&cache, "a"), guarded_of(&cache, "ccc"));
}
