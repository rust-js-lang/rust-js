// The pointer helpers in `@rust-js/std` (ADR 0562): a place's handle, its
// offset in its allocation and its address, as the runtime the compiler
// prints gives them. Split from runtime-package.test.ts by concept (#1334).

import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { node } from "./programs";
import { buildCompiler, compiler, fixture, root, run } from "./support";

beforeAll(buildCompiler, 600_000);

// A tuple's field's pointer is at the field's byte offset, rustc's layout,
// though JS holds the tuple as an array (#1081): its address, of a generic
// tuple whose layout its caller says, is refused where it's asked, as a
// named field's is; and it's no array's item, so stepping it, or the
// distance between two, which the layout says, is refused too.
test("a tuple's field's pointer is at its offset, never an item", () => {
  const dir = fixture("tuple-field-pointer");
  writeFileSync(join(dir, "lib.rs"), `fn address_of<A>(t: &(A, u32)) -> usize {
    &raw const t.1 as usize
}

pub fn generic() -> usize {
    address_of(&(1u8, 2))
}

pub fn stepped() -> u32 {
    let t = (1u32, 2u32);
    let p = &raw const t.0;
    unsafe { *p.add(1) }
}

pub fn distance() -> isize {
    let t = (1u32, 2u32);
    unsafe { (&raw const t.1).offset_from(&raw const t.0) }
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  expect(call("generic")).toBe("rust-js does not support the address of a field of a type its caller lays out, which reads memory by where it is\n");
  expect(call("stepped")).toBe("rust-js does not support stepping a pointer that isn't into an array, which reads memory by where it is\n");
  expect(call("distance")).toBe("rust-js does not support the distance between pointers to two places, which reads memory by where it is\n");
});

// An allocation of `alloc`'s is used as its first use says, its bytes or
// the items of a type it's cast to (ADR 0417, amended, #1099): the other
// use after it, a cast to another type past its first byte, and a `Vec` of
// its bytes as wider items, are rust-js's errors, never one type's memory
// read as another's.
test("an allocation's bytes and its items aren't both used", () => {
  const dir = fixture("allocation-uses");
  writeFileSync(join(dir, "lib.rs"), `use std::alloc::{Layout, alloc};

pub fn bytes_then_items() -> String {
    unsafe {
        let p = alloc(Layout::new::<String>());
        p.write(1);
        let s = p as *mut String;
        s.write(String::from("x"));
        (*s).clone()
    }
}

pub fn items_then_bytes() -> u8 {
    unsafe {
        let p = alloc(Layout::new::<String>());
        (p as *mut String).write(String::from("x"));
        *p
    }
}

pub fn items_past_its_start() -> String {
    unsafe {
        let p = alloc(Layout::from_size_align(48, 4).unwrap());
        let s = p.add(24) as *mut String;
        s.write(String::from("x"));
        (*s).clone()
    }
}

pub fn vec_of_words() -> usize {
    unsafe {
        let p = alloc(Layout::from_size_align(8, 4).unwrap());
        (p as *mut u32).write(1);
        (p.add(4) as *mut u32).write(2);
        Vec::from_raw_parts(p as *mut u32, 2, 2).len()
    }
}

pub fn signed_bytes_mut() -> i8 {
    let mut a = [255u8, 1];
    let s = unsafe { std::slice::from_raw_parts_mut(a.as_mut_ptr() as *mut i8, 2) };
    s[0]
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  const both = "rust-js does not support an allocation's bytes and its items of another type both, which reads memory by where it is\n";
  expect(call("bytesThenItems")).toBe(both);
  expect(call("itemsThenBytes")).toBe(both);
  expect(call("itemsPastItsStart")).toBe("rust-js does not support a pointer into an allocation's bytes as one to another type, which reads memory by where it is\n");
  expect(call("vecOfWords")).toBe("rust-js does not support a `Vec` of what isn't an array's first item, which reads memory by where it is\n");
  // Its slots are unsigned bytes, never an `i8`'s (#1107): a mutable
  // slice of them as `i8`s reads each through its codec (#1257).
  expect(call("signedBytesMut")).toBe("-1\n");
});

// A raw pointer to a place reached through more than one field or item is
// given the root its place starts from and its byte offset there, rustc's
// layout, so its address is in the root's allocation (#1089, the user's
// decision); a place of one step is as it was.
test("a nested place's pointer is given its root and its offset there", () => {
  const dir = fixture("nested-field-pointer");
  writeFileSync(join(dir, "lib.rs"), `pub struct P {
    pub a: u8,
    pub b: u32,
}

pub struct Outer {
    pub x: u16,
    pub inner: P,
}

pub fn offsets(items: [P; 3], o: Outer, i: usize) -> [usize; 4] {
    let first = &raw const items as usize;
    let at = &raw const o as usize;
    [
        &raw const items[1].b as usize - first,
        &raw const items[i].a as usize - first,
        &raw const o.inner.a as usize - at,
        &raw const o.x as usize - at,
    ]
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const js = readFileSync(out, "utf8");
  expect(js).toContain(`$ptr(items[1], "b", 0, items, 8)`);
  expect(js).toContain(`$ptr(items[i], "a", 4, items, i * 8 + 4)`);
  expect(js).toContain(`$ptr(o.inner, "a", 4, o, 4)`);
  expect(js).toContain(`$ptr(o, "x", 8)`);
});

// A pointer's address is a number of rust-js's own, shown as Rust shows one,
// `0x` and hex; the same place shows the same, and a null one's is `0x0`,
// as every program shows it (ADR 0562).
test("a pointer's address is shown", () => {
  const dir = fixture("pointer-shown");
  writeFileSync(join(dir, "lib.rs"), `pub fn debug() -> String {
    let x = 1;
    format!("{:?}", &raw const x)
}

pub fn pointer() -> String {
    let x = 1;
    let p = &raw const x;
    let again = format!("{:p}", p);
    format!("{:p} {}", p, again == format!("{:p}", p))
}

pub fn none() -> String {
    format!("{:?} {:p}", std::ptr::null::<u8>(), std::ptr::null::<u8>())
}

// Reading through a pointer to no place is rust-js's error where it runs.
pub fn no_place() -> u8 {
    let p = 7 as *const u8;
    unsafe { *p }
}

fn named() {}

// A function pointer is a pointer, its address shown as a raw one's is.
pub fn function_pointer() -> String {
    format!("{:?}", named as fn())
}

// What only a failing assertion would show: never.
pub fn compared() -> bool {
    assert_eq!(named as fn(), named as fn());
    assert_eq!(std::any::TypeId::of::<u8>(), std::any::TypeId::of::<u8>());
    true
}

// A TypeId shows its hash, which rust-js doesn't keep.
pub fn type_id() -> String {
    format!("{:?}", std::any::TypeId::of::<u8>())
}
`);
  const out = join(dir, "lib.js");
  run([compiler, join(dir, "lib.rs"), "-o", out]);
  const call = (name: string) =>
    run([node ?? "node", "--input-type=module", "--eval", `try { console.log((await import(${JSON.stringify(out)})).${name}()); } catch (e) { console.log(e.message); }`]);
  expect(call("debug")).toMatch(/^0x[0-9a-f]+\n$/);
  expect(call("pointer")).toMatch(/^0x[0-9a-f]+ true\n$/);
  expect(call("noPlace")).toBe("rust-js does not support reading through a pointer to no place, at 0x7, which reads memory by where it is\n");
  expect(call("functionPointer")).toMatch(/^0x[0-9a-f]+\n$/);
  expect(call("none")).toBe("0x0 0x0\n");
  expect(call("compared")).toBe("true\n");
  expect(call("typeId")).toBe("rust-js does not support showing a `TypeId`'s hash, which it doesn't keep\n");
});

// A pointer to what a place's handle refers to, its `value`, at its
// start, is that handle, the same place: a pointer made in generic code
// from a `&mut T` its caller gave as a field's handle, in a closure or
// through a reborrow, had the handle object's own address, 64 where
// native's is 0 (#1479).
test("a pointer to a place handle's value is that handle", () => {
  const module = join(fixture("runtime-handle-of-a-handle"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $addr, $ptr, $ptrTo } = await import(${JSON.stringify(module)});
const s = { a: 1, n: 2 };
const field = $ptr(s, "n", 4);
console.log($ptr(field, "value", 0) === field, $addr($ptr(field, "value", 0), 4) - $addr($ptrTo(s), 8));
const cell = { value: 3 };
console.log($ptr(cell, "value", 0) === cell);
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe("true 4\nfalse\n");
});

// A handle on what an accessor handle refers to, a `&mut dyn`'s pair's
// `value`, `{ impl, get value() { .. } }`: no place, its address refused,
// never a made-up one (#1519); reads and writes through it still the place's.
// A place handle's `value` is the handle (#1479), its address the field's.
test("the address through a handle on no place it records is refused", () => {
  const module = join(fixture("runtime-handle-through-a-pair"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $addr, $ptr, $ptrTo } = await import(${JSON.stringify(module)});
const s = { a: 0, n: 2 };
const pair = { impl: {}, get value() { return s.n; }, set value(v) { s.n = v; } };
const through = $ptr(pair, "value", 0);
through.value = 5;
let refused = "";
try { $addr(through, 4); } catch (error) { refused = error.message.includes("#1519") ? "refused" : error.message; }
const field = $ptr(s, "n", 4);
console.log(s.n, refused, $addr($ptr(field, "value", 0), 4) - $addr($ptrTo(s), 8));
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe("5 refused 4\n");
});

// One key, two places: an `Ok`'s `_0` at 1 and, once the `Result` is
// written over with an `Err`, its `_0` at 2 (#1484). Each its own handle,
// at its own offset; the same offset the same handle, and a handle given
// none the one there is.
test("a key at another offset is another place's handle", () => {
  const module = join(fixture("runtime-key-at-another-offset"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $addr, $assign, $ptr, $ptrTo } = await import(${JSON.stringify(module)});
const r = { TAG: "Ok", _0: 5 };
const base = $addr($ptrTo(r), 4);
const ok = $ptr(r, "_0", 1);
$assign(r, { TAG: "Err", _0: 6 });
const err = $ptr(r, "_0", 2);
console.log(err === ok, $addr(ok, 1) - base, $addr(err, 2) - base, $ptr(r, "_0", 2) === err, $ptr(r, "_0") === ok, err.value);
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe("false 1 2 true true 6\n");
});

// A handle that reads a place it doesn't record, `{ get value() { .. } }`
// of a `&mut` std lent from inside its storage, is on no allocation it
// says: its address is refused, never a made-up one (#1474). A box's
// `{ value }` cell, its own allocation, and an object's handle have theirs.
test("the address of a handle on no place it records is refused, not made up", () => {
  const module = join(fixture("runtime-placeless-handle"), "runtime.js");
  writeFileSync(module, run([compiler, "--runtime-module"]));
  const script = `
const { $addr, $ptrTo } = await import(${JSON.stringify(module)});
const slot = { _0: 5 };
const lent = { get value() { return slot._0; }, set value(v) { slot._0 = v; } };
const said = (run) => { try { return typeof run(); } catch (e) { return e.message; } };
console.log(said(() => $addr(lent, 4)));
console.log(said(() => $addr({ value: 1 }, 4)));
console.log(said(() => $addr($ptrTo({ a: 1 }), 4)));
`;
  expect(run([node ?? "node", "--input-type=module", "--eval", script])).toBe(
    "rust-js does not support the address of a `&mut` std lends from inside its storage, a handle on no place it records (#1513)\nnumber\nnumber\n",
  );
});
