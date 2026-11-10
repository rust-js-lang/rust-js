# 1117. A string's `chars()` are lazy

Status: Accepted (the user's decision on #1186, 2026-11-10: option 1,
"`chars()` is lazy everywhere"). Amends [0034](0034-strings-and-chars.md)
and [0170](0170-size-hint.md).

Case ([0262](0262-when-rust-and-js-disagree.md)): A for the form, since
JS's own string iterator steps a string's code points as Rust's `chars()`
does and nothing observable differs. R for its hint, its back and its
rest: where those are asked, the runtime's iterator gives Rust's.

## Context

`s.chars()` was `Array.from(s)`, an array of every `char` of the text, and
`s.char_indices()` was `$charIndices(s)`, an array of every `[byte, char]`
pair. So `rest[i..].chars().next()`, a parser's way of reading one `char`,
cost the length of what was left. A loop of it over 100,000 bytes, and a
`char_indices()` tokenizer, didn't end in ten seconds (#1186). Rust's
`Chars` reads only what it steps to.

## Decision

- **Stepped only from its front: JS's own string iterator,**
  `s[Symbol.iterator]()`. It reads one code point at a time, as Rust's
  `next()` does. A `for` over it is `for (const c of s)`, a collect is
  `Array.from(s)`, and a lent one is itself, since it has no `return()` to
  keep from being called. A count is `Array.from(s).length`, as a JS
  developer writes it (the user's decision): a count reads every `char`
  either way. An `enumerate()` is lazy, `s[Symbol.iterator]().map((x, i)
  => [i, x])`, not `Array.from(s).entries()`, so a parser's loop that
  breaks early doesn't read the whole text each time (the user's
  decision). A collect, a `for` and a count of a `$chars(s)` made where
  it's read are the string's too, as is a `rev()` collected,
  `Array.from(s).toReversed()`.
- **Asked more: the runtime's `$chars(s)`.** This is an `Iterator.prototype`
  object with `next`, `back`, `rest`, `clone` and `sizeHint`. It tracks its
  front and back as UTF-16 indices and its front as UTF-8 bytes. Its
  `as_str()` is the text between its front and back, recorded as a part of
  its text (#1184). Its hint is std's, `(bytes.div_ceil(4), Some(bytes))`
  of the bytes it has left. The crate makes every `Chars` this way
  (`analysis/chars_asked.rs`, the fact `chars_whole`; the std names it reads
  are `recognition/chars.rs`') where any of these is true:
  - it steps one from its back (`next_back`, `rev`, `rposition`, any
    `DoubleEndedIterator` method), whether the `Chars` itself or what holds
    one, a `Fuse` say;
  - it asks one's `as_str()`, or the `size_hint()` of what holds one;
  - it shows one with `{:?}`, or through any `Debug` bound;
  - it makes a `dyn` of one;
  - it gives what holds one to a function that isn't std's, its own or
    another crate's, whose body may ask anything of it (a helper taking
    `&mut Chars`, generic code asking `size_hint()`);
  - it's a library, and a function another crate may call gives or takes
    what holds one, through an `impl Trait` and its types' public fields;
  - it clones that type (`cloned`).

  Code it can't see is given only `$chars`, whose methods every
  consumer's helpers (`$nextBack`, `$restStr`, `$sizeHint`, `clone()`)
  read.
- **`char_indices()` is always `$charIndices(s)`,** which is `$chars(s,
  true)`: lazy, with `[byte, char]` items, and its back's byte counted from
  the text's length in bytes. Its `offset()` is its front's byte.
- **In a crate that reads a `Vec`'s capacity**, a `Vec` grown by a
  `chars()` or `char_indices()` stays refused, as before. JS's own string
  iterator has no hint, and JS's own adapters of `$chars`, `drop(n)` and
  `map((x, i) => …)`, keep none, so std's growth by the lower bound can't
  be followed. #1200 is this rule for every iterator; the refusal goes
  once JS's hint is asked of what each adapter keeps.

## Consequences

- `size_hint()` of a `chars()` or `char_indices()` itself is no longer
  refused: it's std's. Corpus case `chars_size_hint.rs`. Of an adapter of
  one made in place, `map` and the like, it's still refused (#733).
- A crate's or a library's helpers given a `Chars`, which `next_back()` of
  a lazy `Chars` used to refuse, step it from both ends. Corpus case
  `chars_given_to_helpers.rs`; crate pair `library_chars_given`.
- Generic code given `s.chars().skip(1)` gives `(0, None)` for its hint,
  not std's. That was true on main too, of the array's `drop(1)`, and it's
  #1200's.
- Numbers: the corpus case `long_text_read_char_by_char.rs`, 100,000
  bytes read a `char` at a time, as `rest[i..].chars().next()`, by a
  `char_indices()` tokenizer and from its back, didn't end in ten seconds
  before; it runs in about half a second.
- Held by the mutations of `recognition/chars.rs`,
  `analysis/chars_asked.rs` and the runtime's `char_indices.js`.
