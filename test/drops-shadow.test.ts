// The drop elaboration's shadow (#2112's change D, step 1): of each `Drop`
// of a case's bodies, whether what rust-js's elaboration drops of each part
// is what rustc's own elaborated body drops there, never, always or as a
// flag says, the `drops` shadow's lines (`--shadow drops=<file>`). On the normal path no case may
// disagree or be unmatched, either refused; a cleanup's are apart, as
// rust-js's body has no unwinding of what can't throw (ADR 0301).
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { runSync } from "./child";
import { buildCompiler, compiler, fixture, root, shadowLines } from "./support";

beforeAll(buildCompiler, 600_000);

/** Each line the `drops` shadow writes of a corpus case, by its verdict and path. */
function shadowOf(name: string): string[][] {
  const dir = fixture("drops-shadow");
  const log = join(dir, `${name}.txt`);
  writeFileSync(log, "");
  const args = [compiler, "--shadow", `drops=${log}`, join(root, "test/corpus", name), "-o", join(dir, `${name}.js`), "--", "--edition=2024"];
  const out = runSync(args, dir, 120_000);
  expect(out.code, out.stderr).toBe(0);
  return shadowLines(log, "drops").map((line) => line.split("\t"));
}

const count = (lines: string[][], verdict: string, path: string) =>
  lines.filter(([v, p]) => v === verdict && p === path).length;

// Partial moves of enums, structs, tuples, arrays and boxes, a copied
// default's: rustc's ladder of each, read through its tests of
// discriminants and flags.
const partial = [
  "match_binding_moves_what_a_partial_arm_keeps.rs",
  "drop_partial.rs",
  "or_pattern_part_moves.rs",
  "arm_temporaries.rs",
  "array_partial_moves.rs",
  "copied_default_impl_drops.rs",
];

test("no normal path's drop disagrees with rustc's elaboration", () => {
  const disagreeing = partial.flatMap((name) =>
    shadowOf(name)
      .filter(([verdict, path]) => verdict === "disagree" && path === "normal")
      .map((line) => `${name}: ${line.slice(2).join(" ")}`),
  );
  expect(disagreeing).toEqual([]);
});

// #2064's: a `match` one arm of which moves part of an enum and another
// all of it, whose variants none takes apart are dropped as the enum's own
// flag says, as rustc's ladder tests its discriminant, then the flag.
test("a match's enum moved in part and whole is dropped where rustc drops it", () => {
  const lines = shadowOf("match_binding_moves_what_a_partial_arm_keeps.rs");
  const flatten = lines.filter(([, path, owner]) => path === "normal" && owner === "flatten");
  // Each of its drops on the normal path as rustc's: a `Text`'s bytes,
  // which JS drops nothing of, never compared.
  expect(flatten.filter(([verdict]) => verdict !== "agree").map((line) => line.slice(3).join(" "))).toEqual([]);
  expect(flatten.length).toBeGreaterThanOrEqual(10);
  expect(count(lines, "agree", "normal")).toBeGreaterThanOrEqual(40);
  // The rest of the moved enum, `(_15.0)`, as its own flag says on each
  // side, which compared at the top alone read as unalike.
  // Where the loop's `node` ends: its moved `Text` bytes and the rest of
  // the enum, each compared; and another drop written there.
  const ends = flatten.filter(([, , , at]) => at.includes(":45:5:")).map(([verdict, , , , detail]) => `${verdict} ${detail}`);
  expect(ends.sort()).toEqual(["agree 1 compared", "agree 2 compared"]);
});

// Change D's (b): a drop behind a pointer, `*it = other`'s or a capture's
// `d = ..`, compared whole, rustc's place read through `Derefer`'s copies;
// and two of rust-js's drops written alike at one place, `?`'s, compared as
// a multiset of what each side drops there, a dead one left out.
const behind = ["closure_in_closure_sets_a_capture.rs", "associated_iterator_dropped.rs"];
const alike = ["default_matches_its_tuple_whole.rs", "array_parts_moved_in_a_copied_default.rs"];

test("a drop behind a pointer is compared whole, as the writer drops it", () => {
  const lines = shadowOf("associated_iterator_dropped.rs").filter(([, path]) => path === "normal");
  const put = lines.filter(([, , , at]) => at.includes(":67:5:"));
  expect(put.map(([verdict, , , , detail]) => `${verdict} ${detail}`)).toEqual(["agree behind a pointer, whole"]);
  expect(lines.filter(([verdict]) => verdict !== "agree")).toEqual([]);
  // A capture's `d = D(..)`, rustc's `drop((*_t))` of `_t = copy ((*_1).0)`.
  const capture = shadowOf("closure_in_closure_sets_a_capture.rs").filter(([, path]) => path === "normal");
  expect(capture.filter(([verdict]) => verdict !== "agree")).toEqual([]);
  expect(capture.filter(([, , , , detail]) => detail === "behind a pointer, whole").length).toBe(1);
});

test("two drops written alike are compared as a multiset", () => {
  const lines = shadowOf("default_matches_its_tuple_whole.rs").filter(([, path]) => path === "normal");
  // One agrees alone; the other as a multiset of the whole, not part by
  // part, which only a place with parts needs (#2177).
  const at = lines.filter(([, , , where]) => where.includes(":22:37:")).map(([verdict, , , , detail]) => `${verdict} ${detail}`);
  expect(at.sort()).toEqual(["agree 1 compared", "agree alike drops as a multiset, 2"]);
});

// A box's deref as `ElaborateBoxDerefs` writes it, `_p = copy (_b.0) as
// *const T (BoxDerefTransmute); drop((*_p))`, is the box's: `**x = Loud(..)`
// through a `&mut Box<Loud>` drops what it held, behind two pointers (#2177).
test("a drop through a box's deref is the box's, as rustc's box derefs read it", () => {
  const lines = shadowOf("retain_map_drops_held.rs").filter(([, path]) => path === "normal");
  const put = lines.filter(([, , , at]) => at.includes(":36:9:"));
  expect(put.map(([verdict, , , , detail]) => `${verdict} ${detail}`)).toEqual(["agree behind a pointer, whole"]);
});

// What had no rustc drop to be compared with, each now compared, none on
// the normal path unmatched (#2177): a closure's body as `call_once` runs
// it, its own drop as rustc's `ClosureOnce` shim drops it; an `async`
// body, rustc's passes run on it but the state machine's, its coroutine
// drop path, where the future is dropped suspended, left out, as JS's
// `await` has none, so what's held across an `.await` isn't refused; and
// what's left of a box something moved out of, its contents dropped as
// `open_drop_for_box_contents` writes it, `(*_p)` of `_p = copy ((_b.0).0)
// as *const T (Transmute)`, a field's box too.
test("a closure called once, an async body and a box's rest are compared with rustc's", () => {
  const cases: [string, string, number][] = [
    ["called_once_closure_drops_after.rs", "borrowed::{closure#0}", 2],
    ["fn_closure_moved_into_a_once_callee.rs", "main::{closure#3}", 1],
    ["future_size_unwinds.rs", "single::{closure#0}", 4],
    ["drop_held_across_an_await.rs", "held_twice::{closure#0}", 2],
    ["drop_held_across_an_await.rs", "named_as_rustc_names_it::{closure#0}", 2],
    ["boxed_value_partly_moved_drops.rs", "main", 3],
    ["boxed_field_moved_out_drops_the_rest.rs", "main", 1],
  ];
  for (const [name, owner, least] of cases) {
    const lines = shadowOf(name).filter(([, path]) => path === "normal");
    expect([name, lines.filter(([verdict]) => verdict !== "agree").map((line) => line.slice(2).join(" "))]).toEqual([name, []]);
    expect([name, lines.filter(([, , of]) => of === owner).length >= least]).toEqual([name, true]);
  }
});

// Two drops of one place written alike, an early return's and the end's,
// of a place with parts: each part compared as a multiset, the drop that
// drops a part never left out as rustc's elaboration leaves a dead one out
// (#2177). What's left of a variant, its fields with no path of their own,
// is compared as the writer drops it, by the enum's own state under a test
// of its discriminant, as rustc's ladder drops it (`rest_path`), not by the
// variant's, which says it's never there where another variant is: or-
// patterns' moves of a part of each of two variants, and a popped
// `Option` of one, regex-syntax's `pop_group_end`'s, each agree.
test("drops of one place written alike are compared part by part", () => {
  for (const [name, at] of [
    ["struct_update_drops.rs", ":44:1:"],
    ["generic_struct_with_a_destructor_dropped_on_an_early_return.rs", ":35:5:"],
  ]) {
    const lines = shadowOf(name).filter(([, path, , where]) => path === "normal" && where.includes(at));
    expect([name, lines.filter(([verdict]) => verdict !== "agree")]).toEqual([name, []]);
    expect([name, lines.some(([, , , , detail]) => detail.startsWith("alike drops part by part as a multiset"))]).toEqual([
      name,
      true,
    ]);
  }
  // Alike drops that differ only by a flag rust-js asks where rustc knows
  // are judged as one drop's are, flag-ours, never a disagreement.
  const flagged = shadowOf("array_parts_moved_in_a_copied_default.rs").filter(([, , owner]) => owner === "Picks::second");
  expect(flagged.filter(([verdict]) => verdict === "disagree")).toEqual([]);
  expect(flagged.some(([verdict, , , , detail]) => verdict === "flag-ours" && detail.includes("alike drops drop it"))).toBe(true);
  const variants = shadowOf("or_pattern_part_moves.rs").filter(([, path]) => path === "normal");
  expect(variants.filter(([verdict]) => verdict !== "agree").map((line) => line.slice(2).join(" "))).toEqual([]);
  const ends = variants.filter(([, , owner, at]) => owner === "middle" && at.includes(":62:1:"));
  // A cleanup's drop only rust-js asks a flag of, which only running it
  // says alike or not: flag-ours, never a disagreement.
  const cleanup = shadowOf("or_pattern_part_moves.rs").filter(([, path, owner, at]) => path === "cleanup" && owner === "middle" && at.includes(":59:5:"));
  expect(cleanup.map(([verdict, , , , detail]) => `${verdict} ${detail}`)).toEqual(["flag-ours `m`: rust-js drops it as its drop flag says, rustc always"]);
  expect(ends.map(([verdict, , , , detail]) => `${verdict} ${detail}`)).toEqual(["agree 4 compared"]);
  const popped = shadowOf("popped_variant_rest_dropped_at_the_match_end.rs").filter(([, path]) => path === "normal");
  expect(popped.filter(([verdict]) => verdict !== "agree").map((line) => line.slice(2).join(" "))).toEqual([]);
  const end = popped.filter(([, , owner, at]) => owner === "Parser::pop_group_end" && at.includes(":41:10:"));
  expect(end.some(([, , , , detail]) => detail.startsWith("alike drops part by part as a multiset"))).toBe(true);
});

// The normal path's verdicts over these cases, each pinned: none disagrees,
// the baseline a refusal of a disagreement (change D's (c)) starts from.
test("each normal-path verdict of the partial moves, behind a pointer and alike, is pinned", () => {
  const tally = new Map<string, number>();
  for (const name of [...partial, ...behind, ...alike]) {
    for (const [verdict, path] of shadowOf(name)) {
      if (path === "normal") tally.set(verdict, (tally.get(verdict) ?? 0) + 1);
    }
  }
  expect(Object.fromEntries(["agree", "disagree", "flag-ours", "unmatched"].map((v) => [v, tally.get(v) ?? 0]))).toEqual({
    agree: 203,
    disagree: 0,
    "flag-ours": 0,
    unmatched: 0,
  });
});

// Their cleanups' verdicts, pinned too: a cleanup no terminator that
// unwinds in JS reaches (`unwinds`) is `unreached`, never judged, so their
// number can't grow unseen (#2112's change D3).
test("each cleanup verdict of the partial moves, behind a pointer and alike, is pinned", () => {
  const tally = new Map<string, number>();
  for (const name of [...partial, ...behind, ...alike]) {
    for (const [verdict, path] of shadowOf(name)) {
      if (path === "cleanup") tally.set(verdict, (tally.get(verdict) ?? 0) + 1);
    }
  }
  const verdicts = ["agree", "disagree", "flag-ours", "flag-theirs", "unmatched", "unreached"];
  expect(Object.fromEntries(verdicts.map((v) => [v, tally.get(v) ?? 0]))).toEqual({
    agree: 164,
    disagree: 0,
    "flag-ours": 11,
    "flag-theirs": 1,
    unmatched: 0,
    unreached: 70,
  });
});

// Change D's (c): a drop the normal path's elaboration drops otherwise than
// rustc's is refused, in every compile, the place and both answers named.
// `--without-moves` makes rust-js's elaboration blind to moves, each drop
// dropping all of what it drops, so a moved part's drop is one rustc's
// never has; without it the case compiles.
/** `drop_partial.rs` compiled, `--without-moves` or not. */
function compilePartial(...knob: string[]) {
  const dir = fixture("drops-shadow-refused");
  return runSync([compiler, ...knob, join(root, "test/corpus/drop_partial.rs"), "-o", join(dir, "drop_partial.js"), "--", "--edition=2024"], dir, 120_000);
}

test("a normal path's drop that disagrees with rustc's elaboration is refused", () => {
  expect(compilePartial().code).toBe(0);
  const refused = compilePartial("--without-moves");
  expect(refused.code).not.toBe(0);
  const errors = refused.stderr.split("\n").filter((line) => line.startsWith("error: rust-js: this drop") && !line.includes(", as a panic unwinds,") && line.includes("is elaborated otherwise"));
  expect(errors).toContain(
    "error: rust-js: this drop in `main` is elaborated otherwise than rustc's drop elaboration, so its JS would drop otherwise than Rust: `whole.0`: rust-js drops it always, rustc never (#2112's change D)",
  );
  expect(errors.length).toBe(4);
  expect(refused.stderr).toContain("drop_partial.rs:42:5");
  // And on a cleanup path a panic in JS reaches, said so (change D3).
  const unwinding = refused.stderr.split("\n").filter((line) => line.startsWith("error: rust-js: this drop") && line.includes(", as a panic unwinds, is elaborated otherwise"));
  expect(unwinding).toContain(
    "error: rust-js: this drop in `main`, as a panic unwinds, is elaborated otherwise than rustc's drop elaboration, so its JS would drop otherwise than Rust: `whole.0`: rust-js drops it always, rustc never (#2112's change D)",
  );
  expect(unwinding.length).toBe(4);
});

// Change D2: the comparison fails closed. A normal path's drop it can't
// compare, which can't be said to drop as Rust does, is refused too:
// blind to moves, rust-js drops a moved value, `inner` given on, where
// rustc's elaboration left no drop to find, which no disagreement says.
test("a normal path's drop that can't be compared with rustc's elaboration is refused", () => {
  const refused = compilePartial("--without-moves");
  const errors = refused.stderr.split("\n").filter((line) => line.startsWith("error: rust-js: this drop") && !line.includes(", as a panic unwinds,") && line.includes("can't be compared"));
  expect(errors).toContain(
    "error: rust-js: this drop in `main` can't be compared with rustc's drop elaboration, so its JS can't be said to drop as Rust does: no rustc drop where it's written; `inner`: rust-js drops it always, rustc never (#2112's change D)",
  );
  expect(errors.length).toBe(13);
  const unwinding = refused.stderr.split("\n").filter((line) => line.startsWith("error: rust-js: this drop") && line.includes(", as a panic unwinds, can't be compared"));
  expect(unwinding.length).toBe(11);
});

// A body with no other side to compare, a thread-local's constant
// initializer, which rustc keeps no elaborated body of, is `uncompared`,
// never refused, as a library's body the crate compiles is, whose rustc
// body is its own crate's (#2112's change D2).
test("a drop of a body rustc keeps no elaborated body of is uncompared", () => {
  const lines = shadowOf("ref_cell_borrowed_by_a_thread_locals_clear.rs").filter(([, , owner]) => owner.endsWith("__RUST_STD_INTERNAL_INIT"));
  expect(lines.map(([verdict, path, , , detail]) => `${verdict} ${path} ${detail}`)).toEqual(["uncompared cleanup no rustc body"]);
});

// A cleanup only what can't throw in JS unwinds to, which rust-js's body
// doesn't unwind from (ADR 0301), is unreached, never judged nor refused:
// drop_functions.rs' `Box::new`'s and `vec!`'s, where rustc drops what's
// there and rust-js's elaboration says nothing is (#2112's change D3).
test("a cleanup no terminator that unwinds in JS reaches is unreached", () => {
  const cleanups = shadowOf("drop_functions.rs").filter(([, path]) => path === "cleanup");
  expect(cleanups.filter(([verdict]) => verdict === "disagree")).toEqual([]);
  expect(cleanups.filter(([verdict]) => verdict === "unreached").map(([, , , , detail]) => detail)).toEqual(
    Array(7).fill("no terminator that unwinds in JS reaches it"),
  );
});

// A part of a packed struct that may be misaligned rustc drops by moving
// it into a temporary of its own first, `_t = move ((_5.1).1.0); drop(_t)`
// (`AddMovesForPackedDrops`): read as the part, which rust-js drops in
// place, so what's left of `held.1.1` beside a moved field is compared, not
// refused as unmatched (rustc's packed/misaligned-reference-drop-field-99838.rs, #2112).
test("a packed struct's part rustc moves out to drop is compared as the part", () => {
  const lines = shadowOf("packed_part_dropped_beside_a_moved_one.rs");
  expect(lines.filter(([verdict]) => verdict !== "agree" && verdict !== "unreached").map((line) => line.join(" "))).toEqual([]);
  const end = lines.filter(([, path, , at]) => path === "normal" && at.includes(":43:1:"));
  expect(end.map(([verdict, , , , detail]) => `${verdict} ${detail}`).sort()).toContain("agree 2 compared");
});

// One shadow flag (ADR 1199): the two it replaced are refused, naming it,
// as is a kind it has none of.
test("--drops-shadow and --intervals-shadow are refused for --shadow, and an unknown kind is", () => {
  const dir = fixture("shadow-flags");
  const said = (...flags: string[]) => runSync([compiler, ...flags, join(root, "test/corpus/drop_partial.rs"), "-o", join(dir, "case.js")], dir, 120_000);
  for (const [gone, kind] of [["--drops-shadow", "drops"], ["--intervals-shadow", "intervals"]]) {
    const refused = said(gone!, join(dir, "log.txt"));
    expect(refused.code).not.toBe(0);
    expect(refused.stderr).toContain(`${gone} is gone: \`--shadow ${kind}=<file>\` records it`);
  }
  const unknown = said("--shadow", `borrows=${join(dir, "log.txt")}`);
  expect(unknown.code).not.toBe(0);
  expect(unknown.stderr).toContain("--shadow: no shadow `borrows`; there are `intervals`, `drops`");
});
