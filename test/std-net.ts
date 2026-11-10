// The std net (ITERATOR_TABLE=std, #1171): std's observable behaviour past
// its iterator consumers, which the consumer net covers. Each stable method
// of std's collections, its slices' sorts and edits, `String`'s edits and
// its cells that changes what it's called on in place, or calls what it's
// given: run on a collection of items that print each clone and drop, and
// whose `Ord`, `Eq` and `Hash` count their calls, as it is and with its
// closure's, its comparison's or its hash's 2nd call panicking, under
// `catch_unwind`. What each gives back, what's left of the collection after
// it, a panic's message, and each clone and drop are what native Rust says:
// `retain` that dropped what it kept, a `sort_by` whose comparator's panic
// lost an item, a `split_off` that cloned are each a line that differs.
//
// What it covers is read from std's own source (`stdNetMethods`), so a
// toolchain that adds one fails test/std-net.test.ts until a template
// covers it or `excluded` says why none does.
//
// What std leaves unspecified isn't compared: a hash map's or a hash set's
// order, printed sorted, and the order a sort leaves after its comparison
// panics ("any permutation", slice::sort_by's docs), printed sorted too. Each
// case prints what it can of its collection's items as std promises them.

import { readFileSync } from "node:fs";
import { join } from "node:path";

import { stdLibrary } from "./consumer-net";
import type { Row } from "./iterator-matrix";

// ── What std has ──────────────────────────────────────────────────────────

/** A stable method of std this net covers or excludes, `Type::method`. */
export type NetMethod = { id: string; file: string };

/** The files whose types the net covers, of std's source. */
const files = [
  "alloc/src/vec/mod.rs",
  "alloc/src/collections/vec_deque/mod.rs",
  "alloc/src/string.rs",
  "alloc/src/collections/binary_heap/mod.rs",
  "alloc/src/collections/btree/map.rs",
  "alloc/src/collections/btree/map/entry.rs",
  "alloc/src/collections/btree/set.rs",
  "std/src/collections/hash/map.rs",
  "std/src/collections/hash/set.rs",
  "core/src/slice/mod.rs",
  "alloc/src/slice.rs",
  "core/src/cell.rs",
  "alloc/src/rc.rs",
];

/** Whether the attributes just above line `i` of `lines` say it's stable. */
function stable(lines: string[], i: number): boolean {
  for (let j = i - 1; j >= Math.max(0, i - 16); j--) {
    const t = lines[j].trim();
    if (t.startsWith("#[stable")) return true;
    if (t.startsWith("#[unstable")) return false;
    if (t.startsWith("pub ") || t === "}" || t.startsWith("fn ")) return false;
  }
  return false;
}

/** Each stable, safe, inherent method of `files`' types that takes `&mut
 * self`, or `this: &mut Self`, or a closure: what changes a collection in
 * place, or calls what it's given. Named by the type of the `impl` it's in,
 * a slice's `[T]`. */
export function stdNetMethods(library = stdLibrary()): NetMethod[] {
  const out: NetMethod[] = [];
  for (const file of files) {
    const lines = readFileSync(join(library, file), "utf8").split("\n");
    let ty: string | undefined;
    for (let i = 0; i < lines.length; i++) {
      const line = lines[i];
      const impl = line.match(/^impl(?:<.*?>)?\s+(?:[\w:]+(?:<.*?>)?\s+for\s+)?([A-Z]\w*|\[T\])/);
      if (impl) {
        ty = / for /.test(line) ? undefined : impl[1];
        continue;
      }
      const m = line.match(/^ {4}pub (?:const )?(unsafe )?fn (\w+)/);
      if (!m || ty === undefined || m[1]) continue;
      const signature = lines.slice(i, i + 5).join(" ").split("{")[0];
      const changes = /&mut self|this: &mut Self/.test(signature);
      const calls = /\bFn(?:Mut|Once)?\b/.test(signature);
      if ((changes || calls) && stable(lines, i)) {
        const id = `${ty}::${m[2]}`;
        if (!out.some((o) => o.id === id)) out.push({ id, file });
      }
    }
  }
  return out;
}

// ── Its templates ─────────────────────────────────────────────────────────

/** What a template runs on: its collection's type, how it's made, and how
 * what's left of it is shown, sorted where std leaves its order unsaid. */
type Kind = { ty: string; make: string; show: string };

const kinds = {
  vec: { ty: "Vec<L>", make: "vec![L(1), L(2), L(3), L(4)]", show: '"{c:?}"' },
  dups: { ty: "Vec<L>", make: "vec![L(1), L(1), L(2), L(3), L(3)]", show: '"{c:?}"' },
  nums: { ty: "Vec<u32>", make: "vec![1, 2, 3, 4, 5, 6]", show: '"{c:?}"' },
  // Of what a sort leaves after its comparison panics, in no order std says.
  unsorted: { ty: "Vec<L>", make: "vec![L(3), L(1), L(4), L(2), L(5)]", show: '"{:?}", quiet(|| sorted_of(&c))' },
  deque: { ty: "VecDeque<L>", make: "VecDeque::from(vec![L(1), L(2), L(3), L(4)])", show: '"{c:?}"' },
  string: { ty: "String", make: "String::from(\"héllo wörld\")", show: '"{c:?}"' },
  // A heap's layout is std's own, "arbitrary order": its items sorted.
  heap: { ty: "BinaryHeap<L>", make: "BinaryHeap::from(vec![L(3), L(1), L(4), L(2)])", show: '"{:?}", quiet(|| sorted_of(c.as_slice()))' },
  btree: { ty: "BTreeMap<u32, L>", make: "BTreeMap::from([(1, L(1)), (2, L(2)), (3, L(3)), (4, L(4))])", show: '"{c:?}"' },
  bset: { ty: "BTreeSet<L>", make: "BTreeSet::from([L(1), L(2), L(3), L(4)])", show: '"{c:?}"' },
  hash: { ty: "HashMap<K, u32>", make: "HashMap::from([(K(1), 1), (K(2), 2), (K(3), 3), (K(4), 4)])", show: '"{:?}", quiet(|| sorted(c.iter().map(|(k, v)| (k.0, *v)).collect()))' },
  counted: { ty: "HashMap<C, u32>", make: "HashMap::from([(C(1), 1), (C(2), 2)])", show: '"{:?}", quiet(|| sorted(c.iter().map(|(k, v)| (k.0, *v)).collect()))' },
  hset: { ty: "HashSet<K>", make: "HashSet::from([K(1), K(2), K(3), K(4)])", show: '"{:?}", quiet(|| sorted(c.iter().map(|k| k.0).collect()))' },
  cell: { ty: "Cell<u32>", make: "Cell::new(5)", show: '"{c:?}"' },
  refcell: { ty: "RefCell<Vec<L>>", make: "RefCell::new(vec![L(1), L(2), L(3)])", show: '"{c:?}"' },
} satisfies Record<string, Kind>;

/** A template: what it covers, of `stdNetMethods`'s ids; the collection it
 * runs on; its statements, of `c`, the collection; and whether it calls
 * what may panic, a closure, a comparison or a hash, so it runs with that
 * panicking too, at its `at`th call: its 2nd, but of a hash table's or a
 * B-tree's search, whose calls std counts as it likes, its 1st, before it
 * changes anything. */
export type Template = { name: string; covers: string[]; kind: keyof typeof kinds; text: string; calls?: boolean; at?: number };

const t = (name: string, covers: string[], kind: keyof typeof kinds, text: string, calls = false, at = 2): Template => ({ name, covers, kind, text, calls, at });

/** A closure that counts its call, `tick()`, and gives `body`. */
const f = (params: string, body: string) => `|${params}| { tick(); ${body} }`;

export const templates: Template[] = [
  // Vec: what it takes, gives back, drops and clones as it's changed.
  t("vec push", ["Vec::push"], "vec", "c.push(L(9));"),
  t("vec push_mut", ["Vec::push_mut"], "vec", "c.push_mut(L(9)).0 += 1;"),
  t("vec insert", ["Vec::insert"], "vec", "c.insert(1, L(9));"),
  t("vec insert past its end", ["Vec::insert"], "vec", "c.insert(9, L(9));"),
  t("vec insert_mut", ["Vec::insert_mut"], "vec", "c.insert_mut(1, L(9)).0 += 1;"),
  t("vec remove", ["Vec::remove"], "vec", 'println!("{:?}", c.remove(1));'),
  t("vec remove past its end", ["Vec::remove"], "vec", 'println!("{:?}", c.remove(9));'),
  t("vec swap_remove", ["Vec::swap_remove"], "vec", 'println!("{:?}", c.swap_remove(1));'),
  t("vec pop", ["Vec::pop"], "vec", 'println!("{:?}", c.pop());'),
  t("vec pop_if", ["Vec::pop_if"], "vec", `println!("{:?}", c.pop_if(${f("x", "x.0 > 2")}));`, true),
  t("vec truncate", ["Vec::truncate"], "vec", "c.truncate(2);"),
  t("vec clear", ["Vec::clear"], "vec", "c.clear();"),
  t("vec retain", ["Vec::retain"], "vec", `c.retain(${f("x", "x.0 % 2 == 0")});`, true),
  t("vec retain_mut", ["Vec::retain_mut"], "vec", `c.retain_mut(${f("x", "x.0 += 10; x.0 % 2 == 0")});`, true),
  t("vec dedup", ["Vec::dedup"], "dups", "c.dedup();", true),
  t("vec dedup_by", ["Vec::dedup_by"], "dups", `c.dedup_by(${f("a, b", "a.0 == b.0")});`, true),
  t("vec dedup_by_key", ["Vec::dedup_by_key"], "dups", `c.dedup_by_key(${f("x", "x.0 / 2")});`, true),
  t("vec append", ["Vec::append"], "vec", 'let mut o = vec![L(7), L(8)];\nc.append(&mut o);\nprintln!("{o:?}");'),
  t("vec drain", ["Vec::drain"], "vec", 'let d: Vec<L> = c.drain(1..3).collect();\nprintln!("{d:?}");'),
  t("vec drain left part-way", ["Vec::drain"], "vec", 'let mut d = c.drain(1..3);\nprintln!("{:?}", d.next());'),
  t("vec split_off", ["Vec::split_off"], "vec", 'let o = c.split_off(1);\nprintln!("{o:?}");'),
  t("vec resize_with", ["Vec::resize_with"], "vec", `c.resize_with(6, ${f("", "L(9)")});`, true),
  t("vec resize", ["Vec::resize"], "vec", "c.resize(6, L(9));\nc.resize(2, L(8));"),
  t("vec extend_from_slice", ["Vec::extend_from_slice"], "vec", "c.extend_from_slice(&[L(7), L(8)]);"),
  t("vec extend_from_within", ["Vec::extend_from_within"], "vec", "c.extend_from_within(1..3);"),
  t("vec splice", ["Vec::splice"], "vec", 'let r: Vec<L> = c.splice(1..3, [L(7), L(8), L(9)]).collect();\nprintln!("{r:?}");'),
  t("vec extract_if", ["Vec::extract_if"], "vec", `let r: Vec<L> = c.extract_if(.., ${f("x", "x.0 % 2 == 0")}).collect();\nprintln!("{r:?}");`, true),
  t("vec extract_if left part-way", ["Vec::extract_if"], "vec", `let mut r = c.extract_if(.., ${f("x", "x.0 % 2 == 0")});\nprintln!("{:?}", r.next());`, true),
  // VecDeque.
  t("deque push_front and back", ["VecDeque::push_front", "VecDeque::push_back"], "deque", "c.push_front(L(8));\nc.push_back(L(9));"),
  t("deque push_mut", ["VecDeque::push_front_mut", "VecDeque::push_back_mut"], "deque", "c.push_front_mut(L(8)).0 += 1;\nc.push_back_mut(L(9)).0 += 1;"),
  t("deque pop", ["VecDeque::pop_front", "VecDeque::pop_back"], "deque", 'println!("{:?} {:?}", c.pop_front(), c.pop_back());'),
  t("deque pop_if", ["VecDeque::pop_front_if", "VecDeque::pop_back_if"], "deque", `println!("{:?}", c.pop_front_if(${f("x", "x.0 < 2")}));\nprintln!("{:?}", c.pop_back_if(${f("x", "x.0 < 2")}));`, true),
  t("deque insert and remove", ["VecDeque::insert", "VecDeque::remove", "VecDeque::insert_mut"], "deque", 'c.insert(1, L(9));\nc.insert_mut(0, L(8)).0 += 1;\nprintln!("{:?} {:?}", c.remove(2), c.remove(9));'),
  t("deque swap_remove", ["VecDeque::swap_remove_front", "VecDeque::swap_remove_back"], "deque", 'println!("{:?} {:?}", c.swap_remove_front(2), c.swap_remove_back(0));'),
  t("deque truncate and clear", ["VecDeque::truncate", "VecDeque::clear"], "deque", 'c.truncate(2);\nprintln!("{c:?}");\nc.clear();'),
  t("deque retain", ["VecDeque::retain", "VecDeque::retain_mut"], "deque", `c.retain(${f("x", "x.0 != 2")});\nc.retain_mut(${f("x", "x.0 += 10; x.0 != 13")});`, true),
  t("deque retain_back", ["VecDeque::retain_back"], "deque", "c.retain_back(2);\nc.retain_back(9);"),
  t("deque drain", ["VecDeque::drain"], "deque", 'let mut d = c.drain(1..3);\nprintln!("{:?}", d.next());'),
  t("deque split_off and append", ["VecDeque::split_off", "VecDeque::append"], "deque", 'let mut o = c.split_off(2);\no.push_front(L(9));\nc.append(&mut o);\nprintln!("{o:?}");'),
  t("deque resize", ["VecDeque::resize", "VecDeque::resize_with"], "deque", `c.resize(5, L(9));\nc.resize_with(7, ${f("", "L(8)")});\nc.resize(2, L(7));`, true),
  t("deque rotate", ["VecDeque::rotate_left", "VecDeque::rotate_right", "VecDeque::make_contiguous"], "deque", 'c.rotate_left(1);\nc.push_front(L(0));\nc.rotate_right(2);\nprintln!("{:?}", c.make_contiguous());'),
  t("deque swap", ["VecDeque::swap"], "deque", "c.swap(0, 3);"),
  t("deque access", ["VecDeque::get_mut", "VecDeque::front_mut", "VecDeque::back_mut", "VecDeque::iter_mut", "VecDeque::range_mut", "VecDeque::as_mut_slices"], "deque", "c.get_mut(1).unwrap().0 += 10;\nc.front_mut().unwrap().0 += 20;\nc.back_mut().unwrap().0 += 30;\nfor x in c.iter_mut() {\n    x.0 += 1;\n}\nfor x in c.range_mut(1..3) {\n    x.0 += 1;\n}\nc.as_mut_slices().0[0].0 += 1;"),
  t("deque binary_search", ["VecDeque::binary_search_by", "VecDeque::binary_search_by_key", "VecDeque::partition_point"], "deque", `println!("{:?}", c.binary_search_by(${f("x", "x.0.cmp(&3)")}));\nprintln!("{:?}", c.binary_search_by_key(&2, ${f("x", "x.0")}));\nprintln!("{:?}", c.partition_point(${f("x", "x.0 < 3")}));`),
  // String, of text with characters past ASCII.
  t("string push", ["String::push", "String::push_str"], "string", "c.push('é');\nc.push_str(\"ü!\");"),
  t("string pop and remove", ["String::pop", "String::remove"], "string", 'println!("{:?} {:?}", c.pop(), c.remove(1));'),
  t("string remove inside a character", ["String::remove"], "string", 'println!("{:?}", c.remove(2));'),
  t("string insert", ["String::insert", "String::insert_str"], "string", "c.insert(1, 'ß');\nc.insert_str(0, \"¡\");"),
  t("string insert inside a character", ["String::insert"], "string", "c.insert(2, 'x');"),
  t("string retain", ["String::retain"], "string", `c.retain(${f("ch", "ch != 'l'")});`, true),
  t("string truncate and clear", ["String::truncate", "String::clear"], "string", 'c.truncate(6);\nprintln!("{c:?}");\nc.clear();'),
  t("string truncate inside a character", ["String::truncate"], "string", "c.truncate(2);"),
  t("string split_off", ["String::split_off"], "string", 'println!("{:?}", c.split_off(6));'),
  t("string drain", ["String::drain"], "string", 'println!("{:?}", c.drain(1..4).collect::<String>());'),
  t("string replace_range", ["String::replace_range"], "string", "c.replace_range(1..3, \"ÉÉÉ\");"),
  t("string extend_from_within", ["String::extend_from_within"], "string", "c.extend_from_within(1..3);"),
  t("string as_mut_str", ["String::as_mut_str"], "string", "c.as_mut_str().make_ascii_uppercase();"),
  // BinaryHeap, its layout as std leaves it.
  t("heap push and pop", ["BinaryHeap::push", "BinaryHeap::pop"], "heap", 'c.push(L(9));\nprintln!("{:?}", c.pop());', true),
  t("heap peek_mut", ["BinaryHeap::peek_mut"], "heap", "c.peek_mut().unwrap().0 = 0;", true),
  t("heap append", ["BinaryHeap::append"], "heap", 'let mut o = BinaryHeap::from(vec![L(7), L(0)]);\nc.append(&mut o);\nprintln!("{}", o.len());', true),
  t("heap retain", ["BinaryHeap::retain"], "heap", `c.retain(${f("x", "x.0 != 3")});`, true),
  t("heap drain and clear", ["BinaryHeap::drain", "BinaryHeap::clear"], "heap", 'let mut d = c.drain();\nprintln!("{:?}", d.next().map(|l| l.0));\ndrop(d);\nc.push(L(5));\nc.clear();'),
  // BTreeMap and BTreeSet.
  t("btree insert and remove", ["BTreeMap::insert", "BTreeMap::remove", "BTreeMap::remove_entry"], "btree", 'println!("{:?}", c.insert(2, L(20)));\nprintln!("{:?}", c.insert(9, L(9)));\nprintln!("{:?} {:?}", c.remove(&1), c.remove_entry(&3));'),
  t("btree get_mut", ["BTreeMap::get_mut", "BTreeMap::iter_mut", "BTreeMap::values_mut", "BTreeMap::range_mut"], "btree", "c.get_mut(&2).unwrap().0 += 10;\nfor (_, v) in c.iter_mut() {\n    v.0 += 1;\n}\nfor v in c.values_mut() {\n    v.0 += 1;\n}\nfor (_, v) in c.range_mut(2..) {\n    v.0 += 1;\n}"),
  t("btree first and last", ["BTreeMap::first_entry", "BTreeMap::last_entry", "BTreeMap::pop_first", "BTreeMap::pop_last"], "btree", 'c.first_entry().unwrap().get_mut().0 += 10;\nprintln!("{:?}", c.last_entry().unwrap().remove());\nprintln!("{:?} {:?}", c.pop_first(), c.pop_last());'),
  t("btree retain", ["BTreeMap::retain"], "btree", `c.retain(${f("k, v", "v.0 += 10; k % 2 == 0")});`, true),
  t("btree append and split_off", ["BTreeMap::append", "BTreeMap::split_off"], "btree", 'let mut o = BTreeMap::from([(2, L(22)), (7, L(7))]);\nc.append(&mut o);\nlet r = c.split_off(&3);\nprintln!("{r:?} {o:?}");'),
  t("btree extract_if", ["BTreeMap::extract_if"], "btree", `let mut r = c.extract_if(.., ${f("k, _v", "k % 2 == 0")});\nprintln!("{:?}", r.next());`, true),
  t("btree entry", ["BTreeMap::entry", "Entry::and_modify", "Entry::or_insert_with", "Entry::or_insert_with_key", "OccupiedEntry::get_mut", "OccupiedEntry::insert"], "btree", `c.entry(2).and_modify(${f("v", "v.0 += 10;")}).or_insert_with(${f("", "L(0)")});\nc.entry(8).and_modify(${f("v", "v.0 += 10;")}).or_insert_with(${f("", "L(8)")});\nc.entry(6).or_insert_with_key(${f("k", "L(*k)")});\nif let std::collections::btree_map::Entry::Occupied(mut o) = c.entry(3) {\n    o.get_mut().0 += 1;\n    println!("{:?}", o.insert(L(30)));\n}`, true),
  t("btree clear", ["BTreeMap::clear"], "btree", "c.clear();"),
  t("bset insert and remove", ["BTreeSet::insert", "BTreeSet::replace", "BTreeSet::remove", "BTreeSet::take"], "bset", 'println!("{} {}", c.insert(L(9)), c.insert(L(2)));\nprintln!("{:?}", c.replace(L(3)));\nprintln!("{} {:?}", c.remove(&L(1)), c.take(&L(4)));', true, 1),
  t("bset first and last", ["BTreeSet::pop_first", "BTreeSet::pop_last"], "bset", 'println!("{:?} {:?}", c.pop_first(), c.pop_last());'),
  t("bset retain", ["BTreeSet::retain"], "bset", `c.retain(${f("x", "x.0 != 2")});`, true),
  t("bset append and split_off", ["BTreeSet::append", "BTreeSet::split_off"], "bset", 'let mut o = BTreeSet::from([L(2), L(7)]);\nc.append(&mut o);\nlet r = c.split_off(&L(3));\nprintln!("{r:?} {o:?}");', true, 1),
  t("bset extract_if", ["BTreeSet::extract_if"], "bset", `let mut r = c.extract_if(.., ${f("x", "x.0 % 2 == 0")});\nprintln!("{:?}", r.next());`, true),
  t("bset clear", ["BTreeSet::clear"], "bset", "c.clear();"),
  // HashMap and HashSet, printed sorted.
  t("hash insert and remove", ["HashMap::insert", "HashMap::remove", "HashMap::remove_entry", "HashMap::get_mut"], "hash", 'println!("{:?} {:?}", c.insert(K(2), 20), c.insert(K(9), 9));\nprintln!("{:?} {:?}", c.remove(&K(1)), c.remove_entry(&K(3)).map(|(k, v)| (k.0, v)));\n*c.get_mut(&K(4)).unwrap() += 1;'),
  t("hash entry", ["HashMap::entry", "Entry::and_modify", "OccupiedEntry::get_mut", "OccupiedEntry::insert"], "hash", `c.entry(K(2)).and_modify(${f("v", "*v += 10;")}).or_insert(0);\nc.entry(K(8)).and_modify(${f("v", "*v += 10;")}).or_insert(8);\nif let std::collections::hash_map::Entry::Occupied(mut o) = c.entry(K(3)) {\n    *o.get_mut() += 1;\n    println!("{:?}", o.insert(30));\n}`, true),
  t("hash retain", ["HashMap::retain", "HashMap::iter_mut", "HashMap::values_mut"], "hash", `for (_, v) in c.iter_mut() {\n    *v += 1;\n}\nfor v in c.values_mut() {\n    *v += 1;\n}\nc.retain(|k, _| k.0 % 2 == 0);`),
  t("hash drain and extract_if", ["HashMap::drain", "HashMap::extract_if", "HashMap::clear"], "hash", 'let mut r: Vec<u32> = c.extract_if(|k, _| k.0 % 2 == 0).map(|(k, _)| k.0).collect();\nr.sort();\nprintln!("{r:?}");\nlet mut d: Vec<u32> = c.drain().map(|(k, _)| k.0).collect();\nd.sort();\nprintln!("{d:?}");\nc.insert(K(5), 5);\nc.clear();'),
  t("hash get_disjoint_mut", ["HashMap::get_disjoint_mut"], "hash", "if let [Some(a), Some(b)] = c.get_disjoint_mut([&K(1), &K(2)]) {\n    *a += 10;\n    *b += 20;\n}"),
  // Of a key whose own `Hash` and `Eq` count their calls, panicking at its
  // 1st, before the table changes: refused till rust-js compiles a key's
  // hand-written `Hash`, so the net shows when it does.
  t("hash of a counted key", ["HashMap::insert", "HashMap::remove"], "counted", 'println!("{:?}", c.insert(C(9), 9));\nprintln!("{:?}", c.remove(&C(1)));', true, 1),
  t("hset insert and remove", ["HashSet::insert", "HashSet::replace", "HashSet::remove", "HashSet::take"], "hset", 'println!("{} {}", c.insert(K(9)), c.insert(K(2)));\nprintln!("{:?}", c.replace(K(3)).map(|k| k.0));\nprintln!("{} {:?}", c.remove(&K(1)), c.take(&K(4)).map(|k| k.0));'),
  t("hset retain, drain and extract_if", ["HashSet::retain", "HashSet::drain", "HashSet::extract_if", "HashSet::clear"], "hset", 'c.retain(|k| k.0 != 1);\nlet mut r: Vec<u32> = c.extract_if(|k| k.0 % 2 == 0).map(|k| k.0).collect();\nr.sort();\nprintln!("{r:?}");\nlet mut d: Vec<u32> = c.drain().map(|k| k.0).collect();\nd.sort();\nprintln!("{d:?}");\nc.insert(K(5));\nc.clear();'),
  // A slice's sorts, its comparison panicking at its 2nd call too, what's
  // left printed sorted, as std promises no order then.
  t("sort", ["[T]::sort"], "unsorted", "c.sort();", true),
  t("sort_by", ["[T]::sort_by"], "unsorted", `c.sort_by(${f("a, b", "b.0.cmp(&a.0)")});`, true),
  t("sort_by_key", ["[T]::sort_by_key"], "unsorted", `c.sort_by_key(${f("x", "x.0 % 3")});`, true),
  t("sort_by_cached_key", ["[T]::sort_by_cached_key"], "unsorted", `c.sort_by_cached_key(${f("x", "x.0 % 3")});`, true),
  t("sort_unstable", ["[T]::sort_unstable"], "unsorted", "c.sort_unstable();", true),
  t("sort_unstable_by", ["[T]::sort_unstable_by"], "unsorted", `c.sort_unstable_by(${f("a, b", "b.0.cmp(&a.0)")});`, true),
  t("sort_unstable_by_key", ["[T]::sort_unstable_by_key"], "unsorted", `c.sort_unstable_by_key(${f("x", "x.0 % 3")});`, true),
  t("select_nth_unstable", ["[T]::select_nth_unstable"], "unsorted", 'let (_, nth, _) = c.select_nth_unstable(2);\nprintln!("{nth:?}");', true),
  t("select_nth_unstable_by", ["[T]::select_nth_unstable_by", "[T]::select_nth_unstable_by_key"], "unsorted", `println!("{:?}", c.select_nth_unstable_by(2, ${f("a, b", "b.0.cmp(&a.0)")}).1);\nprintln!("{:?}", c.select_nth_unstable_by_key(1, ${f("x", "x.0 % 3")}).1);`, true),
  t("is_sorted_by", ["[T]::is_sorted_by", "[T]::is_sorted_by_key"], "vec", `println!("{}", c.is_sorted_by(${f("a, b", "a.0 <= b.0")}));\nprintln!("{}", c.is_sorted_by_key(${f("x", "x.0 % 3")}));`, true),
  t("binary_search_by", ["[T]::binary_search_by", "[T]::binary_search_by_key", "[T]::partition_point"], "vec", `println!("{:?}", c.binary_search_by(${f("x", "x.0.cmp(&3)")}));\nprintln!("{:?}", c.binary_search_by_key(&2, ${f("x", "x.0")}));\nprintln!("{:?}", c.partition_point(${f("x", "x.0 < 3")}));`),
  // A slice's edits in place.
  t("reverse and swap", ["[T]::reverse", "[T]::swap"], "vec", "c.reverse();\nc.swap(0, 2);"),
  t("swap past its end", ["[T]::swap"], "vec", "c.swap(0, 9);"),
  t("rotate", ["[T]::rotate_left", "[T]::rotate_right"], "vec", "c.rotate_left(1);\nc.rotate_right(3);"),
  t("fill", ["[T]::fill", "[T]::fill_with"], "vec", `c[..2].fill(L(9));\nc[2..].fill_with(${f("", "L(8)")});`, true),
  t("clone_from_slice", ["[T]::clone_from_slice", "[T]::swap_with_slice"], "vec", 'let mut o = vec![L(7), L(8)];\nc[1..3].clone_from_slice(&o);\nc[..2].swap_with_slice(&mut o);\nprintln!("{o:?}");'),
  t("copy_within", ["[T]::copy_within", "[T]::copy_from_slice"], "nums", "c.copy_within(0..2, 3);\nc[..2].copy_from_slice(&[9, 8]);"),
  t("split_at_mut", ["[T]::split_at_mut", "[T]::split_at_mut_checked"], "vec", 'let (a, b) = c.split_at_mut(1);\na[0].0 += 10;\nb[0].0 += 20;\nprintln!("{}", c.split_at_mut_checked(9).is_none());'),
  t("first and last mut", ["[T]::first_mut", "[T]::last_mut", "[T]::split_first_mut", "[T]::split_last_mut", "[T]::get_mut", "[T]::first_chunk_mut", "[T]::last_chunk_mut", "[T]::iter_mut", "[T]::split_first_chunk_mut", "[T]::split_last_chunk_mut", "[T]::as_chunks_mut", "[T]::as_rchunks_mut", "[T]::get_disjoint_mut"], "vec", "c.split_first_chunk_mut::<1>().unwrap().0[0].0 += 1;\nc.split_last_chunk_mut::<1>().unwrap().1[0].0 += 1;\nc.as_chunks_mut::<2>().0[0][1].0 += 1;\nc.as_rchunks_mut::<2>().1[0][0].0 += 1;\nif let Ok([a, b]) = c.get_disjoint_mut([0, 2]) {\n    std::mem::swap(a, b);\n}\nc.first_mut().unwrap().0 += 10;\nc.last_mut().unwrap().0 += 10;\nc.split_first_mut().unwrap().1[0].0 += 1;\nc.split_last_mut().unwrap().1[0].0 += 1;\nc.get_mut(1..3).unwrap()[0].0 += 1;\nc.first_chunk_mut::<2>().unwrap()[1].0 += 1;\nc.last_chunk_mut::<2>().unwrap()[0].0 += 1;\nfor x in c.iter_mut() {\n    x.0 += 100;\n}"),
  t("chunks_mut", ["[T]::chunks_mut", "[T]::chunks_exact_mut", "[T]::rchunks_mut", "[T]::rchunks_exact_mut"], "vec", "for (i, ch) in c.chunks_mut(3).enumerate() {\n    ch[0].0 += 10 * i as u32;\n}\nfor ch in c.chunks_exact_mut(3) {\n    ch.reverse();\n}\nfor ch in c.rchunks_mut(3) {\n    ch[0].0 += 1;\n}\nfor ch in c.rchunks_exact_mut(3) {\n    ch.reverse();\n}"),
  t("split by", ["[T]::split", "[T]::split_mut", "[T]::rsplit", "[T]::rsplit_mut", "[T]::splitn", "[T]::splitn_mut", "[T]::rsplitn", "[T]::rsplitn_mut", "[T]::split_inclusive", "[T]::split_inclusive_mut"], "vec", `println!("{:?}", c.split(${f("x", "x.0 == 2")}).collect::<Vec<_>>());\nprintln!("{:?}", c.rsplit(${f("x", "x.0 == 3")}).collect::<Vec<_>>());\nprintln!("{:?}", c.splitn(2, ${f("x", "x.0 % 2 == 0")}).collect::<Vec<_>>());\nprintln!("{:?}", c.rsplitn(2, ${f("x", "x.0 % 2 == 0")}).collect::<Vec<_>>());\nprintln!("{:?}", c.split_inclusive(${f("x", "x.0 == 2")}).collect::<Vec<_>>());\nfor s in c.split_mut(${f("x", "x.0 == 2")}) {\n    s.reverse();\n}\nfor s in c.rsplit_mut(${f("x", "x.0 == 4")}) {\n    s.reverse();\n}\nfor s in c.splitn_mut(2, ${f("x", "x.0 == 1")}) {\n    s.reverse();\n}\nfor s in c.rsplitn_mut(2, ${f("x", "x.0 == 3")}) {\n    s.reverse();\n}\nfor s in c.split_inclusive_mut(${f("x", "x.0 == 2")}) {\n    s.reverse();\n}`, true),
  t("chunk_by", ["[T]::chunk_by", "[T]::chunk_by_mut"], "dups", `println!("{:?}", c.chunk_by(${f("a, b", "a.0 == b.0")}).collect::<Vec<_>>());\nfor ch in c.chunk_by_mut(${f("a, b", "a.0 == b.0")}) {\n    ch[0].0 += 10;\n}`, true),
  // Cells.
  t("cell update", ["Cell::update", "Cell::get_mut"], "cell", `c.update(${f("x", "x + 1")});\n*c.get_mut() += 10;`, true),
  t("refcell get_mut", ["RefCell::get_mut"], "refcell", "c.get_mut().push(L(9));"),
  t("refcell replace_with", ["RefCell::replace_with"], "refcell", `let old = c.replace_with(${f("v", "vec![L(v.len() as u32 + 10)]")});\nprintln!("{old:?}");`, true),
  t("ref map", ["Ref::map", "Ref::filter_map", "Ref::map_split"], "refcell", `let r = std::cell::Ref::map(c.borrow(), ${f("v", "&v[1]")});\nprintln!("{r:?}");\ndrop(r);\nprintln!("{:?}", std::cell::Ref::filter_map(c.borrow(), ${f("v", "v.get(9)")}).is_err());\nlet (a, b) = std::cell::Ref::map_split(c.borrow(), ${f("v", "v.split_at(1)")});\nprintln!("{a:?} {b:?}");`, true),
  t("refmut map", ["RefMut::map", "RefMut::filter_map"], "refcell", `std::cell::RefMut::map(c.borrow_mut(), ${f("v", "&mut v[1]")}).0 += 10;\nprintln!("{:?}", std::cell::RefMut::filter_map(c.borrow_mut(), ${f("v", "v.get_mut(9)")}).is_err());`, true),
  t("rc get_mut and make_mut", ["Rc::get_mut", "Rc::make_mut"], "vec", 'let mut r = std::rc::Rc::new(L(1));\nstd::rc::Rc::get_mut(&mut r).unwrap().0 += 1;\nlet shared = r.clone();\nprintln!("{}", std::rc::Rc::get_mut(&mut r).is_none());\nstd::rc::Rc::make_mut(&mut r).0 += 10;\nprintln!("{r:?} {shared:?}");'),
  t("rc new_cyclic", ["Rc::new_cyclic"], "vec", `let r = std::rc::Rc::new_cyclic(${f("w: &std::rc::Weak<L>", "println!(\"{}\", w.upgrade().is_none()); L(9)")});\nprintln!("{r:?} {}", std::rc::Rc::strong_count(&r));`, true),
];

/** Why no template covers each method left, of `stdNetMethods`'s. */
const untouched = "reserves, shrinks or reads its capacity, or gives a pointer or a slice of what it holds: it moves, clones, drops and calls nothing";
export const excluded: Record<string, string> = Object.fromEntries([
  ...[
    "Vec::reserve", "Vec::reserve_exact", "Vec::try_reserve", "Vec::try_reserve_exact", "Vec::shrink_to_fit", "Vec::shrink_to",
    "Vec::as_mut_slice", "Vec::as_mut_ptr", "Vec::spare_capacity_mut",
    "VecDeque::reserve", "VecDeque::reserve_exact", "VecDeque::try_reserve", "VecDeque::try_reserve_exact", "VecDeque::shrink_to_fit", "VecDeque::shrink_to",
    "String::reserve", "String::reserve_exact", "String::try_reserve", "String::try_reserve_exact", "String::shrink_to_fit", "String::shrink_to",
    "BinaryHeap::reserve", "BinaryHeap::reserve_exact", "BinaryHeap::try_reserve", "BinaryHeap::try_reserve_exact", "BinaryHeap::shrink_to_fit", "BinaryHeap::shrink_to",
    "HashMap::reserve", "HashMap::try_reserve", "HashMap::shrink_to_fit", "HashMap::shrink_to",
    "HashSet::reserve", "HashSet::try_reserve", "HashSet::shrink_to_fit", "HashSet::shrink_to",
    "[T]::as_mut_ptr", "[T]::as_mut_ptr_range", "[T]::as_mut_array", "[T]::as_flattened_mut", "UnsafeCell::get_mut",
  ].map((id) => [id, untouched]),
]);

/** Why `m` is left out, or nothing where a template covers it. */
export function whyNot(m: NetMethod): string | undefined {
  if (templates.some((t) => t.covers.includes(m.id))) return undefined;
  return excluded[m.id];
}

// ── Its cases and programs ────────────────────────────────────────────────

/** How a case runs: as it is, or with what it calls, a closure, a
 * comparison or a hash, panicking at its 2nd call. */
export type Variant = "ok" | "panics";

export type NetCase = Row & { template: Template; variant: Variant };

/** Each template, as it is and, where it calls what may panic, with that
 * panicking, interleaved by a hash of each id, so a program holds many. */
export function stdNetCases(): NetCase[] {
  const out: Omit<NetCase, "n">[] = [];
  for (const template of templates) {
    for (const variant of template.calls ? (["ok", "panics"] as Variant[]) : (["ok"] as Variant[])) {
      out.push({ id: `${template.name} | ${variant}`, template, variant });
    }
  }
  const order = (id: string) => Number(Bun.hash(id) % 1_000_000_007n);
  return out
    .map((c) => ({ c, k: order(c.id) }))
    .sort((x, y) => x.k - y.k || (x.c.id < y.c.id ? -1 : 1))
    .map(({ c }, n) => ({ ...c, n }));
}

const prelude = `// Generated by test/std-net.ts. Each case prints its name, what the
// method gives back, what's left of what it changed, each clone and drop
// of its items, and whether it panicked.
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, BTreeSet, BinaryHeap, HashMap, HashSet, VecDeque};
use std::hash::{Hash, Hasher};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};

/** The call of what a case calls that panics, 0 for none, the calls so
 * far, and whether calls are counted, not while a case prints. */
static ARMED: AtomicU32 = AtomicU32::new(0);
static CALLS: AtomicU32 = AtomicU32::new(0);
static QUIET: AtomicBool = AtomicBool::new(false);

/** A call of a case's closure, comparison or hash, which panics where it's
 * the armed one. */
fn tick() {
    if QUIET.load(Ordering::Relaxed) {
        return;
    }
    let n = CALLS.fetch_add(1, Ordering::Relaxed) + 1;
    if n == ARMED.load(Ordering::Relaxed) {
        panic!("call");
    }
}

/** What \`f\` gives, its calls of \`tick\` uncounted. */
fn quiet<T>(f: impl FnOnce() -> T) -> T {
    QUIET.store(true, Ordering::Relaxed);
    let made = f();
    QUIET.store(false, Ordering::Relaxed);
    made
}

/** An item that prints as it's cloned and dropped, its clone 100 more, so
 * its drop is told apart; its comparisons and its hash count their calls. */
#[derive(Debug)]
struct L(u32);

impl Clone for L {
    fn clone(&self) -> L {
        println!("clone {}", self.0);
        L(self.0 + 100)
    }
}

impl Drop for L {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

impl PartialEq for L {
    fn eq(&self, other: &L) -> bool {
        tick();
        self.0 == other.0
    }
}

impl Eq for L {}

impl PartialOrd for L {
    fn partial_cmp(&self, other: &L) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for L {
    fn cmp(&self, other: &L) -> std::cmp::Ordering {
        tick();
        self.0.cmp(&other.0)
    }
}

/** A hash table's key, which prints nothing: a hash table's drops are in
 * no order std says. Its \`Hash\` and \`Eq\` derived, as a key whose own
 * count their calls, \`C\`, isn't one rust-js compiles yet. */
#[derive(Debug, PartialEq, Eq, Hash)]
struct K(u32);

/** A key whose \`Hash\` and \`Eq\` count their calls, so they panic too. */
#[derive(Debug)]
struct C(u32);

impl PartialEq for C {
    fn eq(&self, other: &C) -> bool {
        tick();
        self.0 == other.0
    }
}

impl Eq for C {}

impl Hash for C {
    fn hash<H: Hasher>(&self, state: &mut H) {
        tick();
        self.0.hash(state);
    }
}

fn sorted<T: Ord>(mut items: Vec<T>) -> Vec<T> {
    items.sort();
    items
}

/** What \`items\` hold, by their numbers, sorted. */
fn sorted_of(items: &[L]) -> Vec<u32> {
    sorted(items.iter().map(|l| l.0).collect())
}

/** A panic's message. */
fn said(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| s.to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| String::from("?"))
}

fn run(name: &str, case: fn()) {
    println!("case {name}");
    ARMED.store(0, Ordering::Relaxed);
    CALLS.store(0, Ordering::Relaxed);
    match catch_unwind(case) {
        Ok(()) => println!("ended"),
        Err(payload) => println!("panicked: {}", said(&*payload)),
    }
}`;

const indent = (text: string) =>
  text
    .split("\n")
    .map((line) => (line === "" ? "" : `    ${line}`))
    .join("\n");

/** A case's function: its collection made, the template's statements run
 * under \`catch_unwind\`, armed where it panics, then what's left shown
 * and dropped. */
function caseFn(c: NetCase, i: number): string {
  const kind = kinds[c.template.kind];
  const lines = [
    `let mut c: ${kind.ty} = ${kind.make};`,
    ...(c.variant === "panics" ? [`ARMED.store(CALLS.load(Ordering::Relaxed) + ${c.template.at ?? 2}, Ordering::Relaxed);`] : []),
    `let r = catch_unwind(AssertUnwindSafe(|| {\n${indent(c.template.text)}\n}));`,
    "ARMED.store(0, Ordering::Relaxed);",
    'match r {\n    Ok(()) => println!("ok"),\n    Err(payload) => println!("panicked: {}", said(&*payload)),\n}',
    `println!(${kind.show});`,
  ];
  return `#[allow(unused_mut)]\nfn case_${i}() {\n${indent(lines.join("\n"))}\n}`;
}

/** A program of `cases`, which runs each in turn. */
export function stdNetProgram(cases: NetCase[]): string {
  const fns = cases.map((c, i) => caseFn(c, i));
  const main = `fn main() {\n${cases.map((c, i) => `    run(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case: its template in
 * its variant, so the slice is all of it. */
export const stdNetStrata = (c: NetCase) => [`${c.template.name} | ${c.variant}`];
