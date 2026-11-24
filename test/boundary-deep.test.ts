// What a JS caller gives a Rust function that holds other values, a
// struct, a tuple, an array or a `Vec`, is checked whole, as deep as it
// goes (#2403 step 2b(b), ADR 1194): each struct by its module's checker,
// `checkOrder`, each field by its key, each item by its type. A failure
// names where in the value it is, `count(order.items[1].qty)`; a value
// that checks runs as before.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

const source = `pub enum Status {
    Open,
    Shipped,
}

pub struct Item {
    pub name: String,
    pub qty: u32,
}

pub struct Order {
    pub id: u32,
    pub items: Vec<Item>,
    pub note: Option<String>,
    pub status: Status,
    pub at: (u32, f64),
    pub codes: [u8; 2],
}

pub fn count(order: &Order) -> u32 {
    order.items.iter().map(|item| item.qty).sum::<u32>() + order.id + order.codes[0] as u32
}

pub fn totals(rows: Vec<Vec<u32>>) -> u32 {
    rows.iter().flatten().sum()
}

// A struct that holds itself checks by its own checker.
pub struct Node {
    pub value: u32,
    pub next: Option<Box<Node>>,
}

pub fn length(node: &Node) -> u32 {
    1 + node.next.as_ref().map_or(0, |next| length(next))
}

// An enum with fields: a fieldless variant its name, one with fields an
// object of its \`TAG\`; of a tag of its own, each an object of it.
pub enum Shape {
    Circle { radius: f64 },
    Square(f64),
    Empty,
}

#[cfg_attr(rust_js, rust_js::tag = "status")]
pub enum Load {
    #[cfg_attr(rust_js, rust_js::name = "pending")]
    Pending,
    #[cfg_attr(rust_js, rust_js::name = "done")]
    Done { value: u32 },
}

pub fn area(shape: &Shape) -> f64 {
    match shape {
        Shape::Circle { radius } => radius * radius,
        Shape::Square(side) => side * side,
        Shape::Empty => 0.0,
    }
}

pub fn loaded(load: &Load) -> u32 {
    match load {
        Load::Pending => 0,
        Load::Done { value } => *value,
    }
}

pub fn parsed(result: Result<u32, String>) -> u32 {
    result.unwrap_or(0)
}

// A generic type: one checker, given its type arguments' checkers.
pub struct Pair<A, B> {
    pub left: A,
    pub right: B,
}

pub struct List<T> {
    pub head: T,
    pub tail: Option<Box<List<T>>>,
}

pub fn sum_pair(pair: &Pair<u32, u32>) -> u32 {
    pair.left + pair.right
}

pub fn label_pair(pair: &Pair<String, u8>) -> usize {
    pair.left.len() + pair.right as usize
}

pub fn head(list: &List<u32>) -> u32 {
    list.head
}

// A field a symbol names, checked by that key (ADR 1051).
thread_local! {
    static ORDER_SYMBOL: &'static js::Symbol = js::symbol::new(Some(&"order"));
}

pub struct Reason {
    pub id: f64,
    #[cfg_attr(rust_js, rust_js::name(ORDER_SYMBOL))]
    pub order: u32,
}

pub fn order(reason: &Reason) -> u32 {
    reason.order
}

pub fn order_symbol() -> &'static js::Symbol {
    ORDER_SYMBOL.with(|symbol| *symbol)
}

// A map and a set, each entry by its types.
pub fn total(prices: &std::collections::HashMap<String, u32>) -> u32 {
    prices.values().sum()
}

pub fn tagged(tags: &std::collections::HashSet<String>) -> usize {
    tags.len()
}

// A \`&mut\`: a number's box, \`{ value }\`; an object, itself.
pub fn bump(n: &mut u32) {
    *n += 1;
}

pub fn restock(item: &mut Item) {
    item.qty += 1;
}

// A tuple struct is an array of its fields.
pub struct Size(pub u32, pub u32);

pub fn size_area(size: &Size) -> u32 {
    size.0 * size.1
}
`;

beforeAll(buildReact, 600_000);

let lib: Record<string, (...args: unknown[]) => unknown>;
let js: string;

beforeAll(async () => {
  const dir = fixture("boundary-deep");
  writeFileSync(join(dir, "lib.rs"), source);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024", "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target]);
  lib = await import(join(dir, "lib.js"));
  js = readFileSync(join(dir, "lib.js"), "utf8");
}, 600_000);

const thrown = (call: () => unknown) => {
  try {
    call();
  } catch (error) {
    return `${(error as Error).name}: ${(error as Error).message}`;
  }
  return "nothing thrown";
};

const order = () => ({
  id: 1,
  items: [
    { name: "tea", qty: 2 },
    { name: "cake", qty: 3 },
  ],
  status: "Open",
  at: [5, 0.5],
  codes: [7, 8],
});

// Its table, a constant its module makes once, marked pure, so a bundler
// drops one nothing reads (#2403, the lead's (B)).
test("a struct's checker is its module's table, each field by its key", () => {
  expect(js).toContain("const checkOrder = /* @__PURE__ */ $checkShape(() => ({\n  id: $checkU32,");
  expect(js).toContain("const checkItem = /* @__PURE__ */ $checkShape(");
  expect(js).toContain('$checks("count(order)", order, checkOrder);');
  expect(js.match(/const checkItem = /g)).toHaveLength(1);
});

test("a value that checks runs as before; an Option field's None is either JS's", () => {
  expect(lib.count!(order())).toBe(13);
  expect(lib.count!({ ...order(), note: null })).toBe(13);
  expect(lib.count!({ ...order(), note: "rush" })).toBe(13);
});

test("a failure names where in the value it is", () => {
  const qty = order();
  (qty.items[1] as { qty: unknown }).qty = "5";
  expect(thrown(() => lib.count!(qty))).toBe('TypeError: rust-js: count(order.items[1].qty): expected a u32, got "5"');
  const negative = order();
  negative.items[0]!.qty = -1;
  expect(thrown(() => lib.count!(negative))).toBe("RangeError: rust-js: count(order.items[0].qty): expected a u32, got -1");
  expect(thrown(() => lib.count!({ ...order(), note: 3 }))).toBe("TypeError: rust-js: count(order.note): expected a String, got 3");
  expect(thrown(() => lib.count!({ ...order(), status: "lost" }))).toBe('TypeError: rust-js: count(order.status): expected "Open" | "Shipped", got "lost"');
  expect(thrown(() => lib.count!({ ...order(), at: [1, "x"] }))).toBe('TypeError: rust-js: count(order.at[1]): expected an f64, got "x"');
  expect(thrown(() => lib.count!({ ...order(), codes: [1, 2, 3] }))).toBe("RangeError: rust-js: count(order.codes): expected an array of 2, got an array");
});

test("a missing field, a value of the wrong kind", () => {
  const missing: Record<string, unknown> = order();
  delete missing.items;
  expect(thrown(() => lib.count!(missing))).toBe("TypeError: rust-js: count(order.items): expected an array, got undefined");
  expect(thrown(() => lib.count!([]))).toBe("TypeError: rust-js: count(order): expected an object, got an array");
  expect(thrown(() => lib.count!(null))).toBe("TypeError: rust-js: count(order): expected an object, got null");
});

test("arrays of arrays, each item by its type", () => {
  expect(lib.totals!([[1, 2], [3]])).toBe(6);
  expect(thrown(() => lib.totals!([[1, 2], [3, -4]]))).toBe("RangeError: rust-js: totals(rows[1][1]): expected a u32, got -4");
});

test("a struct that holds itself", () => {
  expect(lib.length!({ value: 1, next: { value: 2 } })).toBe(2);
  expect(thrown(() => lib.length!({ value: 1, next: { value: "2" } }))).toBe('TypeError: rust-js: length(node.next.value): expected a u32, got "2"');
});

test("a tuple struct is an array of its fields", () => {
  expect(lib.sizeArea!([2, 3])).toBe(6);
  expect(thrown(() => lib.sizeArea!([2, -3]))).toBe("RangeError: rust-js: sizeArea(size[1]): expected a u32, got -3");
  expect(thrown(() => lib.sizeArea!([2]))).toBe("TypeError: rust-js: sizeArea(size): expected an array of 2, got an array");
});

test("an enum by its tag, then the fields of the variant it names", () => {
  expect([lib.area!({ TAG: "Circle", radius: 2 }), lib.area!({ TAG: "Square", _0: 3 }), lib.area!("Empty")]).toEqual([4, 9, 0]);
  expect(thrown(() => lib.area!({ TAG: "Circle", radius: "2" }))).toBe('TypeError: rust-js: area(shape.radius): expected an f64, got "2"');
  expect(thrown(() => lib.area!({ TAG: "Triangle" }))).toBe('TypeError: rust-js: area(shape): expected "Empty" | { TAG: "Circle" } | { TAG: "Square" }, got an object');
  expect(thrown(() => lib.area!("Circle"))).toBe('TypeError: rust-js: area(shape): expected "Empty" | { TAG: "Circle" } | { TAG: "Square" }, got "Circle"');
  expect([lib.loaded!({ status: "pending" }), lib.loaded!({ status: "done", value: 7 })]).toEqual([0, 7]);
  expect(thrown(() => lib.loaded!("pending"))).toBe('TypeError: rust-js: loaded(load): expected { status: "pending" } | { status: "done" }, got "pending"');
  expect([lib.parsed!({ TAG: "Ok", _0: 5 }), lib.parsed!({ TAG: "Err", _0: "no" })]).toEqual([5, 0]);
  expect(thrown(() => lib.parsed!({ TAG: "Err", _0: 5 }))).toBe("TypeError: rust-js: parsed(result._0): expected a String, got 5");
});

test("a generic type has one checker, given its type arguments' checkers", () => {
  expect(js.match(/function checkPair\(value, A, B\)/g)).toHaveLength(1);
  expect(js).toContain('$checks("sumPair(pair)", pair, (value) => checkPair(value, $checkU32, $checkU32));');
  expect(js).toContain('$checks("labelPair(pair)", pair, (value) => checkPair(value, $checkStr, $checkU8));');
  expect([lib.sumPair!({ left: 1, right: 2 }), lib.labelPair!({ left: "ab", right: 3 })]).toEqual([3, 5]);
  expect(thrown(() => lib.sumPair!({ left: 1, right: "2" }))).toBe('TypeError: rust-js: sumPair(pair.right): expected a u32, got "2"');
  expect(thrown(() => lib.labelPair!({ left: 1, right: 3 }))).toBe("TypeError: rust-js: labelPair(pair.left): expected a String, got 1");
  expect(lib.head!({ head: 1, tail: { head: 2 } })).toBe(1);
  expect(thrown(() => lib.head!({ head: 1, tail: { head: -2 } }))).toBe("RangeError: rust-js: head(list.tail.head): expected a u32, got -2");
});

test("a field a symbol names is checked by that key, its path the symbol's", () => {
  const symbol = lib.orderSymbol!() as symbol;
  expect(lib.order!({ id: 1, [symbol]: 7 })).toBe(7);
  expect(thrown(() => lib.order!({ id: 1, [symbol]: "7" }))).toBe('TypeError: rust-js: order(reason[Symbol(order)]): expected a u32, got "7"');
});

test("a map's entries and a set's items, each by its types", () => {
  expect(lib.total!(new Map([["tea", 2], ["cake", 3]]))).toBe(5);
  expect(thrown(() => lib.total!(new Map<string, unknown>([["tea", 2], ["cake", "3"]])))).toBe('TypeError: rust-js: total(prices.get("cake")): expected a u32, got "3"');
  expect(thrown(() => lib.total!(new Map([[1, 2]])))).toBe("TypeError: rust-js: total(prices.keys()): expected a String, got 1");
  expect(thrown(() => lib.total!({ tea: 2 }))).toBe("TypeError: rust-js: total(prices): expected a Map, got an object");
  expect(lib.tagged!(new Set(["a", "b"]))).toBe(2);
  expect(thrown(() => lib.tagged!(new Set(["a", 2])))).toBe("TypeError: rust-js: tagged(tags.values()): expected a String, got 2");
});

test("a &mut's box by its value, and an object it points at as the object", () => {
  const n = { value: 1 };
  lib.bump!(n);
  expect(n.value).toBe(2);
  expect(thrown(() => lib.bump!({ value: -1 }))).toBe("RangeError: rust-js: bump(n.value): expected a u32, got -1");
  expect(thrown(() => lib.bump!(1))).toBe("TypeError: rust-js: bump(n): expected a { value }, got 1");
  expect(thrown(() => lib.restock!({ name: "tea", qty: "1" }))).toBe('TypeError: rust-js: restock(item.qty): expected a u32, got "1"');
});

test("a value that holds itself is checked once of each type, not round its cycle forever", async () => {
  const dir = fixture("boundary-deep-cycles");
  writeFileSync(
    join(dir, "lib.rs"),
    `use std::cell::RefCell;
use std::rc::{Rc, Weak};

pub struct Task {
    pub id: u32,
    pub request: Weak<RefCell<Request>>,
}

pub struct Request {
    pub tasks: Vec<Rc<RefCell<Task>>>,
}

pub fn ids(request: Rc<RefCell<Request>>) -> u32 {
    request.borrow().tasks.iter().map(|task| task.borrow().id).sum()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  const cyclic = await import(join(dir, "lib.js"));
  const request: { tasks: unknown[] } = { tasks: [] };
  request.tasks.push({ id: 1, request }, { id: 2, request });
  expect(cyclic.ids(request)).toBe(3);
  (request.tasks[1] as { id: unknown }).id = "2";
  expect(thrown(() => cyclic.ids(request))).toBe('TypeError: rust-js: ids(request.tasks[1].id): expected a u32, got "2"');
});

// Two types of one name, in two modules, each its own checker: shlex's
// `Shlex` and `bytes::Shlex` were both `checkShlex`, declared twice, and
// where both are of one kind, one would check the other's fields. Now
// the namer allots a checker's name, of its type's, apart from every
// other word its module takes, `checkShlex$1` (ADR 1150, #2403).
test("two types of one name are each checked by their own checker", async () => {
  const dir = fixture("boundary-deep-one-name");
  writeFileSync(join(dir, "lib.rs"), `pub mod bytes {
    pub struct Shlex {
        pub at: u32,
    }
}

pub struct Shlex {
    pub word: String,
}

pub fn both(text: &Shlex, bytes: &bytes::Shlex) -> u32 {
    text.word.len() as u32 + bytes.at
}
`);
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), "--", "--crate-type=lib", "--edition=2024"]);
  const text = readFileSync(join(dir, "lib.js"), "utf8");
  expect(text).toContain("const checkShlex = ");
  expect(text).toContain("const checkShlex$1 = ");
  const named = await import(join(dir, "lib.js"));
  expect(named.both({ word: "ab" }, { at: 1 })).toBe(3);
  expect(thrown(() => named.both({ word: "ab" }, { word: "x" }))).toContain("both(bytes.at)");
  expect(thrown(() => named.both({ at: 1 }, { at: 1 }))).toContain("both(text.word)");
});
