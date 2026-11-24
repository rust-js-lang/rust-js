// The marks the runtime sets on what it makes, which every copy of the
// runtime reads: a bundled library and the app that uses it may each hold
// one, and what one makes, a panic, an `Rc`, an iterator, a part of a
// slice, is read by the other (ADR 1184, #2333). Each is a registry
// symbol, `Symbol.for`, named `rust-js: ..`: a mark made by `Symbol(..)`
// is one copy's alone, which `test/runtime-marks.test.ts` refuses. None is
// enumerable on what it marks, so a host shows it as it is.

// What marks a panic's `Error` as one whose payload is a `String`.
const $stringPayload = Symbol.for("rust-js: a panic whose payload is a String");

// Where a panic was raised, `file:line:column` (#2292), which a binary's
// hook names.
const $raisedAt = Symbol.for("rust-js: where a panic was raised");

// What a slice's chunks or windows were made of, kept on the array of them
// `$chunks` and `$windows` make: its source, its size, and whether its
// parts are windows. std's `Debug` of one shows what it has left as its own
// struct holds it, `Chunks { v: [3], chunk_size: 2 }` (#2252).
const $partsOf = Symbol.for("rust-js: what a slice's chunks or windows were made of");

// What a closure carries its clone under (ADR 0522).
const $cloneKey = Symbol.for("rust-js: what a closure carries its clone under");

// What marks a `Vec`'s or an array's `IntoIter` given its slice's steps,
// `$slicing`'s, which only a caller that knows its type asks, or a JS
// iterator made of one that passes them on (`$given`): the one answer of
// whether it's one, never read of which steps it has, as a flatten has an
// `advanceBackBy` of its own too (#2154).
const $slicedKey = Symbol.for("rust-js: an iterator that steps a slice's items");

// What marks an array's iterator, `$iter`'s, which a clone, `$iter` of its
// items again, is too: the one answer of whether an iterator is one, so its
// `items`, `at` and `end` may be read, never read of whether it has an
// `items` field, as a crate's struct may (#2159).
const $arrayIterKey = Symbol.for("rust-js: an array's iterator");

// The array and the index of the item a `v.iter_mut()`'s handle is on, of
// numbers or strings, whose slot a raw pointer to it is (mut_items.js, #1283).
const $itemArray = Symbol.for("rust-js: the array an item's handle is of");
const $itemIndex = Symbol.for("rust-js: the index an item's handle is at");

// A `#[rust_js::class]` instance's counting box, hidden beside it (rc.js,
// ADR 0320, #1228).
const $countedKey = Symbol.for("rust-js: an Rc's box beside its instance");

// A part's record of where it is, `{ of, start }`, of the array that holds
// its items itself, under a symbol: a property `$ptr` reads, which JS's own
// reads of an array, its items, its length, `for .. of` and `JSON`, never
// see (#1185, the user's decision: always kept, at 1.2–1.3 times a slice's
// cost to make).
const $placeOfPart = Symbol.for("rust-js: where a part of a slice is");

// What stdout holds of a line left unended, kept where every copy of the
// runtime finds it, `globalThis` (print.js's `$stdoutState`): a library's
// copy's `print!` held, its app's copy's line ends it (#2320).
const $stdoutKey = Symbol.for("rust-js: what stdout holds of a line left unended");

// A binary's panic hook, `$hook`, which its `$main` sets, every copy's
// panic writes by: a library's copy's too, whose panic its app's hook
// writes, as native Rust's one hook does (#2447).
const $hookKey = Symbol.for("rust-js: a binary's panic hook");

// Where a `count` or `last` that may reach a `Repeat`'s own was called, a
// slot of `globalThis` while it runs (count.js's `$infinitelyAt`, #2324).
const $infiniteAtKey = Symbol.for("rust-js: where a count or last that may reach a Repeat's own was called");
