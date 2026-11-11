//@ native: wasm32
//@ library-differs
// A generic block generator's `[u64]` results read as `[u32]`, as rand_core
// 0.5.1's `BlockRng64::next_u32` reads them (#1420): the results are the
// core's associated type, borrowed by its `AsRef<[u64]>`, then their
// slice's pointer read as a `[u32]` slice's, its length kept. Little-endian,
// wasm32's, where native Rust runs it.
pub trait BlockRngCore {
    type Item;
    type Results: AsRef<[Self::Item]> + AsMut<[Self::Item]> + Default;

    fn generate(&mut self, results: &mut Self::Results);
}

pub struct BlockRng64<R: BlockRngCore + ?Sized> {
    results: R::Results,
    index: usize,
    half_used: bool,
    pub core: R,
}

impl<R: BlockRngCore> BlockRng64<R> {
    pub fn new(core: R) -> BlockRng64<R> {
        let results_empty = R::Results::default();
        BlockRng64 {
            core,
            index: results_empty.as_ref().len(),
            half_used: false,
            results: results_empty,
        }
    }
}

impl<R: BlockRngCore<Item = u64>> BlockRng64<R>
where
    <R as BlockRngCore>::Results: AsRef<[u64]> + AsMut<[u64]>,
{
    // rand_core 0.5.1's block.rs:353.
    pub fn next_u32(&mut self) -> u32 {
        let mut index = self.index * 2 - self.half_used as usize;
        if index >= self.results.as_ref().len() * 2 {
            self.core.generate(&mut self.results);
            self.index = 0;
            self.half_used = false;
            index = 0;
        }

        self.half_used = !self.half_used;
        self.index += self.half_used as usize;

        // Index as if this is a u32 slice.
        unsafe {
            let results = &*(self.results.as_ref() as *const [u64] as *const [u32]);
            if cfg!(target_endian = "little") {
                *results.get_unchecked(index)
            } else {
                *results.get_unchecked(index ^ 1)
            }
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        if self.index >= self.results.as_ref().len() {
            self.core.generate(&mut self.results);
            self.index = 0;
        }

        let value = self.results.as_ref()[self.index];
        self.index += 1;
        self.half_used = false;
        value
    }
}

struct Counter {
    next: u64,
}

impl BlockRngCore for Counter {
    type Item = u64;
    type Results = [u64; 4];

    fn generate(&mut self, results: &mut Self::Results) {
        for r in results.iter_mut() {
            *r = self.next << 32 | (self.next + 1);
            self.next += 2;
        }
    }
}

fn main() {
    let mut rng = BlockRng64::new(Counter { next: 1 });
    // As many halves as the view's length, which keeps the results' own:
    // past it, `get_unchecked` is out of bounds.
    let halves: Vec<u32> = (0..4).map(|_| rng.next_u32()).collect();
    println!("{halves:?}");
    println!("{:#x} {:#x} {:#x}", rng.next_u64(), rng.next_u64(), rng.next_u64());
    println!("{} {}", rng.next_u32(), rng.next_u32());
}
