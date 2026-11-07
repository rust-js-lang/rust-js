// The unwind table (#854): panic points beyond iterators. Each operation
// of a family, values and their fields, closures that hold what drops,
// `Vec`, `VecDeque`, `BTreeMap`, `HashMap` and `BinaryHeap` operations that
// call the crate's code, a `BTreeMap`'s and a `BTreeSet`'s keys of each
// shape, `Option` and `Result` combinators, comparators and
// `?`, of each shape of value it applies to, a case. Each case runs once a
// point (`run_points`, test/panic-points.ts), as the iterator points table's
// do (#817): every make, drop, clone and comparison of a `D`, every drop of
// a struct or an enum with a `Drop` of its own, and every call of a
// closure's or a fallible step's is the k-th point once, which panics.
// test/iterator-matrix.test.ts runs it (`ITERATOR_TABLE=unwind`) in both
// builds against native Rust, as it runs the iterator tables.
//
// A combination that isn't Rust isn't made: each operation says what its
// shape must have (`Clone`, `Eq`, `Ord`, `Default`), each shape what it
// has. Nothing is skipped by a list. Its output is native Rust's alone to
// say: a `HashMap` holds one entry at a time, so no run depends on its
// order.

import type { Row } from "./iterator-matrix";
import { pointsRunner } from "./panic-points";

/** What a shape has that an operation may need. */
type Trait = "clone" | "eq" | "ord" | "default";
const all: Trait[] = ["clone", "eq", "ord", "default"];

/** A shape of value: its type, how one numbered `n` is made, an
 * expression, and what it has. */
type Shape = { name: string; ty: string; make: (n: string) => string; has: Trait[] };

export const shapes: Shape[] = [
  { name: "D", ty: "D", make: (n) => `d(${n})`, has: all },
  { name: "struct", ty: "P", make: (n) => `P { a: d(${n}), b: d(${n} + 100) }`, has: all },
  { name: "struct with Drop", ty: "W", make: (n) => `W { a: d(${n}), b: d(${n} + 100) }`, has: all },
  { name: "tuple", ty: "(D, D)", make: (n) => `(d(${n}), d(${n} + 100))`, has: all },
  { name: "enum", ty: "E", make: (n) => `e(${n})`, has: all },
  { name: "enum with Drop", ty: "F", make: (n) => `f(${n})`, has: ["clone", "eq", "ord"] },
  { name: "Box", ty: "Box<D>", make: (n) => `Box::new(d(${n}))`, has: all },
  { name: "Option", ty: "Option<D>", make: (n) => `odd(${n})`, has: all },
  { name: "Vec", ty: "Vec<D>", make: (n) => `vec![d(${n}), d(${n} + 100)]`, has: all },
  { name: "Rc", ty: "Rc<D>", make: (n) => `Rc::new(d(${n}))`, has: all },
  { name: "closure", ty: "Box<dyn Fn() -> u32>", make: (n) => `held(${n})`, has: [] },
];

/** What an operation's text is given: its shape's maker and type. */
type Of = { M: (n: number | string) => string; T: string };

/** An operation of a family: its statements, after the family's `head`
 * and before its `tail` unless it `consumes` what the head made; what its
 * shape must have; and, of one that panics of itself in some, the families
 * it's of. No case panics but at its point: a point passed as a panic of
 * its own unwinds would panic again, which aborts native Rust's run of
 * the whole program (#847). */
type Op = { name: string; needs?: Trait[]; consumes?: boolean; only?: string[]; text: (of: Of) => string };

/** A family: what each of its operations starts from and ends with. */
type Family = { name: string; needs?: Trait[]; head?: (of: Of) => string; tail?: (of: Of) => string; ops: Op[] };

const op = (name: string, text: (of: Of) => string, more: Partial<Op> = {}): Op => ({ name, text, ...more });

/** Values, their fields and temporaries. */
const value: Family = {
  name: "value",
  ops: [
    op("scope", ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\nprintln!("{} {}", x.v(), y.v());`),
    op("assign", ({ M }) => `let mut x = ${M(1)};\nprintln!("{}", x.v());\nx = ${M(2)};\nprintln!("{}", x.v());`),
    op("replace", ({ M }) => `let mut x = ${M(1)};\nlet old = std::mem::replace(&mut x, ${M(2)});\nprintln!("{} {}", old.v(), x.v());`),
    op("swap", ({ M }) => `let mut x = ${M(1)};\nlet mut y = ${M(2)};\nstd::mem::swap(&mut x, &mut y);\nprintln!("{} {}", x.v(), y.v());`),
    op("take", ({ M }) => `let mut x = ${M(1)};\nlet t = std::mem::take(&mut x);\nprintln!("{} {}", t.v(), x.v());`, { needs: ["default"] }),
    op("drop", ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\ndrop(x);\nprintln!("{}", y.v());`),
    op("arguments", ({ M }) => `println!("{}", both(${M(1)}, ${M(2)}));`),
    op("tuple", ({ M }) => `let t = (${M(1)}, ${M(2)}, ${M(3)});\nprintln!("{}", t.1.v());`),
    op("array", ({ M }) => `let a = [${M(1)}, ${M(2)}, ${M(3)}];\nprintln!("{}", show(&a));`),
    op("struct", ({ M }) => `let h = Two { x: ${M(1)}, y: ${M(2)} };\nprintln!("{}", h.y.v());`),
    op("guard", ({ M }) => `let g = Guard { x: ${M(1)}, y: ${M(2)} };\nprintln!("{}", g.y.v());`),
    op("set field", ({ M }) => `let mut h = Two { x: ${M(1)}, y: ${M(2)} };\nh.x = ${M(3)};\nprintln!("{}", h.x.v());`),
    op("set guard's field", ({ M }) => `let mut g = Guard { x: ${M(1)}, y: ${M(2)} };\ng.y = ${M(3)};\nprintln!("{}", g.y.v());`),
    op("move field", ({ M }) => `let h = Two { x: ${M(1)}, y: ${M(2)} };\nlet x = h.x;\nlet z = ${M(3)};\nprintln!("{} {}", x.v(), z.v() + h.y.v());`),
    op("pattern rest", ({ M }) => `let Two { x, .. } = Two { x: ${M(1)}, y: ${M(2)} };\nprintln!("{}", x.v());`),
    op("underscore", ({ M }) => `let _ = ${M(1)};\nlet _x = ${M(2)};\nprintln!("after");`),
    op("temporaries", ({ M }) => `println!("{}", ${M(1)}.v() + ${M(2)}.v());`),
    op("match temporary", ({ M }) => `match (Two { x: ${M(1)}, y: ${M(2)} }).x.v() {\n    n => {\n        let z = ${M(3)};\n        println!("{}", n + z.v());\n    }\n}`),
    op(
      "if let temporary",
      ({ M }) =>
        `if let Some(n) = Some(Two { x: ${M(1)}, y: ${M(2)} }.y.v()).filter(|n| n % 2 == 0) {\n    let z = ${M(3)};\n    println!("{}", n + z.v());\n} else {\n    let z = ${M(4)};\n    println!("else {}", z.v());\n}`,
    ),
    op("loop break", ({ M }) => `let mut kept = Vec::new();\nloop {\n    let x = ${M("kept.len() as u32 + 1")};\n    if kept.len() == 2 {\n        break;\n    }\n    kept.push(x);\n}\nprintln!("{}", show(&kept));`),
    op(
      "early return",
      ({ M, T }) =>
        `fn early(stop: bool) -> u32 {\n    let a: ${T} = ${M(1)};\n    if stop {\n        return a.v();\n    }\n    let b = ${M(2)};\n    a.v() + b.v()\n}\nprintln!("{} {}", early(true), early(false));`,
    ),
    op("shadow", ({ M }) => `let x = ${M(1)};\nlet x = Two { x, y: ${M(2)} };\nprintln!("{}", x.x.v());`),
  ],
};

/** Closures that hold what drops. */
const closure: Family = {
  name: "closure",
  ops: [
    op("call", ({ M }) => `let x = ${M(1)};\nlet f = move || {\n    point();\n    x.v()\n};\nprintln!("{} {}", f(), f());`),
    op("call once", ({ M }) => `let x = ${M(1)};\nlet f = move || {\n    point();\n    x\n};\nlet y = f();\nprintln!("{}", y.v());`),
    op("drop inside", ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\nlet f = move || {\n    point();\n    drop(x);\n    y.v()\n};\nprintln!("{}", f());`),
    op(
      "two",
      ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\nlet f = move || {\n    point();\n    x.v()\n};\nlet g = move || {\n    point();\n    y.v()\n};\nprintln!("{}", g() + f());`,
    ),
    op("borrows", ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\nlet f = || {\n    point();\n    x.v() + y.v()\n};\nprintln!("{}", f() + f());`),
    op("boxed", ({ M }) => `let x = ${M(1)};\nlet f: Box<dyn Fn() -> u32> = Box::new(move || {\n    point();\n    x.v()\n});\nprintln!("{}", f());`),
    op("returned", ({ M }) => `let f = capture(${M(1)}, ${M(2)});\nprintln!("{}", f() + f());`),
    op("given", ({ M }) => `let x = ${M(1)};\nprintln!("{}", twice(move || {\n    point();\n    x.v()\n}));`),
    op(
      "given once",
      ({ M }) => `let x = ${M(1)};\nlet y = ${M(2)};\nprintln!("{}", once(move || {\n    point();\n    let z = x;\n    z.v() + y.v()\n}));`,
    ),
    op(
      "pushes",
      ({ M, T }) =>
        `let mut kept: Vec<${T}> = Vec::new();\nlet mut push = |n: u32| {\n    point();\n    kept.push(${M("n")});\n};\npush(1);\npush(2);\nprintln!("{}", show(&kept));`,
    ),
    op("sets", ({ M }) => `let mut x = ${M(1)};\nlet mut set = |n: u32| {\n    point();\n    x = ${M("n")};\n};\nset(2);\nset(3);\nprintln!("{}", x.v());`),
    op("in a tuple", ({ M }) => `let x = ${M(1)};\nlet h = (\n    move || {\n        point();\n        x.v()\n    },\n    ${M(2)},\n);\nprintln!("{}", (h.0)() + h.1.v());`),
    op(
      "in a Vec",
      ({ M }) =>
        `let x = ${M(1)};\nlet y = ${M(2)};\nlet fs: Vec<Box<dyn Fn() -> u32>> = vec![\n    Box::new(move || {\n        point();\n        x.v()\n    }),\n    Box::new(move || {\n        point();\n        y.v()\n    }),\n];\nprintln!("{}", fs[1]() + fs[0]());`,
    ),
    op("in an Option", ({ M }) => `let x = ${M(1)};\nlet f = Some(move || {\n    point();\n    x.v()\n});\nprintln!("{}", f.map_or(0, |f| f()));`),
  ],
};

/** A `Vec`'s operations. */
const vec: Family = {
  name: "Vec",
  head: ({ M, T }) => `let mut v: Vec<${T}> = vec![${M(3)}, ${M(1)}, ${M(4)}, ${M(2)}];`,
  tail: () => `println!("{}", show(&v));`,
  ops: [
    op("drop", () => ""),
    op("push", ({ M }) => `v.push(${M(5)});`),
    op("pop", () => `let p = v.pop();\nprintln!("{}", show_option(&p));`),
    op("insert", ({ M }) => `v.insert(1, ${M(5)});`),
    op("remove", () => `let r = v.remove(1);\nprintln!("{}", r.v());`),
    op("swap_remove", () => `let r = v.swap_remove(0);\nprintln!("{}", r.v());`),
    op("truncate", () => `v.truncate(1);`),
    op("clear", () => `v.clear();`),
    op("retain", () => `v.retain(|x| {\n    point();\n    x.v() % 2 == 1\n});`),
    op("retain_mut", () => `v.retain_mut(|x| {\n    point();\n    x.v() % 2 == 0\n});`),
    op("dedup_by_key", () => `v.dedup_by_key(|x| {\n    point();\n    x.v() % 2\n});`),
    op("dedup_by", () => `v.dedup_by(|a, b| {\n    point();\n    a.v() % 2 == b.v() % 2\n});`),
    op("dedup", () => `v.dedup();`, { needs: ["eq"] }),
    op("drain", () => `v.drain(1..3);`),
    op("drain collected", ({ T }) => `let gone: Vec<${T}> = v.drain(1..3).collect();\nprintln!("{}", show(&gone));`),
    op("split_off", () => `let tail = v.split_off(2);\nprintln!("{}", show(&tail));`),
    op("append", ({ M }) => `let mut w = vec![${M(5)}, ${M(6)}];\nv.append(&mut w);`),
    op("extend", ({ M }) => `v.extend(vec![${M(5)}, ${M(6)}]);`),
    op("splice", ({ M, T }) => `let gone: Vec<${T}> = v.splice(1..3, vec![${M(8)}]).collect();\nprintln!("{}", show(&gone));`),
    op("extend_from_slice", ({ M }) => `let w = vec![${M(5)}, ${M(6)}];\nv.extend_from_slice(&w);`, { needs: ["clone"] }),
    op("resize longer", ({ M }) => `v.resize(6, ${M(9)});`, { needs: ["clone"] }),
    op("resize shorter", ({ M }) => `v.resize(2, ${M(9)});`, { needs: ["clone"] }),
    op("resize_with", ({ M }) => `v.resize_with(6, || {\n    point();\n    ${M(9)}\n});`),
    op("fill", ({ M }) => `v.fill(${M(9)});`, { needs: ["clone"] }),
    op("clone", () => `let w = v.clone();\nprintln!("{}", show(&w));`, { needs: ["clone"] }),
    op("clone_from", ({ M }) => `let mut w = vec![${M(7)}];\nw.clone_from(&v);\nprintln!("{}", show(&w));`, { needs: ["clone"] }),
    op("repeated", ({ M }) => `let w = vec![${M(9)}; 3];\nprintln!("{}", show(&w));`, { needs: ["clone"] }),
    op("to_vec", () => `let w = v[1..3].to_vec();\nprintln!("{}", show(&w));`, { needs: ["clone"] }),
    op("set index", ({ M }) => `v[1] = ${M(9)};`),
    op("set last_mut", ({ M }) => `if let Some(x) = v.last_mut() {\n    *x = ${M(9)};\n}`),
    op("swap", () => `v.swap(0, 3);`),
    op("reverse", () => `v.reverse();`),
    op("rotate_left", () => `v.rotate_left(1);`),
    op("into_boxed_slice", () => `let b = v.into_boxed_slice();\nprintln!("{}", show(&b));`, { consumes: true }),
    op("contains", ({ M }) => `println!("{}", v.contains(&${M(4)}));`, { needs: ["eq"] }),
    op("eq", ({ M }) => `let w = vec![${M(3)}, ${M(1)}, ${M(5)}];\nprintln!("{}", v == w);`, { needs: ["eq"] }),
    op("cmp", ({ M }) => `let w = vec![${M(3)}, ${M(1)}, ${M(5)}];\nprintln!("{:?}", v.cmp(&w));`, { needs: ["ord"] }),
    op("sort", () => `v.sort();`, { needs: ["ord"] }),
    op("sort_unstable", () => `v.sort_unstable();`, { needs: ["ord"] }),
    op("sort_by", () => `v.sort_by(|a, b| {\n    point();\n    a.v().cmp(&b.v())\n});`),
    op("sort_by_key", () => `v.sort_by_key(|x| {\n    point();\n    x.v() % 3\n});`),
    op("sort_by_cached_key", () => `v.sort_by_cached_key(|x| {\n    point();\n    x.v() % 3\n});`),
    op("sort_unstable_by", () => `v.sort_unstable_by(|a, b| {\n    point();\n    b.v().cmp(&a.v())\n});`),
    op("sort_unstable_by_key", () => `v.sort_unstable_by_key(|x| {\n    point();\n    x.v() % 3\n});`),
    op("binary_search_by", () => `println!("{:?}", v.binary_search_by(|x| {\n    point();\n    x.v().cmp(&2)\n}));`),
    op("partition_point", () => `println!("{}", v.partition_point(|x| {\n    point();\n    x.v() < 3\n}));`),
    op(
      "select_nth_unstable_by",
      () => `v.select_nth_unstable_by(1, |a, b| {\n    point();\n    a.v().cmp(&b.v())\n});`,
    ),
  ],
};

/** A `VecDeque`'s, of one that wraps around its buffer. */
const deque: Family = {
  name: "VecDeque",
  head: ({ M, T }) => `let mut q: VecDeque<${T}> = VecDeque::new();\nq.push_back(${M(1)});\nq.push_back(${M(2)});\nq.push_front(${M(3)});`,
  tail: () => `println!("{}", show_deque(&q));`,
  ops: [
    op("drop", () => ""),
    op("push_front", ({ M }) => `q.push_front(${M(5)});`),
    op("push_back", ({ M }) => `q.push_back(${M(5)});`),
    op("pop_front", () => `let p = q.pop_front();\nprintln!("{}", show_option(&p));`),
    op("pop_back", () => `let p = q.pop_back();\nprintln!("{}", show_option(&p));`),
    op("insert", ({ M }) => `q.insert(1, ${M(5)});`),
    op("remove", () => `let r = q.remove(1);\nprintln!("{}", show_option(&r));`),
    op("swap_remove_back", () => `let r = q.swap_remove_back(0);\nprintln!("{}", show_option(&r));`),
    op("truncate", () => `q.truncate(1);`),
    op("clear", () => `q.clear();`),
    op("retain", () => `q.retain(|x| {\n    point();\n    x.v() % 2 == 1\n});`),
    op("retain_mut", () => `q.retain_mut(|x| {\n    point();\n    x.v() % 2 == 0\n});`),
    op("drain", () => `q.drain(1..);`),
    op("split_off", () => `let tail = q.split_off(1);\nprintln!("{}", show_deque(&tail));`),
    op("append", ({ M, T }) => `let mut w: VecDeque<${T}> = VecDeque::new();\nw.push_back(${M(5)});\nq.append(&mut w);`),
    op("extend", ({ M }) => `q.extend(vec![${M(5)}, ${M(6)}]);`),
    op("rotate_left", () => `q.rotate_left(1);`),
    op("make_contiguous sort_by", () => `q.make_contiguous().sort_by(|a, b| {\n    point();\n    a.v().cmp(&b.v())\n});`),
    op("resize", ({ M }) => `q.resize(5, ${M(9)});`, { needs: ["clone"] }),
    op("resize_with", ({ M }) => `q.resize_with(5, || {\n    point();\n    ${M(9)}\n});`),
    op("clone", () => `let w = q.clone();\nprintln!("{}", show_deque(&w));`, { needs: ["clone"] }),
    op("set index", ({ M }) => `q[1] = ${M(9)};`),
    op("contains", ({ M }) => `println!("{}", q.contains(&${M(2)}));`, { needs: ["eq"] }),
    op("eq", () => `let w = q.clone();\nprintln!("{}", q == w);`, { needs: ["clone", "eq"] }),
  ],
};

/** A `BTreeMap`'s, keyed by number, so only its values' code runs. */
const btree: Family = {
  name: "BTreeMap",
  head: ({ M, T }) => `let mut m: BTreeMap<u32, ${T}> = BTreeMap::new();\nm.insert(2, ${M(2)});\nm.insert(1, ${M(1)});\nm.insert(3, ${M(3)});`,
  tail: () => `println!("{}", show_map(&m));`,
  ops: [
    op("drop", () => ""),
    op("insert", ({ M }) => `m.insert(4, ${M(4)});`),
    op("insert again", ({ M }) => `let old = m.insert(2, ${M(5)});\nprintln!("{}", show_option(&old));`),
    op("insert again, dropped", ({ M }) => `m.insert(2, ${M(5)});`),
    op("remove", () => `let r = m.remove(&2);\nprintln!("{}", show_option(&r));`),
    op("remove, dropped", () => `m.remove(&2);`),
    op("retain", () => `m.retain(|k, x| {\n    point();\n    (k + x.v()) % 2 == 0\n});`),
    op("or_insert_with, there", ({ M }) => `m.entry(2).or_insert_with(|| {\n    point();\n    ${M(9)}\n});`),
    op("or_insert_with, not there", ({ M }) => `m.entry(5).or_insert_with(|| {\n    point();\n    ${M(9)}\n});`),
    op("and_modify", ({ M }) => `m.entry(2)\n    .and_modify(|x| *x = ${M(9)})\n    .or_insert_with(|| ${M(8)});`),
    op("set get_mut", ({ M }) => `if let Some(x) = m.get_mut(&1) {\n    *x = ${M(9)};\n}`),
    op("split_off", () => `let tail = m.split_off(&2);\nprintln!("{}", show_map(&tail));`),
    op("append", ({ M }) => `let mut o = BTreeMap::new();\no.insert(2, ${M(7)});\no.insert(5, ${M(8)});\nm.append(&mut o);`),
    op("extend", ({ M }) => `m.extend(vec![(2, ${M(7)}), (6, ${M(8)})]);`),
    op("pop_first", () => `let p = m.pop_first();\nprintln!("{}", p.map_or(0, |(k, x)| k + x.v()));`),
    op("pop_last", () => `let p = m.pop_last();\nprintln!("{}", p.map_or(0, |(k, x)| k + x.v()));`),
    op("clear", () => `m.clear();`),
    op("clone", () => `let w = m.clone();\nprintln!("{}", show_map(&w));`, { needs: ["clone"] }),
  ],
};

/** A `BTreeMap`'s keyed by the shape, which compare by its own `Ord`. */
const keyed: Family = {
  name: "BTreeMap keys",
  needs: ["ord"],
  head: ({ M, T }) => `let mut m: BTreeMap<${T}, u32> = BTreeMap::new();\nm.insert(${M(2)}, 2);\nm.insert(${M(1)}, 1);\nm.insert(${M(3)}, 3);`,
  tail: () => `println!("{}", show_keys!(&m));`,
  ops: [
    op("drop", () => ""),
    op("insert", ({ M }) => `m.insert(${M(4)}, 4);`),
    op("insert again", ({ M }) => `let old = m.insert(${M(2)}, 9);\nprintln!("{:?}", old);`),
    op("remove", ({ M }) => `let r = m.remove(&${M(2)});\nprintln!("{:?}", r);`),
    op("get", ({ M }) => `println!("{:?}", m.get(&${M(1)}));`),
    op("contains_key", ({ M }) => `println!("{}", m.contains_key(&${M(5)}));`),
    op("entry", ({ M }) => `*m.entry(${M(2)}).or_insert(0) += 10;\n*m.entry(${M(5)}).or_insert(0) += 10;`),
    op("retain", () => `m.retain(|k, x| {\n    point();\n    (k.v() + *x) % 2 == 0\n});`),
    op("split_off", ({ M }) => `let tail = m.split_off(&${M(2)});\nprintln!("{}", show_keys!(&tail));`),
    op("clear", () => `m.clear();`),
    op("clone", () => `let w = m.clone();\nprintln!("{}", show_keys!(&w));`, { needs: ["clone"] }),
  ],
};

/** A `BTreeSet`'s, of the shape's own `Ord`. */
const set: Family = {
  name: "BTreeSet",
  needs: ["ord"],
  head: ({ M, T }) => `let mut s: BTreeSet<${T}> = BTreeSet::new();\ns.insert(${M(2)});\ns.insert(${M(1)});\ns.insert(${M(3)});`,
  tail: () => `println!("{}", show_set!(&s));`,
  ops: [
    op("drop", () => ""),
    op("insert", ({ M }) => `println!("{}", s.insert(${M(4)}));`),
    op("insert again", ({ M }) => `println!("{}", s.insert(${M(2)}));`),
    op("replace", ({ M }) => `let old = s.replace(${M(2)});\nprintln!("{}", show_option(&old));`),
    op("remove", ({ M }) => `println!("{}", s.remove(&${M(2)}));`),
    op("take", ({ M }) => `let t = s.take(&${M(2)});\nprintln!("{}", show_option(&t));`),
    op("contains", ({ M }) => `println!("{}", s.contains(&${M(5)}));`),
    op("retain", () => `s.retain(|x| {\n    point();\n    x.v() % 2 == 1\n});`),
    op("split_off", ({ M }) => `let tail = s.split_off(&${M(2)});\nprintln!("{}", show_set!(&tail));`),
    op("clear", () => `s.clear();`),
    op("clone", () => `let w = s.clone();\nprintln!("{}", show_set!(&w));`, { needs: ["clone"] }),
  ],
};

/** A `HashMap`'s, of one entry at a time, whose order no run depends on. */
const hash: Family = {
  name: "HashMap",
  head: ({ M, T }) => `let mut m: HashMap<u32, ${T}> = HashMap::new();\nm.insert(1, ${M(1)});`,
  tail: () => `println!("{}", m.get(&1).map_or(0, |x| x.v()));`,
  ops: [
    op("drop", () => ""),
    op("insert again", ({ M }) => `let old = m.insert(1, ${M(2)});\nprintln!("{}", show_option(&old));`),
    op("insert again, dropped", ({ M }) => `m.insert(1, ${M(2)});`),
    op("remove", () => `let r = m.remove(&1);\nprintln!("{}", show_option(&r));`),
    op("remove, dropped", () => `m.remove(&1);`),
    op("or_insert_with, there", ({ M }) => `m.entry(1).or_insert_with(|| {\n    point();\n    ${M(9)}\n});`),
    op("or_insert_with, not there", ({ M }) => `m.clear();\nm.entry(1).or_insert_with(|| {\n    point();\n    ${M(9)}\n});`),
    op("and_modify", ({ M }) => `m.entry(1)\n    .and_modify(|x| *x = ${M(9)})\n    .or_insert_with(|| ${M(8)});`),
    op("set get_mut", ({ M }) => `if let Some(x) = m.get_mut(&1) {\n    *x = ${M(9)};\n}`),
    op("retain", () => `m.retain(|_, x| {\n    point();\n    x.v() % 2 == 0\n});`),
    op("extend", ({ M }) => `m.extend(vec![(1, ${M(7)})]);`),
    op("clear", () => `m.clear();`),
    op("clone", () => `let w = m.clone();\nprintln!("{}", w.get(&1).map_or(0, |x| x.v()));`, { needs: ["clone"] }),
  ],
};

/** A `BinaryHeap`'s, which compare by the shape's own `Ord`. */
const heap: Family = {
  name: "BinaryHeap",
  needs: ["ord"],
  head: ({ M, T }) => `let mut h: BinaryHeap<${T}> = BinaryHeap::new();\nh.push(${M(3)});\nh.push(${M(1)});\nh.push(${M(4)});`,
  tail: () => `println!("{}", h.len());`,
  ops: [
    op("drop", () => ""),
    op("push", ({ M }) => `h.push(${M(2)});`),
    op("pop", () => `let p = h.pop();\nprintln!("{}", show_option(&p));`),
    op("set peek_mut", ({ M }) => `if let Some(mut top) = h.peek_mut() {\n    *top = ${M(0)};\n}`),
    op("retain", () => `h.retain(|x| {\n    point();\n    x.v() % 2 == 1\n});`),
    op("append", ({ M, T }) => `let mut o: BinaryHeap<${T}> = BinaryHeap::new();\no.push(${M(2)});\nh.append(&mut o);`),
    op("extend", ({ M }) => `h.extend(vec![${M(2)}, ${M(5)}]);`),
    op("drain", () => `h.drain();`),
    op("clear", () => `h.clear();`),
    op("into_sorted_vec", () => `let v = h.into_sorted_vec();\nprintln!("{}", show(&v));`, { consumes: true }),
    op("into_vec", () => `let v = h.into_vec();\nprintln!("{}", show(&v));`, { consumes: true }),
    op("clone", () => `let w = h.clone();\nprintln!("{}", w.len());`, { needs: ["clone"] }),
  ],
};

/** An `Option`'s combinators, of `Some` and of `None`. */
const optionOps: Op[] = [
  op("map", () => `let r = o.map(|x| {\n    point();\n    x.v()\n});\nprintln!("{:?}", r);`),
  op("map kept", ({ M }) => `let r = o.map(|x| {\n    point();\n    Two { x, y: ${M(7)} }\n});\nprintln!("{}", r.map_or(0, |t| t.x.v() + t.y.v()));`),
  op("map holding", ({ M }) => `let g = ${M(50)};\nlet r = o.map(move |x| {\n    point();\n    x.v() + g.v()\n});\nprintln!("{:?}", r);`),
  op("and_then", ({ M }) => `let r = o.and_then(|x| {\n    point();\n    if x.v() < 1_000_000 { Some(${M(8)}) } else { None }\n});\nprintln!("{}", show_option(&r));`),
  op("and_then to None", ({ T }) => `let r = o.and_then(|x| {\n    point();\n    println!("saw {}", x.v());\n    None::<${T}>\n});\nprintln!("{}", show_option(&r));`),
  op("unwrap_or", ({ M }) => `let r = o.unwrap_or(${M(9)});\nprintln!("{}", r.v());`),
  op("unwrap_or_else", ({ M }) => `let r = o.unwrap_or_else(|| {\n    point();\n    ${M(9)}\n});\nprintln!("{}", r.v());`),
  op(
    "unwrap_or_else holding",
    ({ M }) => `let g = ${M(50)};\nlet r = o.unwrap_or_else(move || {\n    point();\n    let k = g;\n    println!("had {}", k.v());\n    ${M(9)}\n});\nprintln!("{}", r.v());`,
  ),
  op("unwrap_or_default", () => `let r = o.unwrap_or_default();\nprintln!("{}", r.v());`, { needs: ["default"] }),
  op("map_or", () => `let r = o.map_or(0, |x| {\n    point();\n    x.v()\n});\nprintln!("{r}");`),
  op("map_or_else", () => `let r = o.map_or_else(\n    || {\n        point();\n        0\n    },\n    |x| {\n        point();\n        x.v()\n    },\n);\nprintln!("{r}");`),
  op("filter", () => `let r = o.filter(|x| {\n    point();\n    x.v() % 2 == 0\n});\nprintln!("{}", show_option(&r));`),
  op("take", () => `let mut o = o;\nlet t = o.take();\nprintln!("{} {}", show_option(&t), show_option(&o));`),
  op("replace", ({ M }) => `let mut o = o;\nlet old = o.replace(${M(9)});\nprintln!("{} {}", show_option(&old), show_option(&o));`),
  op("insert", ({ M }) => `let mut o = o;\nlet r = o.insert(${M(9)}).v();\nprintln!("{r}");`),
  op("get_or_insert_with", ({ M }) => `let mut o = o;\nlet r = o\n    .get_or_insert_with(|| {\n        point();\n        ${M(9)}\n    })\n    .v();\nprintln!("{r}");`),
  op("zip", ({ M }) => `let r = o.zip(Some(${M(9)}));\nprintln!("{}", r.map_or(0, |(a, b)| a.v() + b.v()));`),
  op("zip None", ({ T }) => `let r = o.zip(None::<${T}>);\nprintln!("{}", r.map_or(0, |(a, b)| a.v() + b.v()));`),
  op("xor", ({ M }) => `let r = o.xor(Some(${M(9)}));\nprintln!("{}", show_option(&r));`),
  op("or", ({ M }) => `let r = o.or(Some(${M(9)}));\nprintln!("{}", show_option(&r));`),
  op("or_else", ({ M }) => `let r = o.or_else(|| {\n    point();\n    Some(${M(9)})\n});\nprintln!("{}", show_option(&r));`),
  op("and", ({ M }) => `let r = o.and(Some(${M(9)}));\nprintln!("{}", show_option(&r));`),
  op("ok_or", () => `let r = o.ok_or(d(9));\nprintln!("{}", show_result(&r));`),
  op("ok_or_else", () => `let r = o.ok_or_else(|| {\n    point();\n    d(9)\n});\nprintln!("{}", show_result(&r));`),
  op("is_some_and", () => `let r = o.is_some_and(|x| {\n    point();\n    x.v() > 0\n});\nprintln!("{r}");`),
  op("is_none_or", () => `let r = o.is_none_or(|x| {\n    point();\n    x.v() > 0\n});\nprintln!("{r}");`),
  op("inspect", () => `let r = o.inspect(|x| {\n    point();\n    println!("saw {}", x.v());\n});\nprintln!("{}", show_option(&r));`),
  op("take_if", () => `let mut o = o;\nlet t = o.take_if(|x| {\n    point();\n    x.v() % 2 == 1\n});\nprintln!("{} {}", show_option(&t), show_option(&o));`),
  op("set as_mut", ({ M }) => `let mut o = o;\nif let Some(x) = o.as_mut() {\n    *x = ${M(9)};\n}\nprintln!("{}", show_option(&o));`),
  op("as_ref map", () => `let r = o.as_ref().map(|x| {\n    point();\n    x.v()\n});\nprintln!("{:?} {}", r, show_option(&o));`),
  op("flatten", () => `let r = Some(o).flatten();\nprintln!("{}", show_option(&r));`),
  op("unwrap", () => `let r = o.unwrap();\nprintln!("{}", r.v());`, { only: ["Some"] }),
  op("clone", () => `let c = o.clone();\nprintln!("{} {}", show_option(&c), show_option(&o));`, { needs: ["clone"] }),
  op("eq", ({ M }) => `println!("{}", o == Some(${M(1)}));`, { needs: ["eq"] }),
  op("cmp", ({ M }) => `println!("{:?}", o.cmp(&Some(${M(2)})));`, { needs: ["ord"] }),
];

const option = (some: boolean): Family => ({
  name: some ? "Some" : "None",
  head: ({ M, T }) => `let o: Option<${T}> = ${some ? `Some(${M(1)})` : "None"};`,
  ops: optionOps,
});

/** A `Result`'s combinators, of `Ok` and of `Err`, whose error is a `D`. */
const resultOps: Op[] = [
  op("map", () => `let r = r.map(|x| {\n    point();\n    x.v()\n});\nprintln!("{}", show_result(&r));`),
  op("map_err", () => `let r = r.map_err(|err| {\n    point();\n    err.0 + 1000\n});\nprintln!("{}", show_result(&r));`),
  op("map_err holding", ({ M }) => `let g = ${M(50)};\nlet r = r.map_err(move |err| {\n    point();\n    err.0 + g.v()\n});\nprintln!("{}", show_result(&r));`),
  op("and_then", ({ M }) => `let r = r.and_then(|x| {\n    point();\n    if x.v() < 1_000_000 { Ok(${M(8)}) } else { Err(d(8)) }\n});\nprintln!("{}", show_result(&r));`),
  op("and_then to Err", ({ T }) => `let r = r.and_then(|x| {\n    point();\n    println!("saw {}", x.v());\n    Err::<${T}, D>(d(8))\n});\nprintln!("{}", show_result(&r));`),
  op("or_else", ({ M }) => `let r = r.or_else(|err| {\n    point();\n    if err.0 > 0 { Ok::<_, D>(${M(8)}) } else { Err(err) }\n});\nprintln!("{}", show_result(&r));`),
  op("unwrap_or", ({ M }) => `let r = r.unwrap_or(${M(9)});\nprintln!("{}", r.v());`),
  op("unwrap_or_else", ({ M }) => `let r = r.unwrap_or_else(|err| {\n    point();\n    ${M("err.0 + 10")}\n});\nprintln!("{}", r.v());`),
  op("unwrap_or_default", () => `let r = r.unwrap_or_default();\nprintln!("{}", r.v());`, { needs: ["default"] }),
  op("ok", () => `let r = r.ok();\nprintln!("{}", show_option(&r));`),
  op("err", () => `let r = r.err();\nprintln!("{}", show_option(&r));`),
  op("and", ({ M, T }) => `let r = r.and(Ok::<${T}, D>(${M(9)}));\nprintln!("{}", show_result(&r));`),
  op("or", ({ M, T }) => `let r = r.or(Ok::<${T}, D>(${M(9)}));\nprintln!("{}", show_result(&r));`),
  op("map_or", () => `let r = r.map_or(0, |x| {\n    point();\n    x.v()\n});\nprintln!("{r}");`),
  op("map_or_else", () => `let r = r.map_or_else(\n    |err| {\n        point();\n        err.0\n    },\n    |x| {\n        point();\n        x.v()\n    },\n);\nprintln!("{r}");`),
  op("is_ok_and", () => `let r = r.is_ok_and(|x| {\n    point();\n    x.v() > 0\n});\nprintln!("{r}");`),
  op("is_err_and", () => `let r = r.is_err_and(|err| {\n    point();\n    err.0 > 0\n});\nprintln!("{r}");`),
  op("inspect", () => `let r = r.inspect(|x| {\n    point();\n    println!("saw {}", x.v());\n});\nprintln!("{}", show_result(&r));`),
  op("inspect_err", () => `let r = r.inspect_err(|err| {\n    point();\n    println!("saw {}", err.0);\n});\nprintln!("{}", show_result(&r));`),
  op("set as_mut", ({ M }) => `let mut r = r;\nif let Ok(x) = r.as_mut() {\n    *x = ${M(9)};\n}\nprintln!("{}", show_result(&r));`),
  op("unwrap", () => `let r = r.unwrap();\nprintln!("{}", r.v());`, { only: ["Ok"] }),
  op("clone", () => `let c = r.clone();\nprintln!("{} {}", show_result(&c), show_result(&r));`, { needs: ["clone"] }),
  op("eq", ({ M }) => `println!("{}", r == Ok(${M(1)}));`, { needs: ["eq"] }),
];

const result = (ok: boolean): Family => ({
  name: ok ? "Ok" : "Err",
  head: ({ M, T }) => `let r: Result<${T}, D> = ${ok ? `Ok(${M(1)})` : "Err(d(2))"};`,
  ops: resultOps,
});

/** `?`, of a step that fails at its third, or never: each a `fn run` the
 * case calls, whose result it prints. */
const tryOps: Op[] = [
  op("locals", ({ M }) => `fn run() -> Result<u32, D> {\n    let a = ${M(1)};\n    let b = step(2)?;\n    let c = step(3)?;\n    let e = ${M(4)};\n    Ok(a.v() + b.v() + c.v() + e.v())\n}`),
  op("struct", () => `fn run() -> Result<u32, D> {\n    let h = Two { x: step(2)?, y: step(3)? };\n    Ok(h.x.v() + h.y.v())\n}`),
  op("struct made", ({ M }) => `fn run() -> Result<u32, D> {\n    Ok(Two { x: ${M(1)}, y: step(3)? }.y.v())\n}`),
  op("guard made", ({ M }) => `fn run() -> Result<u32, D> {\n    Ok(Guard { x: ${M(1)}, y: step(3)? }.y.v())\n}`),
  op("arguments", ({ M }) => `fn run() -> Result<u32, D> {\n    Ok(both(${M(1)}, step(3)?))\n}`),
  op("vec!", ({ M }) => `fn run() -> Result<u32, D> {\n    let v = vec![${M(1)}, step(3)?, ${M(4)}];\n    Ok(show(&v).len() as u32)\n}`),
  op("tuple", ({ M }) => `fn run() -> Result<u32, D> {\n    let t = (${M(1)}, step(3)?, ${M(4)});\n    Ok(t.1.v())\n}`),
  op("loop", () => `fn run() -> Result<u32, D> {\n    let mut v = Vec::new();\n    for i in 1..5 {\n        v.push(step(i)?);\n    }\n    Ok(show(&v).len() as u32)\n}`),
  op("From", ({ M }) => `fn run() -> Result<u32, Big> {\n    let a = ${M(1)};\n    let b = step(3)?;\n    Ok(a.v() + b.v())\n}`),
  op("Option", ({ M }) => `fn run() -> Option<u32> {\n    let a = ${M(1)};\n    let b = step(3).ok()?;\n    Some(a.v() + b.v())\n}`),
  op(
    "closure",
    ({ M }) =>
      `fn run() -> Result<u32, D> {\n    let f = |n: u32| -> Result<u32, D> {\n        let a = ${M("n")};\n        let b = step(n + 2)?;\n        Ok(a.v() + b.v())\n    };\n    f(1)\n}`,
  ),
  op(
    "match arm",
    () =>
      `fn run() -> Result<u32, D> {\n    match step(2)? {\n        x if x.v() < 1_000_000 => {\n            let y = step(3)?;\n            Ok(x.v() + y.v())\n        }\n        x => Ok(x.v()),\n    }\n}`,
  ),
  op("guard", ({ M }) => `fn run() -> Result<u32, D> {\n    let g = Guard { x: ${M(1)}, y: ${M(2)} };\n    let b = step(3)?;\n    Ok(g.x.v() + b.v())\n}`),
  op("temporaries", ({ M }) => `fn run() -> Result<u32, D> {\n    Ok(Two { x: ${M(1)}, y: ${M(2)} }.x.v() + step(3)?.v())\n}`),
];

const tried = (fails: boolean): Family => ({
  name: fails ? "? failing" : "? passing",
  head: ({ M, T }) =>
    `fn step(n: u32) -> Result<${T}, D> {\n    point();\n    if n == ${fails ? 3 : 0} { Err(d(90)) } else { Ok(${M("n")}) }\n}`,
  tail: () => `println!("{}", run().said());`,
  ops: tryOps,
});

/** Comparisons of two values, by the shape's own `PartialEq` and `Ord`. */
const compare: Family = {
  name: "compare",
  ops: [
    op("max", ({ M }) => `let m = std::cmp::max(${M(1)}, ${M(2)});\nprintln!("{}", m.v());`, { needs: ["ord"] }),
    op("min", ({ M }) => `let m = std::cmp::min(${M(2)}, ${M(1)});\nprintln!("{}", m.v());`, { needs: ["ord"] }),
    op("Ord::max", ({ M }) => `let m = ${M(1)}.max(${M(2)});\nprintln!("{}", m.v());`, { needs: ["ord"] }),
    // Its bounds in order in every shape, an enum's `Zero` and `Two`.
    op("clamp", ({ M }) => `let m = ${M(4)}.clamp(${M(3)}, ${M(5)});\nprintln!("{}", m.v());`, { needs: ["ord"] }),
    op("cmp", ({ M }) => `println!("{:?}", ${M(1)}.cmp(&${M(2)}));`, { needs: ["ord"] }),
    op("partial_cmp", ({ M }) => `println!("{:?}", ${M(2)}.partial_cmp(&${M(2)}));`, { needs: ["ord"] }),
    op("lt", ({ M }) => `println!("{}", ${M(1)} < ${M(2)});`, { needs: ["ord"] }),
    op("eq", ({ M }) => `println!("{}", ${M(1)} == ${M(1)});`, { needs: ["eq"] }),
    op("ne", ({ M }) => `println!("{}", ${M(1)} != ${M(2)});`, { needs: ["eq"] }),
  ],
};

export const families: Family[] = [value, closure, vec, deque, btree, keyed, set, hash, heap, option(true), option(false), result(true), result(false), tried(true), tried(false), compare];

/** A case of the unwind table: its family's operation, of a shape. */
export type UnwindCase = Row & { family: Family; op: Op; shape: Shape };

const has = (shape: Shape, needs: Trait[] = []) => needs.every((t) => shape.has.includes(t));

/** Every case of the unwind table, in one order: each family's operations,
 * each of every shape that has what it needs. */
export function unwindCases(): UnwindCase[] {
  const out: UnwindCase[] = [];
  for (const family of families) {
    for (const o of family.ops) {
      for (const shape of shapes) {
        if (!has(shape, family.needs) || !has(shape, o.needs) || (o.only !== undefined && !o.only.includes(family.name))) continue;
        const id = `${family.name} ${o.name} | ${shape.name} | points`;
        out.push({ n: out.length, id, family, op: o, shape });
      }
    }
  }
  return out;
}

const indent = (text: string) =>
  text
    .split("\n")
    .map((line) => (line === "" ? "" : `    ${line}`))
    .join("\n");

/** A case's function, `case_<i>`. */
function caseFn(c: UnwindCase, i: number): string {
  const of: Of = { M: (n) => c.shape.make(String(n)), T: c.shape.ty };
  const parts = [c.family.head?.(of), c.op.text(of), c.op.consumes ? undefined : c.family.tail?.(of)].filter((p) => p !== undefined && p !== "");
  return `fn case_${i}() {\n${indent(parts.join("\n"))}\n}`;
}

const prelude = `// Generated by test/unwind-points.ts (#854). Each case prints its name,
// what it makes, clones, compares and drops, and how each run of it ended,
// a run a panic point.
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, VecDeque};
use std::panic::catch_unwind;
use std::rc::Rc;
use std::sync::atomic::{AtomicU32, Ordering};

/** A value that prints as it's made, dropped, cloned and compared, each a point. */
#[derive(Debug)]
struct D(u32);

fn d(n: u32) -> D {
    println!("make {n}");
    point();
    D(n)
}

impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
        point();
    }
}

impl Clone for D {
    fn clone(&self) -> D {
        println!("clone {}", self.0);
        point();
        D(self.0)
    }
}

impl PartialEq for D {
    fn eq(&self, other: &D) -> bool {
        println!("eq {} {}", self.0, other.0);
        point();
        self.0 == other.0
    }
}

impl Eq for D {}

impl PartialOrd for D {
    fn partial_cmp(&self, other: &D) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for D {
    fn cmp(&self, other: &D) -> std::cmp::Ordering {
        println!("cmp {} {}", self.0, other.0);
        point();
        self.0.cmp(&other.0)
    }
}

impl Default for D {
    fn default() -> D {
        d(0)
    }
}

/** A struct of two. */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct P {
    a: D,
    b: D,
}

/** A struct of two with a \`Drop\` of its own, which runs before its fields'. */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
struct W {
    a: D,
    b: D,
}

impl Drop for W {
    fn drop(&mut self) {
        println!("drop W {}", self.a.0);
        point();
    }
}

/** An enum, of none, one and two. */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Default)]
enum E {
    #[default]
    Zero,
    One(D),
    Two { a: D, b: D },
}

fn e(n: u32) -> E {
    match n % 3 {
        0 => E::Zero,
        1 => E::One(d(n)),
        _ => E::Two { a: d(n), b: d(n + 100) },
    }
}

/** An enum with a \`Drop\` of its own. */
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
enum F {
    One(D),
    Two(D, D),
}

impl Drop for F {
    fn drop(&mut self) {
        println!("drop F {}", self.v());
        point();
    }
}

fn f(n: u32) -> F {
    if n % 2 == 1 { F::One(d(n)) } else { F::Two(d(n), d(n + 100)) }
}

fn odd(n: u32) -> Option<D> {
    if n % 2 == 1 { Some(d(n)) } else { None }
}

/** A closure that holds a \`D\`, its call a point. */
fn held(n: u32) -> Box<dyn Fn() -> u32> {
    let x = d(n);
    Box::new(move || {
        point();
        x.0
    })
}

/** A value's number. */
trait V {
    fn v(&self) -> u32;
}

impl V for u32 {
    fn v(&self) -> u32 {
        *self
    }
}

impl V for D {
    fn v(&self) -> u32 {
        self.0
    }
}

impl V for P {
    fn v(&self) -> u32 {
        self.a.0 * 1000 + self.b.0
    }
}

impl V for W {
    fn v(&self) -> u32 {
        self.a.0 * 1000 + self.b.0
    }
}

impl V for (D, D) {
    fn v(&self) -> u32 {
        self.0.0 * 1000 + self.1.0
    }
}

impl V for E {
    fn v(&self) -> u32 {
        match self {
            E::Zero => 0,
            E::One(x) => x.0,
            E::Two { a, b } => a.0 * 1000 + b.0,
        }
    }
}

impl V for F {
    fn v(&self) -> u32 {
        match self {
            F::One(x) => x.0,
            F::Two(a, b) => a.0 * 1000 + b.0,
        }
    }
}

impl V for Box<D> {
    fn v(&self) -> u32 {
        self.0
    }
}

impl V for Option<D> {
    fn v(&self) -> u32 {
        self.as_ref().map_or(0, |x| x.0)
    }
}

impl V for Vec<D> {
    fn v(&self) -> u32 {
        if self.len() == 2 { self[0].0 * 1000 + self[1].0 } else { 0 }
    }
}

impl V for Rc<D> {
    fn v(&self) -> u32 {
        self.0
    }
}

impl V for Box<dyn Fn() -> u32> {
    fn v(&self) -> u32 {
        self()
    }
}

/** An error made of a \`D\`, by a \`From\` that's a point. */
struct Big(D, D);

impl From<D> for Big {
    fn from(e: D) -> Big {
        println!("from {}", e.0);
        point();
        Big(e, d(91))
    }
}

impl V for Big {
    fn v(&self) -> u32 {
        self.0.0 * 1000 + self.1.0
    }
}

/** Two values, in a struct without a \`Drop\` of its own. */
struct Two<T> {
    x: T,
    y: T,
}

/** Two values, in a struct with a \`Drop\` of its own. */
struct Guard<T: V> {
    x: T,
    y: T,
}

impl<T: V> Drop for Guard<T> {
    fn drop(&mut self) {
        println!("drop guard {}", self.x.v());
        point();
    }
}

/** A call given two values, its call a point. */
fn both<T: V>(a: T, b: T) -> u32 {
    point();
    a.v() * 10 + b.v()
}

/** A closure that holds two values. */
fn capture<T: V>(x: T, y: T) -> impl Fn() -> u32 {
    move || {
        point();
        x.v() * 10 + y.v()
    }
}

fn twice<F: FnMut() -> u32>(mut f: F) -> u32 {
    f() + f()
}

fn once<F: FnOnce() -> u32>(f: F) -> u32 {
    f()
}

/** The items' numbers, by an index, not an iterator. */
fn show<T: V>(items: &[T]) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < items.len() {
        out.push_str(&format!(" {}", items[i].v()));
        i += 1;
    }
    out
}

fn show_deque<T: V>(q: &VecDeque<T>) -> String {
    let mut out = String::new();
    let mut i = 0;
    while i < q.len() {
        out.push_str(&format!(" {}", q[i].v()));
        i += 1;
    }
    out
}

fn show_map<T: V>(m: &BTreeMap<u32, T>) -> String {
    let mut out = String::new();
    for (k, x) in m {
        out.push_str(&format!(" {k}:{}", x.v()));
    }
    out
}

/** A map's keys and values, or a set's items, made where it's shown: a
 * generic function's would need its keys' \`Ord\`, which rust-js refuses. */
macro_rules! show_keys {
    ($m:expr) => {{
        let mut out = String::new();
        for (k, x) in $m {
            out.push_str(&format!(" {}:{x}", k.v()));
        }
        out
    }};
}

macro_rules! show_set {
    ($s:expr) => {{
        let mut out = String::new();
        for x in $s {
            out.push_str(&format!(" {}", x.v()));
        }
        out
    }};
}

fn show_option<T: V>(o: &Option<T>) -> String {
    match o {
        Some(x) => format!("Some({})", x.v()),
        None => "None".to_string(),
    }
}

fn show_result<T: V, U: V>(r: &Result<T, U>) -> String {
    match r {
        Ok(x) => format!("Ok({})", x.v()),
        Err(e) => format!("Err({})", e.v()),
    }
}

/** What a case's fallible \`run\` gave. */
trait Said {
    fn said(&self) -> String;
}

impl<U: V> Said for Result<u32, U> {
    fn said(&self) -> String {
        show_result(self)
    }
}

impl Said for Option<u32> {
    fn said(&self) -> String {
        show_option(self)
    }
}
`;

/** A program of `cases`, which runs each at each of its points. */
export function unwindProgram(cases: UnwindCase[]): string {
  const fns = cases.map((c, i) => caseFn(c, i));
  const main = `fn main() {\n${cases.map((c, i) => `    run_points(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude + pointsRunner, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case (`slice()` in
 * test/iterator-matrix.ts): its family's operation, and its family with
 * its shape. */
export const unwindStrata = (c: UnwindCase) => [`${c.family.name} ${c.op.name}`, `${c.family.name} | ${c.shape.name}`];
