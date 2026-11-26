// A consumer refused for what a library it uses can't give it, each case
// a pair of crates under test/crates/refused/, compiled on its own and
// chained, its refusal's words and their places checked, and the pair's
// native output beside it. Split from test/crates.test.ts by concept
// (#1334).

import { beforeAll, expect, test } from "bun:test";
import { join } from "node:path";
import { buildCompiler, root } from "./support";
import { chain } from "./crates";

beforeAll(buildCompiler, 600_000);

// rd553: React's Flight server's request kept its prerender's `onAllReady`
// in a public field, which another crate may clone out and call after the
// request is gone, when the cell's `Weak` gives `None`, as this app does:
// the library's return doesn't alone own it, so its consumer's cell is
// counted, its `dyn Fn`s drop, and the library, whose `dyn Fn`s drop
// nothing, refuses it, naming the closure, what it holds and the call
// whose return doesn't own it (ADRs 0317, 0626, amended).
test("a library's request whose callback is a public field refuses its consumer's owner handle, saying why", () => {
  const native = chain(join(root, "test", "crates", "refused"), "owner_handle_public_field", ["dep", "app"], [
    "this crate's `dyn Fn`s may hold a value with a destructor, which their drop runs, but `dep`, a rust-js library whose `dyn Fn`s drop nothing, holds one of them",
    "app.rs:22:",
    "this `dyn Fn` holds a `std::rc::Rc<std::cell::OnceCell<std::rc::Weak<dep::Request>>>`, whose drop runs code",
    "and `dep::create_prerender_request`'s doesn't own `on_all_ready`: the field it's moved to must be private to its crate",
  ]);
  expect(native).toBe("a is ready\nno request\n");
}, 300_000);

// #682 across crates: a library's generic code gives a `Vec<&mut S>` to
// its trait's method through `S`'s dictionary, which a consumer's impl's
// entry takes as objects: refused at the consumer's call, as the library's
// own call of it is, though the trait isn't the consumer's (the
// `is_local()` audit).
test("a consumer's call giving a library trait's method a Vec of &mut objects through generic code is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_trait_dispatch_takes_mut_vec", ["dep", "app"], [
    "rust-js does not support a `&mut C` inside a generic function's parameters or result (#1421) yet",
    "app.rs:17:5",
  ]);
  expect(native).toBe("2 7\n");
}, 300_000);

// And of a library's generic impl whose dictionary its consumer makes of
// its own type (#685, #686 across crates).
test("a consumer's dictionary of a library's generic impl giving a Vec of &mut objects through its bound is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_impl_dispatch_takes_mut_vec", ["dep", "app"], [
    "a `&mut C` inside what a generic impl gives a method of a trait it's bound by",
    "app.rs:",
  ]);
  expect(native).toBe("2 7\n");
}, 300_000);

// #476, ADR 0145, amended: a library's manifest says what its traits'
// dictionaries carry of each associated type; a consumer's ask of what
// they don't carry is refused, never read as `undefined` of them.
test("a consumer's fact of a library's associated type its dictionaries don't carry is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "uncarried_associated_fact", ["dep", "app"], [
    "rust-js does not support `size_of` of a type parameter yet",
    "app.rs:8:",
  ]);
  expect(native).toBe("24\n");
}, 300_000);

// #1608: a library's writer that may stop part-way by a panic, which its
// contract says (`stopping_where_caught`), shown in stdout's line by a
// program that catches one, is refused, naming #1633 and what to write:
// Rust's stdout keeps `x [!L1, !L2, !`, where the library's JS makes the
// writer's text whole.
test("a library's writer that may stop part-way, shown in a line where a panic may be caught, is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_writer_stopping_where_caught", ["dep", "app"], [
    "rust-js does not support showing a `dep::Loud` in stdout's line where a `catch_unwind` may catch a panic yet",
    "show it in a `format!` first, whose text a panic drops, or where no `catch_unwind` catches (#1633)",
    "app.rs:17:28",
    "app.rs:22:5",
    "showing a `&T`, which may be a `dep::Loud`,",
    "app.rs:26:34",
  ]);
  expect(native).toBe("x [!L1, !L2, ! caught true\nx w! caught true\nshown ! caught true\n");
}, 300_000);

// And of a library whose writer shows its dependency's that may stop: its
// own may too, as its contract says (#1608).
test("a library's writer showing its dependency's writer that may stop part-way is refused where a panic may be caught", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_writer_showing_a_dependencys_stopping_writer", ["dep", "mid", "app"], [
    "rust-js does not support showing a `mid::Holder` in stdout's line where a `catch_unwind` may catch a panic yet",
    "app.rs:9:",
  ]);
  expect(native).toBe("x <! caught true\n");
}, 300_000);

// #1657: a library's derived `Debug` of a generic struct with a
// `PhantomData<T>` field shows `T`'s name, `type_name::<T>()`, which its
// generic code compiled in the library is given no fact of: refused,
// naming the issue, where native shows `PhantomData<u8>`.
test("a library's derived Debug of a PhantomData of a type parameter is refused, naming #1657", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_debug_of_phantom_data", ["dep", "app"], [
    "rust-js does not support `type_name` of a type parameter (#1657) yet",
  ]);
  expect(native).toBe("P { v: 1, m: PhantomData<u8> }\n");
}, 300_000);

// A consumer's impl of a library's trait whose generic method asks a fact
// of its own type parameter the library's contract doesn't give through a
// dictionary of the trait: the library's generic code calls it giving
// none, so it's refused, naming the fact and the contract, never read as
// `undefined` (#2285).
test("a consumer's impl of a library's trait asking a fact its contract doesn't give is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "own_impl_asks_a_fact_the_contract_doesnt_give", ["dep", "app"], [
    "`size_of` of `X` in `dep::Pick::pick`, which its library's contract doesn't give through a dictionary of its trait",
  ]);
  expect(native).toBe("8 4\n");
}, 300_000);

// #2542: a library's generic zip of what its consumer gives it, which
// std's `Zip` of a `TrustedRandomAccess` side stops at its shorter side
// of: refused at the consumer's call, as one crate's own generic code is,
// the library's ask read of its MIR, through another of its functions
// too.
test("a consumer giving a library's generic zip a closure std stops short of is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_zip_of_a_closure_std_stops_short", ["dep", "app"], [
    "giving generic code a zip it folds or steps, which std stops at its shorter side",
    "app.rs:11:13",
    "app.rs:20:13",
  ]);
  expect(native).toBe("[(2, 10), (4, 20)] [1, 2]\n[(1, 10), (2, 20)] 2\n");
}, 300_000);

// A library's generic `decode_utf16` of what its consumer gives, an
// iterator with a `Drop`: std's `DecodeUtf16` drops it at the loop's end,
// which the runtime's `$decodeUtf16` iterator, std's struct not compiled,
// can't. One crate's own generic code is refused; so is a consumer's call
// (#2575), where it compiled the library's function itself (ADR 0914) and
// dropped the `iter` field the runtime's iterator hasn't, a `TypeError`.
test("a consumer giving a library's decode_utf16 an iterator that drops is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_decode_utf16_of_an_iterator_that_drops", ["dep", "app"], [
    "giving `dep::decoded`'s `I` a type with a destructor, where it takes none",
    "app.rs:28:5",
    "app.rs:32:5",
  ]);
  expect(native).toBe("next own\nOk('a')\nnext own\nnext own\nErr(DecodeUtf16Error { code: 55296 })\nOk('b')\nnext own\ndrop own\nnext\nOk('c')\nnext\ndrop source\n");
}, 300_000);

// #1385's and #1389's shape, itertools' `MultiProduct::last`: a library's
// zip of a `Vec<Held<I>>` with values, given items that drop, whose fold
// std takes to the shorter side without stepping the first again. One
// crate's own generic code refuses it; so does a consumer's call (#2542),
// where it compiled the library's function itself (ADR 0914) and stepped
// the first side once more. The numbers' halves stay crate pairs.
for (const [name, line, printed] of [
  ["library_zip_of_what_is_never_random_access_of_items_that_drop", "app.rs:23:16", "drop 1\ndrop 8\ndrop 9\n[Loud(2), Loud(9)]\ndrop 2\ndrop 9\n"],
  ["library_zip_of_a_vec_of_a_slots_items_of_items_that_drop", "app.rs:23:16", "drop 1\ndrop 8\n[Loud(2), Loud(9)]\ndrop 2\ndrop 9\n"],
] as const) {
  test(`a consumer giving a library's zip items that drop, which std stops short of, is refused: ${name}`, () => {
    const native = chain(join(root, "test", "crates", "refused"), name, ["dep", "app"], [
      "giving generic code a zip it folds or steps, which std stops at its shorter side",
      line,
    ]);
    expect(native).toBe(printed);
  }, 300_000);
}

// #2559 across crates: a consumer's impl of a library's trait whose method
// gives a write's result where the trait gives `Self::Out`, its dictionary
// given to the library's generic code, which holds it as a value: refused
// where the dictionary is given, as one crate refuses it, until the entry
// gives its result as a value (#2565).
test("a consumer's impl giving a library's generic code a write's result as its Self::Out is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_valued_made_by_a_consumers_entry", ["dep", "app"], [
    "gives through its impl's dictionary, which generic code, given it or a `dyn` of it, holds as a value",
    "app.rs:19:",
  ]);
  expect(native).toBe("Err(Error) Err(Error) true\n");
}, 300_000);

// #2560 across crates: write results as the items of an iterator given to
// a library's generic code that reads them as `Result`s of its type
// parameters: refused at the call, as one crate refuses it, until each item
// is tagged where it crosses (#2566).
test("a consumer giving a library's generic code write results as Result items is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_given_write_results_as_items", ["dep", "app"], [
    "among the items of generic code's `I`, which it reads as `std::result::Result<T, E>`",
    "app.rs:9:",
  ]);
  expect(native).toBe("2\n");
}, 300_000);


// #2609: a closure holding a value with a destructor, moved into a
// library's struct and given to its generic code, whose impl calls it by
// `FnOnce`: the consumer's copy of `run_it` names the library trait's
// method of what its `R` is, `Once<{closure}>`, which may call what's in
// it either way, so it's refused where it's made, as one crate refused
// it, never compiled with its drop lost. A boxed `Fn` held so is refused
// already, as the library's `dyn Fn`s drop nothing (ADR 0626).
test("a closure in a library's struct its impl calls by value is refused", () => {
  const native = chain(join(root, "test", "crates", "refused"), "library_impl_calls_a_held_closure_by_value", ["dep", "app"], [
    "a closure that holds a value with a destructor, called by `FnOnce`, which drops none of it",
    "app.rs:16:",
  ]);
  expect(native).toBe("in through a library's trait\ndrop through a library's trait\n4\n");
}, 300_000);
