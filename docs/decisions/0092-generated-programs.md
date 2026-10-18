# 0092. Generated programs, each from a seed, and reduced when they fail

Status: Accepted. Extends [0088](0088-corpus.md).

Case: N ([0262](0262-when-rust-and-js-disagree.md)).

## Context

The corpus and rustc's own tests are programs someone wrote, each about
what its writer thought of. What rust-js gets wrong where no one thought to
look, they can't find. The research's step 6 asks for bounded generated
programs, with their seeds recorded and a failure reduced to a small case
([research](../research/compiler-testing.md)).

## Decision

**`test/generate.ts` makes a Rust program from a seed,** the same one each
time, valid and deterministic by construction:

- Integers of every width rust-js has, `i8` to `u64`, and `bool`: literals
  at the edges (0, ±1, the type's bounds, 2^53 + 1), operators, shifts by
  any integer type, casts, wrapping, saturating, `abs`, `count_ones` and
  the like, comparisons, and `if` as an expression; `let`, assignment and
  compound assignment, `if`, bounded `for` loops, and `println!` with `{}`,
  `{:?}`, `{:x}`, `{:#x}` and widths. `usize` isn't generated, as it's a
  known difference (ADR 0090).
- `Vec`s of them: `vec![..]`, `push`, `pop`, `sort`, `reverse`, an item
  read or written, which may not be there and panic, `len`, `contains`,
  `sum`, `max`, and `map` and `filter` with closures; `for` over one.
  `Option`s: `Some`, `None`, `checked_add` and the like, `first`,
  `unwrap_or`, `map`, `is_some`, and `if let Some(x)`. A `Copy` struct of
  three widths, made, compared, read and written. Closures, `move`, of
  what's `Copy`, called.
- Text where UTF-8 and UTF-16 part: `String`s and `char`s of accents, `ß`,
  CJK and an emoji, upper- and lower-cased, trimmed, replaced, reversed,
  counted in `char`s and by `split`, compared, pushed to, and padded, as
  `{:>8}` counts `char`s; a `char`'s questions, `as u32` and `from_digit`.
- An enum, `E { A, B(i32), C { x: u8, y: bool } }`, made, compared, and
  `match`ed, its arms binding what the variant holds, some with a guard;
  `matches!` and `if let E::B(n)`.
- A `BTreeMap<u8, i32>`, whose order is its keys', not a `HashMap`'s, which
  changes from run to run natively: `insert`, `remove`, `entry(..)
  .or_insert(0) +=`, `get`, `contains_key`, `len`, `values().sum()`, and
  `for` over it.
- Writes inside expressions, `({ x = a; b })`, where any expression may
  be, and an item written through an index that writes first, `v[{ x =
  a; id(0usize) }] += b`: what's read before and after a write says in
  which order an expression's parts run, which Rust's reference fixes and
  JS's rules differ from, in an assignment's place first. A write becomes
  a statement of its own in JS, which runs where it should, so calls with
  effects too, which stay inside an expression: `note(3, a)` prints its
  tag and gives `a`, and `bump(&mut s)` changes `s.a` and gives it,
  as in `s.a += bump(&mut s)`; an index may be `note`'s, and the value
  written through an index that writes may be what it wrote.
- A closure that captures a variable by reference and writes it, called
  where that variable, or another, is assigned, in a block of its own:
  `{ let mut g = || { x = a; b }; x += g(); }`, as `x += g()` reads `x`
  after the call. It captures only `x`, so nothing else it reads is
  borrowed while it lives.
- Drops, and when: `D(u32)`, whose `Drop` prints its tag. One held, `let
  mut d3 = D(3);`, is dropped where its block ends, a loop's body each
  time round, in the reverse of the order they were made; one not, `let _
  = D(3);`, at once; one replaced, `d3 = D(4);`, after the new one is made;
  one moved, `drop(d3)`, `take(d3)` or `let d5 = d3;`, where it goes; and
  a temporary read, `D(7).0`, as its statement ends, a `println!` after it
  prints. Only what a block made does it move, so nothing is moved twice
  or read after (2026-10-14).
- Iterators' chains, `v.into_iter()`, reversed or not, through up to
  three of `map`, `filter`, `take` and `skip`, to an end: `collect`,
  `sum`, `fold`, `count`, `max`, `min`, `last`, `next`, `any` or `all`.
  Their closures are `move`, each a copy of what it reads, which a write
  after it's made doesn't change, as a JS closure's variable would; and
  generated expressions, which may `note`: Rust's
  iterators are lazy, each item through every step before the next, and
  `next`, `any`, `all` and `take` stop early, which an array's eager
  `map` and `filter` wouldn't (2026-10-14).
- Control flow: a bounded `while`, its test read each time round; loops
  one in another, the outer labeled, left or gone round again from the
  inner, `break 'l` and `continue 'l`; `loop` as an expression, `break
  value` where its test holds; and a `match` of an integer by values,
  `1 | 2`, a range `3..=9`, or a binding with a guard (2026-10-14).
- Methods of `S`: `s.get()`; `s.add(n)`, of `&mut self`, its argument
  reading `s`, as Rust's two-phase borrow lets it, never writing it; and
  `s.scaled(k).a`, of `self`, copied before its argument is made, which
  may write `s`: JS gives a function an object, not a copy (2026-10-15).
  A chain that ends in `collect` has no `skip`, where std's `Vec` of a
  chain it knows is empty runs none of it (ADR 0055's B).
- Functions `main` declares, of two integers, giving an `Option` or a
  `Result<T, String>`: steps `let t = a.checked_add(b)?;`, `ok_or(..)?`
  of a `Result`'s, each of which may give up, and `if c { return .. }`,
  then their value; called with arguments in order, an `Option`'s value
  as it is, a `Result`'s `unwrap_or(d)` or `is_ok()`. What they note says
  which steps ran before one gave up (2026-10-15).
- A trait, `Shape`, of an `area`, a `name` by default that `Rect` has its
  own of, and a `grow(&mut self)`, which `Sq` and `Rect` have: shapes as
  `Vec<Box<dyn Shape>>`, made, pushed to and each grown through `&mut dyn
  Shape`, their areas summed by each one's own `area`, one `describe`d by
  `&dyn Shape`; and `total(&[Sq(a), Sq(b)])`, a generic function made for
  `Sq` (2026-10-15).
- Floats, `f64` and `f32`, at the values where JS and Rust most often
  part: `-0.0`, `0.1`, halves, `1e21` and `1e-7`, where JS's text turns to
  an exponent, `2^53 + 1`, the smallest and largest, `NaN` and the
  infinities. Arithmetic and `%`, `abs`, `floor`, `ceil`, `round`,
  `trunc`, `sqrt`, `signum`, `fract`, `powi`, `min` and `max`; casts from
  integers, between the two, and to integers, which saturate, `NaN` 0;
  comparisons, `is_nan` and the like; and shown `{}`, `{:?}`, `{:.2}`,
  `{:.0}`, which rounds a half to even, `{:e}`, `{:10.3}` and `{:+}`. Not
  `exp`, `sin` and the like, which JS's `Math` needn't round as Rust does
  (2026-10-15).
- Shared state, `Rc<RefCell<i64>>`: made, or shared, `Rc::clone(&r)`,
  so a write through one, `*r.borrow_mut() += e`, is read through the
  other, `*r.borrow()`, its borrow over where it's read; and counted,
  `Rc::strong_count`. And a `Vec` sorted where ties show, each stable, as
  JS's `sort` is: `sort_by_key(|x| *x % k)`, `sort_by` of a key then its
  reverse, `dedup_by_key`, and `retain` by a closure that may note. Not
  `sort_unstable` of ties, whose order Rust doesn't say (2026-10-15).
- Tuples and destructuring: a `Pair`, `(i32, u8)`, made, its parts read,
  `p.0`, shown; `let (mut x, mut y) = p;` and `let S { a: mut x, c: mut
  y, .. } = s;`; two assigned at once, `(x, y) = (e1, e2);`, a swap or of
  what reads and writes either, its right side all made before either is
  written; a slice's first and last, `if let &[x, .., y] =
  v.as_slice()`; and a `match` of two values at once, `match (a, b) {
  (1, _) => .., (_, 2) => .., (x, y) if .. => .., _ => .. }`
  (2026-10-15).
- It keeps to what the borrow checker allows: a `Vec` is read through
  `clone()`, never moved, and a closure takes copies, so a later write to
  what it captured doesn't conflict with it.
- Each literal is `id(..)`'s, a function rustc doesn't see through, so it
  can't reject a program for an overflow it would work out. A program may
  panic, dividing by zero, as the oracle compares panics too.

**`test/fuzz.test.ts` runs each seed's program as a corpus case is run**
(`test/programs.ts`): natively, and as JS under Node (and Bun, until ADR 0095), which must
print the same and end the same. A program rustc rejects is the
generator's bug and fails the test; one rust-js says it doesn't support is
skipped.

**A program that differs is kept, then reduced:** before anything else,
`target/fuzz/` gets the program as generated, `seed-N.original.rs`, and
`seed-N.json`: its seed, the compiler's path and SHA-256, the source's
commit, how it failed and what each run printed. Then statements are taken
away, an `if` or a loop made what's in it, an expression made one of its
parts or a literal, each change kept if the program still fails the same
way and is shorter, so the reducing ends. The same way is the same
signature: the same runtimes differing from native Rust in the same of
stdout, stderr and how it ended, or the same crash, so a reduction can't
drift to another bug. Each smaller program is written to `seed-N.rs` as
it's found, and reducing stops after five minutes, saying it didn't
finish, so a reduction that never ends, or a job stopped, loses nothing.
Found in review. What's left becomes a corpus case.

- `bun test` runs the first 12 seeds, which must compile and match: they
  did, so one rust-js rejects is a regression. `FUZZ_START` and
  `FUZZ_SEEDS` explore others, as many as there's time for, where a program
  rust-js doesn't support is skipped. A big batch runs on GitHub, where it
  isn't slowed by macOS's scan of each new binary (AGENTS.md): the rustc
  tests workflow with `fuzz_seeds` splits it across six machines and keeps
  each reduced program as an artifact.

## Why

- **It found what the written tests hadn't:** seed 39, reduced from 17
  statements to one, printed `{:#x}` of a negative `i64` as `0x-80000000`,
  where Rust prints its 64 bits, `0xffffffff80000000`. It's fixed, and the
  corpus keeps it (`radix_negative.rs`). Seed 79, reduced to one line,
  replaced an empty pattern in `"🦀x"`: JS's `replaceAll` put the
  replacement between the emoji's two UTF-16 units, where Rust puts it at
  each char's boundary; `split("")` differed too (`empty_pattern.rs`).
  And ten of 300 seeds used a binding in a guarded arm, `E::B(n) if n > 0`,
  which rust-js rejected; it's supported now (`guarded_bindings.rs`).
- **A failure is a few lines,** which say what's wrong, rather than a
  program of dozens.

## Consequences

- The generator makes only what's here; each kind of Rust it's taught to
  make, structs or collections next, finds its own bugs.
- Seeds 1,000 to 2,199 ran on GitHub before writes were generated; one
  differed, seed 1476, an index checked before its value
  (`assignment_order.rs`). A review found what no seed could, a value read
  after an index that wrote it, and writes are generated since.
- A seed is a program only for this version of the generator: a change to
  it makes other programs of the same seeds, so a failure is kept as its
  reduced program, not its seed.
- A batch is the seeds it's asked for, or it fails: `FUZZ_START`,
  `FUZZ_SEEDS` and `FUZZ_REDUCE_BUDGET` are whole numbers, or the run
  fails before a seed does, and on GitHub each part says which seeds it
  ran to the end, which must be each one asked for, once. Found in review:
  `FUZZ_START=invalid` passed, as a run of none.
- A seed is a 32-bit number to the generator, so `FUZZ_START` and the
  batch's last seed must be in that range, or the run fails before a seed
  does. Found in review: a start past 2^53 didn't count up, and ran none.

## Amendment: a program ends

What's appended to a text reads no text: `v4.push_str(v4.clone())` in
nested loops, seed 1311's, doubled a string 64 times, and the program ran
until its runner ran out of memory, a fuzz job killed with nothing said,
twice. A test holds it of 2000 seeds' programs.
