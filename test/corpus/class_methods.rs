// A struct and its impl marked `#[rust_js::class]` are a JS class (ADR
// 0761): its constructor `new Counter(..)`, its methods the prototype's,
// called on the value, `counter.tick()`, reading and changing `this`; its
// constants the prototype's, its functions of no `self` the constructor's.
// What Rust does with it is what it does natively.

#[cfg_attr(rust_js, rust_js::class)]
pub struct Counter {
    count: u32,
    step: u32,
    history: Vec<u32>,
}

#[cfg_attr(rust_js, rust_js::class)]
impl Counter {
    pub const LIMIT: u32 = 10;

    pub fn new(step: u32) -> Self {
        if step == 0 {
            return Counter {
                count: 0,
                step: 1,
                history: Vec::new(),
            };
        }
        Counter {
            count: 0,
            step,
            history: vec![0],
        }
    }

    pub fn starting() -> u32 {
        Self::LIMIT / 2
    }

    pub fn tick(&mut self) -> bool {
        if self.count + self.step > Self::LIMIT {
            return false;
        }
        self.count += self.step;
        self.history.push(self.count);
        true
    }

    pub fn current(&self) -> u32 {
        self.count
    }

    pub fn summary(&self) -> String {
        let shown: Vec<String> = self.history.iter().map(|n| n.to_string()).collect();
        format!("{} after [{}]", self.count, shown.join(", "))
    }

    pub fn reset(&mut self, to: u32) {
        let before = self.current();
        self.count = to;
        self.history.push(before);
    }
}

fn run(counter: &mut Counter, times: u32) -> u32 {
    (0..times).filter(|_| counter.tick()).count() as u32
}

fn main() {
    let mut counter = Counter::new(3);
    println!("{}", run(&mut counter, 5));
    println!("{}", counter.summary());
    counter.reset(Counter::starting());
    println!("{} {}", counter.current(), counter.summary());
    let mut ones = Counter::new(0);
    while ones.tick() {}
    let counters = [counter, ones];
    let counts: Vec<u32> = counters.iter().map(Counter::current).collect();
    println!("{counts:?} {}", Counter::LIMIT);
}
