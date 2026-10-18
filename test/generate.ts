// Generated programs (ADR 0092): Rust that no one wrote, each from a seed,
// valid and deterministic by construction, to run as the corpus's are. And a
// reducer, which makes one that fails as small as it still fails.
//
//   seed ──► program ──► native and JS differ? ──► reduce ──► a few lines, and the seed

// A random number generator whose numbers a seed says: mulberry32.
export function random(seed: number) {
  let state = seed >>> 0;
  const next = () => {
    state = (state + 0x6d2b79f5) >>> 0;
    let t = state;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
  const int = (n: number) => Math.floor(next() * n);
  const pick = <T>(items: readonly T[]): T => items[int(items.length)];
  return { next, int, pick, chance: (p: number) => next() < p };
}
type Random = ReturnType<typeof random>;

// `usize` is 32 bits in rust-js and 64 natively (ADR 0090): not generated.
export const intTypes = ["i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64"] as const;
export type IntTy = (typeof intTypes)[number];
// An integer, a `bool`, `Vec<i32>`, `Option<u8>`, the struct `S`, or a
// closure from an integer to one, `fn(i32)`.
export type Ty = string;

const isInt = (ty: Ty): ty is IntTy => (intTypes as readonly string[]).includes(ty);
export const floatTypes = ["f64", "f32"] as const;
const isFloat = (ty: Ty) => ty === "f64" || ty === "f32";
// Where a float's JS and Rust most often part: a sign of zero, a half that
// rounds, where JS's text turns to an exponent, `2^53 + 1`, the smallest
// and largest, and what isn't a number.
const floatEdges: Record<"f64" | "f32", string[]> = {
  f64: ["0.0", "-0.0", "1.0", "-1.0", "0.1", "0.5", "1.5", "2.5", "-2.5", "1e21", "1e-7", "123456789.125", "9007199254740993.0", "1e300", "5e-324", "f64::NAN", "f64::INFINITY", "f64::NEG_INFINITY", "f64::MAX", "f64::MIN_POSITIVE", "f64::EPSILON", "3.0", "-7.25", "1e15", "1e16"],
  f32: ["0.0", "-0.0", "1.0", "0.1", "0.5", "2.5", "16777217.0", "1e-7", "3.4028235e38", "1e-45", "f32::NAN", "f32::INFINITY", "f32::MAX", "f32::EPSILON", "-7.25", "1e10"],
};
const inside = (ty: Ty, outer: "Vec" | "Option"): IntTy | undefined => {
  const m = new RegExp(`^${outer}<(\\w+)>$`).exec(ty);
  return m && isInt(m[1]) ? m[1] : undefined;
};
// What's `Copy` can be read as it is; a `Vec`, a `String` or a map is read
// through `clone()`, so it's never moved from the variable that holds it.
// `(i32, u8)`, a tuple of two widths.
const pairParts: [IntTy, IntTy] = ["i32", "u8"];
const isCopy = (ty: Ty) => inside(ty, "Vec") === undefined && !["String", "Map", "D", "Shapes", "Shared"].includes(ty);

// The struct every program has, whose fields are of three widths.
export const struct = { name: "S", fields: [["a", "i32"], ["b", "u8"], ["c", "i64"]] as [string, IntTy][] };

// Text where UTF-8 and UTF-16 part: accents, `ß`, which upper-cases to two
// letters, CJK, and an emoji, which is two UTF-16 units.
const texts = ["", "a", "héllo", "ß", "日本", "🦀x", "  pad ", "A,b,,c", "Ω"];
const chars = ["a", "Z", "é", "ß", "7", " ", "🦀", "日"];

const bits = (ty: IntTy) => Number(ty.slice(1));
const signed = (ty: IntTy) => ty.startsWith("i");
const range = (ty: IntTy): [bigint, bigint] => {
  const n = BigInt(bits(ty));
  return signed(ty) ? [-(1n << (n - 1n)), (1n << (n - 1n)) - 1n] : [0n, (1n << n) - 1n];
};

export type Expr =
  | { kind: "lit"; ty: Ty; value: bigint | boolean }
  // A float, as Rust writes it, `0.1f64`, `f64::NAN`.
  | { kind: "flit"; ty: Ty; text: string }
  | { kind: "zero"; ty: Ty }
  | { kind: "var"; ty: Ty; name: string }
  | { kind: "bin"; ty: Ty; op: string; a: Expr; b: Expr }
  | { kind: "shift"; ty: Ty; op: "<<" | ">>"; a: Expr; b: Expr }
  | { kind: "method"; ty: Ty; name: string; a: Expr; b?: Expr }
  | { kind: "cast"; ty: Ty; a: Expr }
  | { kind: "cmp"; ty: Ty; op: string; a: Expr; b: Expr }
  | { kind: "logic"; ty: Ty; op: "&&" | "||"; a: Expr; b: Expr }
  | { kind: "not"; ty: Ty; a: Expr }
  | { kind: "unary-neg"; ty: Ty; a: Expr }
  | { kind: "if"; ty: Ty; c: Expr; a: Expr; b: Expr }
  // `vec![a, b]`, `v[i]`, `S { a, b, c }`, `s.a`, `f(a)`, and what a
  // `Vec` or an `Option` can say, with or without a closure.
  | { kind: "vec"; ty: Ty; items: Expr[] }
  | { kind: "index"; ty: Ty; a: Expr; at: number }
  | { kind: "struct"; ty: Ty; a: Expr; b: Expr; c: Expr }
  | { kind: "field"; ty: Ty; a: Expr; field: string }
  | { kind: "call"; ty: Ty; name: string; a: Expr }
  | { kind: "some"; ty: Ty; a: Expr }
  | { kind: "use"; ty: Ty; form: string; a: Expr; b?: Expr; c?: Expr }
  // A `String` or a `char` literal, a variant of `E`, and a `match` on one,
  // whose arms bind what the variant holds.
  | { kind: "text"; ty: Ty; value: string }
  | { kind: "enum"; ty: Ty; variant: "A" | "B" | "C"; a?: Expr; b?: Expr }
  | { kind: "match"; ty: Ty; a: Expr; whenA: Expr; bind: string; guard?: Expr; whenB: Expr; otherB?: Expr; x: string; y: string; whenC: Expr }
  // `({ x = a; b })`: a write, then a value, so what's read before and after
  // it says in which order an expression's parts run.
  | { kind: "write"; ty: Ty; name: string; value: Expr; result: Expr }
  // `note(3, a)`, which prints its tag, and `bump(&mut s)`, which changes
  // `s.a` and gives it: effects inside an expression, which JS may run in
  // another order than Rust does.
  | { kind: "note"; ty: Ty; tag: number; a: Expr }
  | { kind: "bump"; ty: Ty; name: string }
  // `D(7).0`, a temporary dropped at the end of its statement, and `d3.0`,
  // a `D` held: what's dropped, and when, is printed (ADR 0092).
  | { kind: "dtemp"; ty: Ty; tag: number }
  | { kind: "dread"; ty: Ty; name: string }
  // An iterator's chain, `v.into_iter().map(|x| ..).filter(|&x| ..).sum()`,
  // whose closures may `note`: Rust's iterators are lazy, each item through
  // every step before the next, and an end may stop early, `next()`,
  // `any(..)`, so what's noted, and when, says in which order they run.
  | { kind: "chain"; ty: Ty; elem: IntTy; source: Expr; rev: boolean; steps: Step[]; end: End }
  // `loop` as an expression, `break value` where its test holds or its
  // count runs out, its counter `c3` in scope; and a `match` of an integer
  // whose arms are a value, values `1 | 2`, a range `3..=9`, or a binding
  // with a guard, `n if n > 4`.
  | { kind: "loop"; ty: Ty; counter: string; n: number; c: Expr; value: Expr }
  // `S`'s methods: `s.get()`; `s.add(n)`, of `&mut self`, its argument
  // reading `s`, as Rust's two-phase borrow lets it, never writing it; and
  // `s.scaled(k).a`, of `self`, copied before its argument, which may write
  // `s`, is made: JS passes an object, not a copy.
  | { kind: "method-s"; ty: Ty; name: string; method: "get" | "add" | "scaled"; arg?: Expr }
  // A call of a function `main` declares, its arguments in order: one of
  // an `Option`, its value; one of a `Result`, `unwrap_or(d)` or `is_ok()`.
  | { kind: "fcall"; ty: Ty; name: string; a: Expr; b: Expr; ending?: { op: "unwrap_or"; value: Expr } | { op: "is_ok" } }
  // The trait `Shape`, by `dyn` and by a generic: `vec![Box::new(Sq(a)),
  // Box::new(Rect(a, b))]` of `Box<dyn Shape>`, its areas summed by each
  // one's own `area`, `describe(&*v[i])` of its `name`, a default one's or
  // `Rect`'s own, and `total(&[Sq(a), Sq(b)])`, generic, made for `Sq`.
  | { kind: "shapes"; ty: Ty; items: { sq: boolean; a: Expr; b: Expr }[] }
  | { kind: "areas"; ty: Ty; name: string }
  | { kind: "describe"; ty: Ty; name: string; at: number }
  | { kind: "total"; ty: Ty; a: Expr; b: Expr }
  // `Rc<RefCell<i64>>`, `Shared`: made, or shared, `Rc::clone(&r)`, so a
  // write through one is read through the other; read, `*r.borrow()`; and
  // counted, `Rc::strong_count(&r)`.
  | { kind: "shared"; ty: Ty; value?: Expr; of?: string }
  | { kind: "shared-read"; ty: Ty; name: string; count: boolean }
  // A `Pair`, `(i32, u8)`, made, `(a, b)`, its parts read, `p.0`, by the
  // `field` kind; and a `match` of two values at once, `match (a, b) {
  // (1, _) => .., (_, 2) => .., (x, y) if x > 0 => .., _ => .. }`.
  | { kind: "pair"; ty: Ty; a: Expr; b: Expr }
  | { kind: "tmatch"; ty: Ty; a: Expr; b: Expr; first: string; second: string; x: string; y: string; guard: Expr; whenFirst: Expr; whenSecond: Expr; whenGuard: Expr; other: Expr }
  | { kind: "imatch"; ty: Ty; scrut: Expr; arms: { pat: string; bind?: string; guard?: Expr; value: Expr }[]; other: Expr };

// A closure's parameters keep the names they were made with, so a chain in
// another's closure never takes the outer's name for its own.
export type FnStep =
  | { op: "try"; name: string; how: "add" | "sub" | "mul" | "div"; a: Expr; b: Expr }
  | { op: "early"; c: Expr; value?: Expr };

export type Step = { op: "map" | "filter"; x: string; body: Expr } | { op: "take" | "skip"; n: number };
export type End =
  | { op: "collect" | "sum" | "count" | "max" | "min" | "last" | "next" }
  | { op: "any" | "all"; x: string; body: Expr }
  | { op: "fold"; init: Expr; x: string; acc: string; body: Expr };

export type Stmt =
  | { kind: "let"; name: string; ty: Ty; value: Expr }
  | { kind: "assign"; name: string; ty: Ty; op: string; value: Expr }
  | { kind: "print"; format: string; value: Expr }
  | { kind: "if"; c: Expr; then: Stmt[]; else: Stmt[] }
  | { kind: "for"; name: string; n: number; body: Stmt[] }
  | { kind: "for-each"; name: string; items: string; body: Stmt[] }
  | { kind: "if-let"; name: string; value: Expr; then: Stmt[]; else: Stmt[] }
  | { kind: "vec-op"; name: string; op: "push" | "pop" | "sort" | "reverse"; value?: Expr }
  // `v[i] = x`, `v[i] += x` or `s.a = x`; an index may write first,
  // `v[{ y = a; i }] = x`.
  | { kind: "set"; name: string; index?: number; field?: string; op: string; write?: { name: string; value: Expr }; note?: number; value: Expr }
  | { kind: "closure"; name: string; param: IntTy; body: Expr }
  // `let mut g = || { x = a; b };` and `y op= g()` in a block of their own:
  // a closure that captures `x` by reference and writes it, called where
  // `x`, or another, is assigned, as `x += g()` reads `x` after the call.
  | { kind: "call-mut"; name: string; captured: string; ty: IntTy; body: Expr; result: Expr; target: string; op: string; twice: boolean }
  | { kind: "text-op"; name: string; op: "push_str" | "push"; value: Expr }
  | { kind: "map-op"; name: string; op: "insert" | "remove" | "entry"; key: Expr; value?: Expr }
  | { kind: "for-map"; key: string; item: string; map: string; body: Stmt[] }
  | { kind: "if-let-b"; name: string; value: Expr; then: Stmt[]; else: Stmt[] }
  // A `D`, which prints its tag as it's dropped: held, `let mut d3 =
  // D(3);`, dropped at its block's end, or not, `let _ = D(3);`, dropped at
  // once; replaced, `d3 = D(4);`, the old one dropped after the new one is
  // made; and moved, `drop(d3)`, `take(d3)` or `let d5 = d3;`, only where
  // it was made, so it's never moved twice nor read after.
  | { kind: "dlet"; name: string; tag: number; bound: boolean }
  | { kind: "dset"; name: string; tag: number }
  | { kind: "dmove"; name: string; how: "drop" | "take" | "let"; into?: string }
  // A bounded `while`, `let mut w3 = 0; while w3 < n && c { w3 += 1; .. }`,
  // its test read each time round; and loops one in another, the outer
  // labeled, left or gone round again from the inner, `break 'l3` and
  // `continue 'l3`.
  | { kind: "while"; counter: string; n: number; c: Expr; body: Stmt[] }
  // A `Shape` pushed, and each grown through `&mut dyn Shape`, `for s in
  // v.iter_mut() { s.grow(by) }`, `by` made each time round.
  | { kind: "shape-push"; name: string; sq: boolean; a: Expr; b: Expr }
  | { kind: "shape-grow"; name: string; by: Expr }
  // A write through a `Shared`, `*r.borrow_mut() += e`, `e` made before
  // the borrow, which may read it too, its own borrow over by then.
  | { kind: "shared-write"; name: string; op: string; value: Expr }
  // Destructuring: a `let` of a `Pair`'s parts, `let (mut x, mut y) = p;`,
  // or an `S`'s, `let S { a: mut x, c: mut y, .. } = s;`; an assignment of
  // two at once, `(x, y) = (e1, e2);`, whose right side is all made before
  // either is written, as a swap needs; and a slice's first and last,
  // `if let &[x, .., y] = v.as_slice() { .. } else { .. }`.
  | { kind: "let-pair"; x: string; y: string; from: "Pair" | "S"; value: Expr }
  | { kind: "assign-two"; x: string; y: string; a: Expr; b: Expr }
  | { kind: "slice-pat"; x: string; y: string; name: string; then: Stmt[]; else: Stmt[] }
  // A `Vec` sorted where ties show, each stable as JS's `sort` is: by a key,
  // `sort_by_key(|x| *x % k)`, which keeps equal ones in order, by a
  // comparator of two, filtered by a closure that may note, `retain`, and
  // `dedup_by_key`.
  | { kind: "vec-sort"; name: string; elem: IntTy; how: "key" | "by" | "retain" | "dedup"; k: number; keep?: Expr; x?: string }
  // A function of two integers, declared in `main`, that gives an `Option`
  // or a `Result`: steps `let x = a.checked_add(b)?;`, each of which may
  // give up, and `if c { return .. }`, then its value. What it notes says
  // which steps ran before one gave up.
  | { kind: "fn"; name: string; ty: IntTy; result: "Option" | "Result"; params: [string, string]; steps: FnStep[]; tail: Expr }
  | { kind: "labeled"; label: string; outer: string; inner: string; n: number; m: number; brk: Expr; cont: Expr; body: Stmt[] };

export type Program = Stmt[];

// A loop's or a pattern's variable can be read, not written.
type Scope = { name: string; ty: Ty; mutable: boolean }[];

function literal(r: Random, ty: Ty): Expr {
  if (ty === "bool") return { kind: "lit", ty, value: r.chance(0.5) };
  if (ty === "f64" || ty === "f32") return { kind: "flit", ty, text: r.pick(floatEdges[ty]) };
  if (ty === "Pair") return { kind: "pair", ty, a: literal(r, pairParts[0]), b: literal(r, pairParts[1]) };
  if (ty === "String") return { kind: "text", ty, value: r.pick(texts) };
  if (ty === "char") return { kind: "text", ty, value: r.pick(chars) };
  if (ty === "E") {
    const variant = r.pick(["A", "B", "C"] as const);
    if (variant === "A") return { kind: "enum", ty, variant };
    if (variant === "B") return { kind: "enum", ty, variant, a: literal(r, "i32") };
    return { kind: "enum", ty, variant, a: literal(r, "u8"), b: literal(r, "bool") };
  }
  if (!isInt(ty)) return { kind: "zero", ty };
  const [lo, hi] = range(ty);
  // Where JS and Rust are most likely to part: the edges.
  const edges = [0n, 1n, 2n, lo, hi, lo + 1n, hi - 1n, hi / 2n, 1n << 31n, (1n << 53n) + 1n, (1n << 32n) - 1n];
  const values = signed(ty) ? [...edges, -1n, -2n, -(1n << 31n)] : edges;
  const inRange = values.filter((v) => v >= lo && v <= hi);
  const value = r.chance(0.7) ? r.pick(inRange) : lo + BigInt(r.int(Number(hi - lo > 1000n ? 1000n : hi - lo + 1n)));
  return { kind: "lit", ty, value };
}

// What an expression may write: a variable of what's `Copy`, but not a
// closure, which is never assigned.
const assignable = (scope: Scope) => scope.filter((v) => v.mutable && isCopy(v.ty) && !v.ty.startsWith("fn("));

function expr(r: Random, ty: Ty, scope: Scope, depth: number): Expr {
  if (ty === "Shared") {
    const others = scope.filter((v) => v.ty === "Shared");
    return others.length > 0 && r.chance(0.5)
      ? { kind: "shared", ty, of: r.pick(others).name }
      : { kind: "shared", ty, value: expr(r, "i64", scope, Math.min(1, depth)) };
  }
  // Shapes are made, never read whole: a `Box<dyn Shape>` isn't `Clone`.
  if (ty === "Shapes") {
    const items = Array.from({ length: 1 + r.int(3) }, () => ({ sq: r.chance(0.5), a: expr(r, "i64", scope, Math.min(1, depth)), b: expr(r, "i64", scope, Math.min(1, depth)) }));
    return { kind: "shapes", ty, items };
  }
  const vars = scope.filter((v) => v.ty === ty);
  const sub = (t: Ty) => expr(r, t, scope, depth - 1);
  const holding = (outer: "Vec" | "Option", elem: IntTy) => scope.filter((v) => v.ty === `${outer}<${elem}>`);
  if (depth <= 0 || r.chance(0.2)) {
    if (vars.length > 0 && r.chance(0.6)) return { kind: "var", ty, name: r.pick(vars).name };
    if (ty === "S") return { kind: "struct", ty, a: literal(r, "i32"), b: literal(r, "u8"), c: literal(r, "i64") };
    // Mostly something to work on: a few items, or `Some` of one.
    const vecOf = inside(ty, "Vec"), optionOf = inside(ty, "Option");
    if (vecOf) return { kind: "vec", ty, items: Array.from({ length: r.chance(0.1) ? 0 : 1 + r.int(4) }, () => literal(r, vecOf)) };
    if (optionOf && r.chance(0.7)) return { kind: "some", ty, a: literal(r, optionOf) };
    return literal(r, ty);
  }
  const written = assignable(scope);
  if (written.length > 0 && r.chance(0.08)) {
    const w = r.pick(written);
    return { kind: "write", ty, name: w.name, value: sub(w.ty), result: sub(ty) };
  }
  if (r.chance(0.05)) return { kind: "note", ty, tag: r.int(100), a: sub(ty) };
  if (r.chance(0.12)) {
    const chained = chain(r, ty, scope, depth);
    if (chained) return chained;
  }
  const shared = scope.filter((v) => v.ty === "Shared");
  if (ty === "i64" && shared.length > 0 && r.chance(0.2)) return { kind: "shared-read", ty, name: r.pick(shared).name, count: r.chance(0.3) };
  const shapes = scope.filter((v) => v.ty === "Shapes");
  if (ty === "i64" && r.chance(0.12)) {
    if (shapes.length > 0 && r.chance(0.6)) return { kind: "areas", ty, name: r.pick(shapes).name };
    return { kind: "total", ty, a: expr(r, "i64", scope, depth - 1), b: expr(r, "i64", scope, depth - 1) };
  }
  if (ty === "String" && shapes.length > 0 && r.chance(0.25)) return { kind: "describe", ty, name: r.pick(shapes).name, at: r.chance(0.85) ? 0 : r.int(4) };
  const optionFns = scope.filter((v) => v.ty === `opt-fn<${inside(ty, "Option")}>`);
  if (optionFns.length > 0 && r.chance(0.4)) {
    const elem = inside(ty, "Option")!;
    return { kind: "fcall", ty, name: r.pick(optionFns).name, a: expr(r, elem, scope, depth - 1), b: expr(r, elem, scope, depth - 1) };
  }
  const resultFns = scope.filter((v) => v.ty === `res-fn<${ty}>`);
  if (isInt(ty) && resultFns.length > 0 && r.chance(0.3)) {
    const ending = { op: "unwrap_or" as const, value: expr(r, ty, scope, depth - 1) };
    return { kind: "fcall", ty, name: r.pick(resultFns).name, a: expr(r, ty, scope, depth - 1), b: expr(r, ty, scope, depth - 1), ending };
  }
  const anyResult = scope.filter((v) => v.ty.startsWith("res-fn<"));
  if (ty === "bool" && anyResult.length > 0 && r.chance(0.2)) {
    const f = r.pick(anyResult);
    const elem = f.ty.slice(7, -1);
    return { kind: "fcall", ty, name: f.name, a: expr(r, elem, scope, depth - 1), b: expr(r, elem, scope, depth - 1), ending: { op: "is_ok" } };
  }
  const structsHere = scope.filter((v) => v.ty === "S");
  if (ty === "i32" && structsHere.length > 0 && r.chance(0.35)) {
    const s = r.pick(structsHere);
    const method = r.pick(["get", "add", "scaled"] as const);
    if (method === "get") return { kind: "method-s", ty, name: s.name, method };
    if (method === "add" && s.mutable) {
      // What it's given reads `s`, and writes anything else.
      const reading = scope.map((v) => (v.name === s.name ? { ...v, mutable: false } : v));
      return { kind: "method-s", ty, name: s.name, method, arg: expr(r, "i32", reading, depth - 1) };
    }
    if (method === "scaled") return { kind: "method-s", ty, name: s.name, method, arg: expr(r, "i32", scope, depth - 1) };
  }
  if (isInt(ty) && r.chance(0.06)) {
    const counter = `c${r.int(1_000_000)}`;
    const counted = [...scope, { name: counter, ty: "u32", mutable: false }];
    return { kind: "loop", ty, counter, n: 1 + r.int(4), c: expr(r, "bool", counted, depth - 1), value: expr(r, ty, counted, depth - 1) };
  }
  if (isInt(ty) && r.chance(0.06)) {
    const t = r.pick(intTypes);
    const value = () => (literal(r, t) as { value: bigint }).value;
    const arms = Array.from({ length: 1 + r.int(3) }, () => {
      const form = r.int(4);
      if (form === 0) return { pat: `${value()}`, value: expr(r, ty, scope, depth - 1) };
      if (form === 1) return { pat: `${value()} | ${value()}`, value: expr(r, ty, scope, depth - 1) };
      if (form === 2) {
        const [a, b] = [value(), value()];
        return { pat: `${a < b ? a : b}..=${a < b ? b : a}`, value: expr(r, ty, scope, depth - 1) };
      }
      const bind = `n${r.int(1_000_000)}`;
      const bound = [...scope, { name: bind, ty: t, mutable: false }];
      return { pat: bind, bind, guard: expr(r, "bool", bound, depth - 1), value: expr(r, ty, bound, depth - 1) };
    });
    return { kind: "imatch", ty, scrut: expr(r, t, scope, depth - 1), arms, other: expr(r, ty, scope, depth - 1) };
  }
  if (ty === "u32" && r.chance(0.1)) {
    const held = scope.filter((v) => v.ty === "D");
    return held.length > 0 && r.chance(0.5) ? { kind: "dread", ty, name: r.pick(held).name } : { kind: "dtemp", ty, tag: 1000 + r.int(1000) };
  }
  const structs = written.filter((v) => v.ty === "S");
  if (ty === "i32" && structs.length > 0 && r.chance(0.3)) return { kind: "bump", ty, name: r.pick(structs).name };
  // A float cast to an integer, which saturates, and `NaN` is 0; and what a
  // float says of itself, or of another.
  if (isInt(ty) && r.chance(0.06)) return { kind: "cast", ty, a: sub(r.pick(floatTypes)) };
  if (ty === "bool" && r.chance(0.12)) {
    const f = r.pick(floatTypes);
    return r.chance(0.5)
      ? { kind: "cmp", ty, op: r.pick(["==", "!=", "<", "<=", ">", ">="]), a: sub(f), b: sub(f) }
      : { kind: "method", ty, name: r.pick(["is_nan", "is_finite", "is_infinite", "is_sign_negative"]), a: sub(f) };
  }
  if (ty === "bool") {
    const choice = r.int(6);
    if (choice === 0) {
      const t = r.pick(intTypes);
      return { kind: "cmp", ty, op: r.pick(["==", "!=", "<", "<=", ">", ">="]), a: sub(t), b: sub(t) };
    }
    if (choice === 1) return { kind: "logic", ty, op: r.pick(["&&", "||"] as const), a: sub("bool"), b: sub("bool") };
    if (choice === 2) return { kind: "not", ty, a: sub("bool") };
    if (choice === 3) {
      const elem = r.pick(intTypes);
      return r.chance(0.5)
        ? { kind: "use", ty, form: "contains", a: sub(`Vec<${elem}>`), b: sub(elem) }
        : { kind: "use", ty, form: r.pick(["is_empty"]), a: sub(`Vec<${elem}>`) };
    }
    if (choice === 4) return { kind: "use", ty, form: r.pick(["is_some", "is_none"]), a: sub(`Option<${r.pick(intTypes)}>`) };
    if (choice === 5 && r.chance(0.5)) return { kind: "cmp", ty, op: r.pick(["==", "!="]), a: sub("S"), b: sub("S") };
    // What text, a `char`, an `E` and a map can say.
    if (r.chance(0.5)) {
      switch (r.int(6)) {
        case 0:
          return { kind: "use", ty, form: r.pick(["str-contains", "starts_with"]), a: sub("String"), b: sub("String") };
        case 1:
          return { kind: "use", ty, form: "is_empty", a: sub("String") };
        case 2:
          return { kind: "use", ty, form: r.pick(["is_alphabetic", "is_numeric", "is_uppercase", "is_ascii_digit", "is_whitespace"]), a: sub("char") };
        case 3: {
          const t = r.pick(["String", "char", "E"]);
          return { kind: "cmp", ty, op: t === "E" ? r.pick(["==", "!="]) : r.pick(["==", "!=", "<", ">="]), a: sub(t), b: sub(t) };
        }
        case 4:
          return { kind: "use", ty, form: "matches-b", a: sub("E") };
        default:
          return { kind: "use", ty, form: "contains_key", a: sub("Map"), b: sub("u8") };
      }
    }
    return { kind: "if", ty, c: sub("bool"), a: sub("bool"), b: sub("bool") };
  }
  if (ty === "Map") return vars.length > 0 ? { kind: "var", ty, name: r.pick(vars).name } : { kind: "zero", ty };
  if (ty === "Pair") return { kind: "pair", ty, a: sub(pairParts[0]), b: sub(pairParts[1]) };
  // A `Pair`'s part, and a `match` of two values at once.
  if ((ty === "i32" || ty === "u8") && r.chance(0.08)) return { kind: "field", ty, a: sub("Pair"), field: ty === "i32" ? "0" : "1" };
  if (isInt(ty) && r.chance(0.05)) {
    const [t1, t2] = [r.pick(intTypes), r.pick(intTypes)];
    const x = `tx${r.int(1_000_000)}`, y = `ty${r.int(1_000_000)}`;
    const bound = [...scope, { name: x, ty: t1, mutable: false }, { name: y, ty: t2, mutable: false }];
    return {
      kind: "tmatch",
      ty,
      a: sub(t1),
      b: sub(t2),
      first: `${(literal(r, t1) as { value: bigint }).value}`,
      second: `${(literal(r, t2) as { value: bigint }).value}`,
      x,
      y,
      guard: expr(r, "bool", bound, depth - 1),
      whenFirst: sub(ty),
      whenSecond: sub(ty),
      whenGuard: expr(r, ty, bound, depth - 1),
      other: sub(ty),
    };
  }
  if (isFloat(ty)) {
    const other = ty === "f64" ? "f32" : "f64";
    switch (r.int(9)) {
      case 0:
      case 1:
        return { kind: "bin", ty, op: r.pick(["+", "-", "*", "/", "%"]), a: sub(ty), b: sub(ty) };
      case 2:
        return { kind: "method", ty, name: r.pick(["abs", "floor", "ceil", "round", "trunc", "sqrt", "signum", "fract"]), a: sub(ty) };
      case 3:
        return { kind: "method", ty, name: r.pick(["min", "max"]), a: sub(ty), b: sub(ty) };
      case 4:
        return { kind: "method", ty, name: "powi", a: sub(ty), b: { kind: "lit", ty: "i32", value: BigInt(r.int(7) - 2) } };
      case 5:
        return { kind: "cast", ty, a: sub(r.pick(intTypes)) };
      case 6:
        return { kind: "cast", ty, a: sub(other) };
      case 7:
        return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
      default:
        return { kind: "unary-neg", ty, a: sub(ty) };
    }
  }
  if (ty === "String") {
    switch (r.int(7)) {
      case 0:
        return { kind: "use", ty, form: r.pick(["to_uppercase", "to_lowercase", "trim", "rev"]), a: sub(ty) };
      case 1:
        return { kind: "use", ty, form: "replace", a: sub(ty), b: literal(r, ty), c: literal(r, ty) };
      case 2:
        return { kind: "use", ty, form: "concat", a: sub(ty), b: sub(r.pick(["String", "char"])) };
      case 3:
        return { kind: "use", ty, form: "pad", a: sub(ty) };
      case 4:
        return { kind: "use", ty, form: "to_string", a: sub(r.chance(0.6) ? r.pick(intTypes) : "char") };
      default:
        return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
    }
  }
  if (ty === "char") {
    switch (r.int(4)) {
      case 0:
        return { kind: "use", ty, form: "first-char", a: sub("String"), b: sub("char") };
      case 1:
        return { kind: "use", ty, form: "to_ascii_uppercase", a: sub("char") };
      case 2:
        return { kind: "use", ty, form: "from-digit", a: sub("u32") };
      default:
        return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
    }
  }
  if (ty === "E") {
    const variant = r.pick(["A", "B", "C"] as const);
    if (variant === "A") return r.chance(0.5) ? { kind: "enum", ty, variant } : { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
    return variant === "B" ? { kind: "enum", ty, variant, a: sub("i32") } : { kind: "enum", ty, variant, a: sub("u8"), b: sub("bool") };
  }
  const vecOf = inside(ty, "Vec");
  if (vecOf) {
    switch (r.int(4)) {
      case 0:
        return { kind: "vec", ty, items: Array.from({ length: r.int(5) }, () => sub(vecOf)) };
      case 1:
        return { kind: "use", ty, form: "map", a: sub(ty), b: sub(vecOf) };
      case 2:
        return { kind: "use", ty, form: "filter", a: sub(ty), b: sub(vecOf) };
      default:
        return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
    }
  }
  const optionOf = inside(ty, "Option");
  if (optionOf) {
    switch (r.int(5)) {
      case 0:
        return { kind: "some", ty, a: sub(optionOf) };
      case 1:
        return { kind: "method", ty, name: r.pick(["checked_add", "checked_sub", "checked_mul", "checked_div"]), a: sub(optionOf), b: sub(optionOf) };
      case 2:
        return { kind: "use", ty, form: r.pick(["first", "last", "max", "min"]), a: sub(`Vec<${optionOf}>`) };
      case 3:
        return { kind: "use", ty, form: "map-option", a: sub(ty), b: sub(optionOf) };
      default:
        return { kind: "zero", ty };
    }
  }
  if (ty === "S") return { kind: "struct", ty, a: sub("i32"), b: sub("u8"), c: sub("i64") };
  // An integer.
  const int = ty as IntTy;
  const closures = scope.filter((v) => v.ty === `fn(${int})`);
  switch (r.int(12)) {
    case 0:
    case 1:
      return { kind: "bin", ty, op: r.pick(["+", "-", "*", "/", "%", "&", "|", "^"]), a: sub(ty), b: sub(ty) };
    case 2:
      return { kind: "shift", ty, op: r.pick(["<<", ">>"] as const), a: sub(ty), b: sub(r.pick(intTypes)) };
    case 3: {
      const binary = ["wrapping_add", "wrapping_sub", "wrapping_mul", "saturating_add", "saturating_sub", "saturating_mul", "max", "min"];
      const unary = ["count_ones", "leading_zeros", "trailing_zeros", ...(signed(int) ? ["abs"] : [])];
      if (r.chance(0.7)) return { kind: "method", ty, name: r.pick(binary), a: sub(ty), b: sub(ty) };
      const name = r.pick(unary);
      // What a count counts is a `u32`.
      if (name !== "abs") return { kind: "cast", ty, a: { kind: "method", ty: "u32", name, a: sub(ty) } };
      return { kind: "method", ty, name, a: sub(ty) };
    }
    case 4:
      return { kind: "cast", ty, a: sub(r.pick(intTypes)) };
    case 5: {
      // From a `Vec`: an item, which may not be there and panic, its sum, or
      // its length.
      const choice = r.int(3);
      if (choice === 0) return { kind: "index", ty, a: sub(`Vec<${int}>`), at: r.chance(0.8) ? 0 : r.int(4) };
      if (choice === 1) return { kind: "use", ty, form: "sum", a: sub(`Vec<${int}>`) };
      return { kind: "cast", ty, a: { kind: "use", ty: "usize", form: "len", a: sub(`Vec<${r.pick(intTypes)}>`) } };
    }
    case 6:
      return { kind: "use", ty, form: "unwrap_or", a: sub(`Option<${int}>`), b: sub(ty) };
    case 7: {
      const field = struct.fields.find(([, t]) => t === int);
      if (field) return { kind: "field", ty, a: sub("S"), field: field[0] };
      if (closures.length > 0) return { kind: "call", ty, name: r.pick(closures).name, a: sub(ty) };
      return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
    }
    case 9:
      // How many chars, or pieces between commas, a string has; a char's code.
      if (r.chance(0.6)) return { kind: "use", ty, form: r.pick(["char-count", "split-count"]), a: sub("String") };
      return { kind: "cast", ty, a: { kind: "use", ty: "u32", form: "char-code", a: sub("char") } };
    case 10: {
      // What a map holds.
      if (int === "i32" && r.chance(0.6)) {
        return r.chance(0.5)
          ? { kind: "use", ty, form: "map-get", a: sub("Map"), b: sub("u8"), c: sub("i32") }
          : { kind: "use", ty, form: "map-sum", a: sub("Map") };
      }
      return { kind: "use", ty, form: "map-len", a: sub("Map") };
    }
    case 11: {
      // A `match` on an `E`, whose arms have what its variant holds, and
      // sometimes a guard.
      const n = r.int(1e6);
      const bind = `m${n}`, x = `mx${n}`, y = `my${n}`;
      const withB = [...scope, { name: bind, ty: "i32", mutable: false }];
      const withC = [...scope, { name: x, ty: "u8", mutable: false }, { name: y, ty: "bool", mutable: false }];
      const guarded = r.chance(0.4);
      return {
        kind: "match",
        ty,
        a: sub("E"),
        whenA: sub(ty),
        bind,
        // A guard writes nothing: what the match reads is borrowed through
        // it, which borrowck keeps unchanged.
        guard: guarded ? expr(r, "bool", withB.map((v) => ({ ...v, mutable: false })), depth - 1) : undefined,
        whenB: expr(r, ty, withB, depth - 1),
        otherB: guarded ? sub(ty) : undefined,
        x,
        y,
        whenC: expr(r, ty, withC, depth - 1),
      };
    }
    default:
      if (closures.length > 0 && r.chance(0.5)) return { kind: "call", ty, name: r.pick(closures).name, a: sub(ty) };
      return { kind: "if", ty, c: sub("bool"), a: sub(ty), b: sub(ty) };
  }
}

// A value's type for a new variable: mostly integers, and the rest.
/** An iterator's chain whose end is `ty`, of what's in `scope`, or none
 * where no end is one: its closures see what's `Copy` of it, and their
 * own parameters, `x` and a fold's `acc`. */
function chain(r: Random, ty: Ty, scope: Scope, depth: number): Expr | undefined {
  if (depth <= 1) return undefined;
  const vecOf = inside(ty, "Vec"), optionOf = inside(ty, "Option");
  const captures = scope.filter((v) => isInt(v.ty) || v.ty === "bool").map((v) => ({ ...v, mutable: false }));
  const closure = (elem: IntTy, result: Ty, acc = false) => {
    const x = `x${r.int(1_000_000)}`, a = `acc${r.int(1_000_000)}`;
    const bound = [...captures, { name: x, ty: elem, mutable: false }, ...(acc ? [{ name: a, ty: elem, mutable: false }] : [])];
    return { x, acc: a, body: expr(r, result, bound, depth - 1) };
  };
  let elem: IntTy;
  let end: End;
  if (vecOf) {
    elem = vecOf;
    end = { op: "collect" };
  } else if (optionOf) {
    elem = optionOf;
    end = { op: r.pick(["max", "min", "last", "next"] as const) };
  } else if (ty === "bool") {
    elem = r.pick(intTypes);
    const { x, body } = closure(elem, "bool");
    end = { op: r.pick(["any", "all"] as const), x, body };
  } else if (ty === "u32" && r.chance(0.3)) {
    elem = r.pick(intTypes);
    end = { op: "count" };
  } else if (isInt(ty)) {
    elem = ty;
    if (r.chance(0.5)) {
      end = { op: "sum" };
    } else {
      const init = expr(r, ty, scope, 1);
      const { x, acc, body } = closure(ty, ty, true);
      end = { op: "fold", init, x, acc, body };
    }
  } else {
    return undefined;
  }
  // No `skip` before a `collect`: std's `Vec` of a chain it knows is empty
  // runs none of it, where the JS's steps through it (ADR 0055's B).
  const ops = end.op === "collect" ? (["map", "map", "filter", "filter", "take"] as const) : (["map", "map", "filter", "filter", "take", "skip"] as const);
  const steps: Step[] = Array.from({ length: r.int(4) }, (): Step => {
    const op = r.pick(ops);
    if (op === "map" || op === "filter") {
      const { x, body } = closure(elem, op === "map" ? elem : "bool");
      return { op, x, body };
    }
    return { op, n: r.int(4) };
  });
  return { kind: "chain", ty, elem, source: expr(r, `Vec<${elem}>`, scope, depth - 1), rev: r.chance(0.3), steps, end };
}

function valueType(r: Random): Ty {
  if (r.chance(0.08)) return r.pick(floatTypes);
  if (r.chance(0.05)) return "Pair";
  const n = r.next();
  if (n < 0.45) return r.pick(intTypes);
  if (n < 0.52) return "bool";
  if (n < 0.62) return `Vec<${r.pick(intTypes)}>`;
  if (n < 0.7) return `Option<${r.pick(intTypes)}>`;
  if (n < 0.79) return "S";
  if (n < 0.86) return "String";
  if (n < 0.89) return "char";
  if (n < 0.95) return "E";
  return "Map";
}

function block(r: Random, scope: Scope, depth: number, counter: { n: number }, size: number): Stmt[] {
  const stmts: Stmt[] = [];
  const inner = [...scope];
  // The `D`s made in this block, which only it may move.
  const owned: string[] = [];
  for (let i = 0; i < size; i++) {
    const writable = inner.filter((v) => v.mutable && v.ty !== "D");
    const held = inner.filter((v) => v.ty === "D");
    const vecs = writable.filter((v) => inside(v.ty, "Vec"));
    const texts = writable.filter((v) => v.ty === "String");
    const maps = writable.filter((v) => v.ty === "Map");
    const choice = r.int(29);
    if (choice < 3 || writable.length === 0) {
      const ty = r.chance(0.08) ? "Shapes" : r.chance(0.09) ? "Shared" : valueType(r);
      const name = `v${counter.n++}`;
      stmts.push({ kind: "let", name, ty, value: expr(r, ty, inner, 3) });
      inner.push({ name, ty, mutable: true });
    } else if (choice < 5) {
      const target = r.pick(writable);
      const ops = isInt(target.ty) ? ["=", "+=", "-=", "*=", "^=", "|=", "&="] : isFloat(target.ty) ? ["=", "+=", "-=", "*=", "/="] : ["="];
      stmts.push({ kind: "assign", name: target.name, ty: target.ty, op: r.pick(ops), value: expr(r, target.ty, inner, 2) });
    } else if (choice < 8) {
      const ty = r.chance(0.7) ? r.pick(intTypes) : valueType(r);
      const format = isFloat(ty)
        ? r.pick(["{}", "{:?}", "{:.2}", "{:.0}", "{:e}", "{:10.3}|", "{:+}", "{:.1}"])
        : isInt(ty)
        ? r.pick(["{}", "{:?}", "{:x}", "{:#x}", "{:5}", "{:<4}|"])
        : ty === "bool"
          ? r.pick(["{}", "{:?}"])
          : ty === "String"
            ? r.pick(["{}", "{:?}", "{:>8}|", "{:<6}|", "{:^7}|"])
            : ty === "char"
              ? r.pick(["{}", "{:?}", "{:>3}|"])
              : "{:?}";
      stmts.push({ kind: "print", format, value: expr(r, ty, inner, 3) });
    } else if (choice === 26) {
      const from = r.pick(["Pair", "S"] as const);
      const [x, y] = [`v${counter.n++}`, `v${counter.n++}`];
      stmts.push({ kind: "let-pair", x, y, from, value: expr(r, from, inner, 2) });
      const [tx, ty] = from === "Pair" ? pairParts : (["i32", "i64"] as [IntTy, IntTy]);
      inner.push({ name: x, ty: tx, mutable: true }, { name: y, ty, mutable: true });
    } else if (choice === 27 && writable.filter((v) => isInt(v.ty)).length > 1) {
      // Two of one type, assigned at once: a swap, or what reads either.
      const ints = writable.filter((v) => isInt(v.ty));
      const x = r.pick(ints);
      const others = ints.filter((v) => v.name !== x.name);
      if (others.length > 0) {
        const y = r.pick(others);
        const swap = y.ty === x.ty && r.chance(0.4);
        stmts.push({
          kind: "assign-two",
          x: x.name,
          y: y.name,
          a: swap ? { kind: "var", ty: y.ty, name: y.name } : expr(r, x.ty, inner, 2),
          b: swap ? { kind: "var", ty: x.ty, name: x.name } : expr(r, y.ty, inner, 2),
        });
      }
    } else if (choice === 28 && depth > 0 && vecs.length > 0) {
      const v = r.pick(vecs);
      const elem = inside(v.ty, "Vec")!;
      const [x, y] = [`v${counter.n++}`, `v${counter.n++}`];
      const bound = [...inner, { name: x, ty: elem, mutable: false }, { name: y, ty: elem, mutable: false }];
      stmts.push({ kind: "slice-pat", x, y, name: v.name, then: block(r, bound, depth - 1, counter, 1 + r.int(3)), else: block(r, inner, depth - 1, counter, r.int(2)) });
    } else if (choice === 24 && writable.some((v) => v.ty === "Shared")) {
      const name = r.pick(writable.filter((v) => v.ty === "Shared")).name;
      stmts.push({ kind: "shared-write", name, op: r.pick(["=", "+=", "-=", "^="]), value: expr(r, "i64", inner, 2) });
    } else if (choice === 25 && vecs.length > 0) {
      const v = r.pick(vecs);
      const elem = inside(v.ty, "Vec")!;
      const how = r.pick(["key", "by", "retain", "dedup"] as const);
      const x = `x${counter.n++}`;
      // What `retain` keeps reads its item, and what's `Copy` but the `Vec`.
      const reading = [...inner.filter((u) => u.name !== v.name && (isInt(u.ty) || u.ty === "bool")).map((u) => ({ ...u, mutable: false })), { name: x, ty: elem, mutable: false }];
      stmts.push({ kind: "vec-sort", name: v.name, elem, how, k: 2 + r.int(3), x, keep: how === "retain" ? expr(r, "bool", reading, 2) : undefined });
    } else if (choice === 8 && vecs.length > 0) {
      const v = r.pick(vecs);
      const elem = inside(v.ty, "Vec")!;
      const op = r.pick(["push", "push", "pop", "sort", "reverse"] as const);
      stmts.push({ kind: "vec-op", name: v.name, op, value: op === "push" ? expr(r, elem, inner, 2) : undefined });
    } else if (choice === 9) {
      const structs = writable.filter((v) => v.ty === "S");
      if (vecs.length > 0 && r.chance(0.5)) {
        const v = r.pick(vecs);
        const elem = inside(v.ty, "Vec")!;
        const written = assignable(inner);
        const w = written.length > 0 && r.chance(0.5) ? r.pick(written) : undefined;
        stmts.push({
          kind: "set",
          name: v.name,
          index: r.chance(0.8) ? 0 : r.int(4),
          op: r.pick(["=", "=", "+=", "-=", "^="]),
          write: w && { name: w.name, value: expr(r, w.ty, inner, 2) },
          note: r.chance(0.3) ? r.int(100) : undefined,
          // Sometimes what the index wrote, read after it.
          value: w && w.ty === elem && r.chance(0.5) ? { kind: "var", ty: elem, name: w.name } : expr(r, elem, inner, 2),
        });
      } else if (structs.length > 0) {
        const [field, t] = r.pick(struct.fields);
        stmts.push({ kind: "set", name: r.pick(structs).name, field, op: r.pick(["=", "=", "+=", "|="]), value: expr(r, t, inner, 2) });
      }
    } else if (choice === 10) {
      // A closure captures what's `Copy`, by value, so a later write can't
      // conflict with it.
      const param = r.pick(intTypes);
      const name = `f${counter.n++}`;
      const x = `x${counter.n++}`;
      const captures = inner.filter((v) => isInt(v.ty) || v.ty === "bool").map((v) => ({ ...v, mutable: false }));
      const body = expr(r, param, [...captures, { name: x, ty: param, mutable: false }], 2);
      stmts.push({ kind: "closure", name, param, body: substitute(body, x) });
      inner.push({ name, ty: `fn(${param})`, mutable: false });
    } else if (choice === 11 && depth > 0) {
      stmts.push({ kind: "if", c: expr(r, "bool", inner, 2), then: block(r, inner, depth - 1, counter, 1 + r.int(3)), else: block(r, inner, depth - 1, counter, r.int(3)) });
    } else if (choice === 12 && depth > 0) {
      const elem = r.pick(intTypes);
      const name = `x${counter.n++}`;
      stmts.push({
        kind: "if-let",
        name,
        value: expr(r, `Option<${elem}>`, inner, 2),
        then: block(r, [...inner, { name, ty: elem, mutable: false }], depth - 1, counter, 1 + r.int(3)),
        else: block(r, inner, depth - 1, counter, r.int(2)),
      });
    } else if (choice === 14 && texts.length > 0) {
      const push = r.chance(0.5);
      const name = r.pick(texts).name;
      // What's appended reads no text, which a loop would double each turn,
      // itself or two each other's, until the program runs out of memory.
      const unread = inner.filter((v) => v.ty !== "String");
      stmts.push({ kind: "text-op", name, op: push ? "push" : "push_str", value: expr(r, push ? "char" : "String", push ? inner : unread, 2) });
    } else if (choice === 15 && maps.length > 0) {
      const op = r.pick(["insert", "insert", "entry", "remove"] as const);
      stmts.push({ kind: "map-op", name: r.pick(maps).name, op, key: expr(r, "u8", inner, 1), value: op === "remove" ? undefined : expr(r, "i32", inner, 2) });
    } else if (choice === 16 && depth > 0) {
      const all = inner.filter((v) => v.ty === "Map");
      if (all.length > 0) {
        const key = `k${counter.n++}`, item = `w${counter.n++}`;
        const bound = [...inner, { name: key, ty: "u8", mutable: false }, { name: item, ty: "i32", mutable: false }];
        stmts.push({ kind: "for-map", key, item, map: r.pick(all).name, body: block(r, bound, depth - 1, counter, 1 + r.int(3)) });
      }
    } else if (choice === 17 && depth > 0) {
      const name = `m${counter.n++}`;
      stmts.push({
        kind: "if-let-b",
        name,
        value: expr(r, "E", inner, 2),
        then: block(r, [...inner, { name, ty: "i32", mutable: false }], depth - 1, counter, 1 + r.int(3)),
        else: block(r, inner, depth - 1, counter, r.int(2)),
      });
    } else if (choice === 18 && writable.some((v) => isInt(v.ty))) {
      const ints = writable.filter((v) => isInt(v.ty));
      const x = r.pick(ints);
      const ty = x.ty as IntTy;
      // The closure sees only what it captures, so nothing else it reads is
      // borrowed while it lives.
      const only = [{ name: x.name, ty, mutable: true }];
      const others = ints.filter((v) => v.ty === ty && v.name !== x.name);
      stmts.push({
        kind: "call-mut",
        name: `g${counter.n++}`,
        captured: x.name,
        ty,
        body: expr(r, ty, only, 2),
        result: expr(r, ty, only, 2),
        target: others.length > 0 && r.chance(0.4) ? r.pick(others).name : x.name,
        op: r.pick(["=", "+=", "-=", "^="]),
        twice: r.chance(0.3),
      });
    } else if (choice === 13 && writable.some((v) => v.ty === "Shapes")) {
      const name = r.pick(writable.filter((v) => v.ty === "Shapes")).name;
      // What's grown by reads no shapes, which the loop borrows.
      const reading = inner.filter((v) => v.ty !== "Shapes");
      if (r.chance(0.5)) {
        stmts.push({ kind: "shape-push", name, sq: r.chance(0.5), a: expr(r, "i64", reading, 2), b: expr(r, "i64", reading, 2) });
      } else {
        stmts.push({ kind: "shape-grow", name, by: expr(r, "i64", reading, 2) });
      }
    } else if (choice === 22 && depth > 0) {
      const name = `w${counter.n++}`;
      const counted = [...inner, { name, ty: "u32", mutable: false }];
      stmts.push({ kind: "while", counter: name, n: 1 + r.int(4), c: expr(r, "bool", counted, 2), body: block(r, counted, depth - 1, counter, 1 + r.int(3)) });
    } else if (choice === 23 && depth > 0) {
      const label = `l${counter.n++}`, outer = `i${counter.n++}`, nested = `j${counter.n++}`;
      const counted = [...inner, { name: outer, ty: "u32", mutable: false }, { name: nested, ty: "u32", mutable: false }];
      stmts.push({
        kind: "labeled",
        label,
        outer,
        inner: nested,
        n: 1 + r.int(4),
        m: 1 + r.int(4),
        brk: expr(r, "bool", counted, 2),
        cont: expr(r, "bool", counted, 2),
        body: block(r, counted, depth - 1, counter, 1 + r.int(3)),
      });
    } else if (choice === 19) {
      const name = `d${counter.n++}`;
      const bound = r.chance(0.85);
      stmts.push({ kind: "dlet", name, tag: counter.n++, bound });
      if (bound) {
        inner.push({ name, ty: "D", mutable: true });
        owned.push(name);
      }
    } else if (choice === 20 && held.length > 0) {
      stmts.push({ kind: "dset", name: r.pick(held).name, tag: counter.n++ });
    } else if (choice === 21 && owned.length > 0) {
      const name = r.pick(owned);
      const how = r.pick(["drop", "take", "let"] as const);
      inner.splice(inner.findIndex((v) => v.name === name), 1);
      owned.splice(owned.indexOf(name), 1);
      if (how === "let") {
        const into = `d${counter.n++}`;
        stmts.push({ kind: "dmove", name, how, into });
        inner.push({ name: into, ty: "D", mutable: true });
        owned.push(into);
      } else {
        stmts.push({ kind: "dmove", name, how });
      }
    } else if (depth > 0) {
      const name = `k${counter.n++}`;
      const lists = inner.filter((v) => inside(v.ty, "Vec"));
      if (lists.length > 0 && r.chance(0.4)) {
        const list = r.pick(lists);
        stmts.push({ kind: "for-each", name, items: list.name, body: block(r, [...inner, { name, ty: inside(list.ty, "Vec")!, mutable: false }], depth - 1, counter, 1 + r.int(3)) });
      } else {
        stmts.push({ kind: "for", name, n: 1 + r.int(4), body: block(r, [...inner, { name, ty: "u32", mutable: false }], depth - 1, counter, 1 + r.int(3)) });
      }
    }
  }
  return stmts;
}

// A closure's parameter is `x`, whatever its generated name was, or `to`.
function substitute(e: Expr, name: string, to = "x"): Expr {
  if (e.kind === "var") return e.name === name ? { ...e, name: to } : e;
  const out: Record<string, unknown> = { ...e };
  for (const [key, value] of Object.entries(e)) {
    // An expression, or what holds one, a chain's step.
    if (Array.isArray(value)) out[key] = value.map((item) => substitute(item as Expr, name, to));
    else if (value && typeof value === "object") out[key] = substitute(value as Expr, name, to);
  }
  return out as Expr;
}

/** A function of two `ty`s, `name`'s, of `?`s and early returns. */
function fnDef(r: Random, name: string, ty: IntTy, result: "Option" | "Result", counter: { n: number }): Stmt {
  const params: [string, string] = [`p${counter.n++}`, `q${counter.n++}`];
  const scope: Scope = params.map((p) => ({ name: p, ty, mutable: false }));
  const steps: FnStep[] = [];
  for (let i = 0, n = 1 + r.int(4); i < n; i++) {
    if (r.chance(0.65)) {
      const name = `t${counter.n++}`;
      steps.push({ op: "try", name, how: r.pick(["add", "sub", "mul", "div"] as const), a: expr(r, ty, scope, 2), b: expr(r, ty, scope, 2) });
      scope.push({ name, ty, mutable: false });
    } else {
      steps.push({ op: "early", c: expr(r, "bool", scope, 2), value: r.chance(0.6) ? expr(r, ty, scope, 2) : undefined });
    }
  }
  return { kind: "fn", name, ty, result, params, steps, tail: expr(r, ty, scope, 2) };
}

/** The program seed `seed` says, the same each time. */
export function generate(seed: number): Program {
  const r = random(seed);
  const counter = { n: 0 };
  // Its functions first, which what follows may call.
  const fns: Stmt[] = [];
  for (let i = 0, n = r.int(3); i < n; i++) {
    const result = r.pick(["Option", "Result"] as const);
    fns.push(fnDef(r, `h${counter.n++}`, r.pick(intTypes), result, counter));
  }
  const scope: Scope = fns.map((f) => (f.kind === "fn" ? { name: f.name, ty: `${f.result === "Option" ? "opt" : "res"}-fn<${f.ty}>`, mutable: false } : undefined)!);
  return [...fns, ...block(r, scope, 2, counter, 6 + r.int(8))];
}

const suffix = (ty: Ty) => (ty === "bool" ? "" : ty);

function show(e: Expr): string {
  switch (e.kind) {
    case "lit":
      // Through `id`, so rustc can't work out an overflow and reject it.
      return typeof e.value === "boolean" ? `id(${e.value})` : `id(${e.value}${suffix(e.ty)})`;
    case "flit":
      return /^f(32|64)::/.test(e.text) ? `id(${e.text})` : `id(${e.text}${e.ty})`;
    case "unary-neg":
      return `(-${show(e.a)})`;
    case "zero":
      if (inside(e.ty, "Vec")) return `Vec::<${inside(e.ty, "Vec")}>::new()`;
      if (inside(e.ty, "Option")) return `None::<${inside(e.ty, "Option")}>`;
      if (e.ty === "S") return "(S { a: 0, b: 0, c: 0 })";
      if (e.ty === "String") return "String::new()";
      if (e.ty === "char") return "'a'";
      if (e.ty === "E") return "E::A";
      if (e.ty === "Map") return "BTreeMap::<u8, i32>::new()";
      if (isFloat(e.ty)) return `id(0.0${e.ty})`;
      return e.ty === "bool" ? "false" : `id(0${e.ty})`;
    case "var":
      return isCopy(e.ty) ? e.name : `${e.name}.clone()`;
    case "write":
      return `({ ${e.name} = ${show(e.value)}; ${show(e.result)} })`;
    case "note":
      return `note(${e.tag}, ${show(e.a)})`;
    case "bump":
      return `bump(&mut ${e.name})`;
    case "dtemp":
      return `D(id(${e.tag}u32)).0`;
    case "pair":
      return `(${show(e.a)}, ${show(e.b)})`;
    case "tmatch":
      return `(match (${show(e.a)}, ${show(e.b)}) { (${e.first}, _) => ${show(e.whenFirst)}, (_, ${e.second}) => ${show(e.whenSecond)}, (${e.x}, ${e.y}) if ${show(e.guard)} => ${show(e.whenGuard)}, _ => ${show(e.other)} })`;
    case "shared":
      return e.of ? `Rc::clone(&${e.of})` : `Rc::new(RefCell::new(${show(e.value!)}))`;
    case "shared-read":
      // The borrow ends where it's read: a `Ref` kept to the statement's end
      // would conflict with what assigns the `Rc` in it.
      return e.count ? `(Rc::strong_count(&${e.name}) as i64)` : `({ let b = *${e.name}.borrow(); b })`;
    case "shapes":
      return `vec![${e.items.map((i): string => (i.sq ? `Box::new(Sq(${show(i.a)})) as Box<dyn Shape>` : `Box::new(Rect(${show(i.a)}, ${show(i.b)})) as Box<dyn Shape>`)).join(", ")}]`;
    case "areas":
      return `${e.name}.iter().map(|s| s.area()).fold(id(0i64), |a, b| a.wrapping_add(b))`;
    case "describe":
      return `describe(&*${e.name}[id(${e.at}usize)])`;
    case "total":
      return `total(&[Sq(${show(e.a)}), Sq(${show(e.b)})])`;
    case "fcall": {
      const call = `${e.name}(${show(e.a)}, ${show(e.b)})`;
      if (!e.ending) return call;
      return e.ending.op === "is_ok" ? `${call}.is_ok()` : `${call}.unwrap_or(${show(e.ending.value)})`;
    }
    case "method-s":
      if (e.method === "get") return `${e.name}.get()`;
      if (e.method === "add") return `${e.name}.add(${show(e.arg!)})`;
      return `${e.name}.scaled(${show(e.arg!)}).a`;
    case "loop":
      return `({ let mut ${e.counter}: u32 = 0; loop { ${e.counter} += 1; if ${e.counter} > id(${e.n}u32) || ${show(e.c)} { break ${show(e.value)}; } } })`;
    case "imatch": {
      // A pattern's values are of the scrutinee's type, which says so.
      const arms = e.arms.map((arm) => `${arm.pat}${arm.guard ? ` if ${show(arm.guard)}` : ""} => ${show(arm.value)}, `);
      return `(match ${show(e.scrut)} { ${arms.join("")}_ => ${show(e.other)} })`;
    }
    case "chain": {
      const t = e.elem;
      const steps = e.steps.map((step) => {
        if ("n" in step) return `.${step.op}(id(${step.n}usize))`;
        if (step.op === "map") return `.map(move |${step.x}: ${t}| -> ${t} { ${show(step.body)} })`;
        return `.filter(move |&${step.x}: &${t}| ${show(step.body)})`;
      });
      const end = e.end;
      const tail =
        end.op === "collect"
          ? `.collect::<Vec<${t}>>()`
          : end.op === "sum"
            ? `.sum::<${t}>()`
            : end.op === "count"
              ? ".count() as u32"
              : end.op === "any" || end.op === "all"
                ? `.${end.op}(move |${end.x}: ${t}| ${show(end.body)})`
                : end.op === "fold"
                  ? `.fold(${show(end.init)}, move |${end.acc}: ${t}, ${end.x}: ${t}| -> ${t} { ${show(end.body)} })`
                  : `.${end.op}()`;
      return `(${show(e.source)}.into_iter()${e.rev ? ".rev()" : ""}${steps.join("")}${tail})`;
    }
    case "dread":
      return `${e.name}.0`;
    case "bin":
    case "cmp":
    case "logic":
    case "shift":
      return `(${show(e.a)} ${e.op} ${show(e.b)})`;
    case "method":
      return e.b ? `${show(e.a)}.${e.name}(${show(e.b)})` : `${show(e.a)}.${e.name}()`;
    case "cast":
      return `(${show(e.a)} as ${e.ty})`;
    case "not":
      return `!${show(e.a)}`;
    case "if":
      return `(if ${show(e.c)} { ${show(e.a)} } else { ${show(e.b)} })`;
    case "vec":
      return e.items.length === 0 ? `Vec::<${inside(e.ty, "Vec")}>::new()` : `vec![${e.items.map(show).join(", ")}]`;
    case "index":
      return `${show(e.a)}[id(${e.at}usize)]`;
    // In parentheses, as a struct literal can't begin an `if`'s condition.
    case "struct":
      return `(S { a: ${show(e.a)}, b: ${show(e.b)}, c: ${show(e.c)} })`;
    case "field":
      return `${show(e.a)}.${e.field}`;
    case "call":
      return `${e.name}(${show(e.a)})`;
    case "some":
      return `Some(${show(e.a)})`;
    case "text":
      return e.ty === "char" ? `'${e.value}'` : `String::from(${JSON.stringify(e.value)})`;
    case "enum":
      if (e.variant === "A") return "E::A";
      // In parentheses, as a struct-like variant can't begin an `if`'s condition.
      return e.variant === "B" ? `E::B(${show(e.a!)})` : `(E::C { x: ${show(e.a!)}, y: ${show(e.b!)} })`;
    case "match": {
      const arms = [
        `E::A => ${show(e.whenA)}`,
        `E::B(${e.bind})${e.guard ? ` if ${show(e.guard)}` : ""} => ${show(e.whenB)}`,
        ...(e.otherB ? [`E::B(_) => ${show(e.otherB)}`] : []),
        `E::C { x: ${e.x}, y: ${e.y} } => ${show(e.whenC)}`,
      ];
      return `(match ${show(e.a)} { ${arms.join(", ")} })`;
    }
    case "use": {
      const a = show(e.a), b = e.b ? show(e.b) : "", c = e.c ? show(e.c) : "";
      switch (e.form) {
        case "to_uppercase":
        case "to_lowercase":
        case "to_ascii_uppercase":
        case "is_alphabetic":
        case "is_numeric":
        case "is_uppercase":
        case "is_ascii_digit":
        case "is_whitespace":
        case "to_string":
          return `${a}.${e.form}()`;
        case "trim":
          return `${a}.trim().to_string()`;
        case "rev":
          return `${a}.chars().rev().collect::<String>()`;
        case "replace":
          return `${a}.replace(${b}.as_str(), ${c}.as_str())`;
        case "concat":
          return `format!("{}{}", ${a}, ${b})`;
        case "pad":
          return `format!("{:>6}", ${a})`;
        case "first-char":
          return `${a}.chars().next().unwrap_or(${b})`;
        case "from-digit":
          return `char::from_digit(${a} % 10, 10).unwrap_or('?')`;
        case "char-count":
          return `(${a}.chars().count() as ${e.ty})`;
        case "split-count":
          return `(${a}.split(',').count() as ${e.ty})`;
        case "char-code":
          return `(${a} as u32)`;
        case "str-contains":
          return `${a}.contains(${b}.as_str())`;
        case "starts_with":
          return `${a}.starts_with(${b}.as_str())`;
        case "matches-b":
          return `matches!(${a}, E::B(_))`;
        case "contains_key":
          return `${a}.contains_key(&${b})`;
        case "map-len":
          return `(${a}.len() as ${e.ty})`;
        case "map-get":
          return `${a}.get(&${b}).copied().unwrap_or(${c})`;
        case "map-sum":
          return `${a}.values().sum::<i32>()`;
        case "contains":
          return `${a}.contains(&${b})`;
        case "is_empty":
        case "is_some":
        case "is_none":
          return `${a}.${e.form}()`;
        case "len":
          return `${a}.len()`;
        case "sum":
          return `${a}.iter().sum::<${e.ty}>()`;
        case "first":
        case "last":
          return `${a}.${e.form}().copied()`;
        case "max":
        case "min":
          return `${a}.iter().copied().${e.form}()`;
        // Their items are `e`, which no generated variable is, so a closure's
        // `x` isn't hidden by it.
        case "map":
          return `${a}.iter().map(|e| e.wrapping_add(${b})).collect::<${e.ty}>()`;
        case "filter":
          return `${a}.into_iter().filter(|e| *e > ${b}).collect::<${e.ty}>()`;
        case "map-option":
          return `${a}.map(|e| e.wrapping_mul(${b}))`;
        case "unwrap_or":
          return `${a}.unwrap_or(${b})`;
      }
      throw new Error(`no form ${e.form}`);
    }
  }
}

function lines(stmts: Stmt[], indent: string): string[] {
  const nested = (body: Stmt[]) => lines(body, indent + "    ");
  return stmts.flatMap((s): string[] => {
    switch (s.kind) {
      case "let":
        return [`${indent}let mut ${s.name}: ${s.ty} = ${show(s.value)};`];
      case "assign":
        return [`${indent}${s.name} ${s.op} ${show(s.value)};`];
      case "print":
        return [`${indent}println!("${s.format}", ${show(s.value)});`];
      case "if":
        return [`${indent}if ${show(s.c)} {`, ...nested(s.then), `${indent}} else {`, ...nested(s.else), `${indent}}`];
      case "for":
        return [`${indent}for ${s.name} in 0..id(${s.n}u32) {`, ...nested(s.body), `${indent}}`];
      case "for-each":
        return [`${indent}for ${s.name} in ${s.items}.clone() {`, ...nested(s.body), `${indent}}`];
      case "if-let":
        return [`${indent}if let Some(${s.name}) = ${show(s.value)} {`, ...nested(s.then), `${indent}} else {`, ...nested(s.else), `${indent}}`];
      case "vec-op":
        return [`${indent}${s.name}.${s.op}(${s.value ? show(s.value) : ""});`];
      case "set": {
        const at = s.note === undefined ? `id(${s.index}usize)` : `note(${s.note}, id(${s.index}usize))`;
        const index = s.write ? `{ ${s.write.name} = ${show(s.write.value)}; ${at} }` : at;
        return [`${indent}${s.name}${s.field !== undefined ? `.${s.field}` : `[${index}]`} ${s.op} ${show(s.value)};`];
      }
      case "closure":
        return [`${indent}let ${s.name} = move |x: ${s.param}| -> ${s.param} { ${show(s.body)} };`];
      case "call-mut": {
        const call = s.twice ? `(${s.name}() ^ ${s.name}())` : `${s.name}()`;
        return [
          `${indent}{`,
          `${indent}    let mut ${s.name} = || -> ${s.ty} {`,
          `${indent}        ${s.captured} = ${show(s.body)};`,
          `${indent}        ${show(s.result)}`,
          `${indent}    };`,
          `${indent}    ${s.target} ${s.op} ${call};`,
          `${indent}}`,
        ];
      }
      case "text-op":
        return [`${indent}${s.name}.${s.op}(${s.op === "push_str" ? `${show(s.value)}.as_str()` : show(s.value)});`];
      case "map-op":
        if (s.op === "remove") return [`${indent}${s.name}.remove(&${show(s.key)});`];
        if (s.op === "insert") return [`${indent}${s.name}.insert(${show(s.key)}, ${show(s.value!)});`];
        return [`${indent}*${s.name}.entry(${show(s.key)}).or_insert(0) += ${show(s.value!)};`];
      case "for-map":
        return [`${indent}for (${s.key}, ${s.item}) in ${s.map}.clone() {`, ...nested(s.body), `${indent}}`];
      case "if-let-b":
        return [`${indent}if let E::B(${s.name}) = ${show(s.value)} {`, ...nested(s.then), `${indent}} else {`, ...nested(s.else), `${indent}}`];
      case "fn": {
        const [p, q] = s.params;
        const out = s.result === "Option" ? (v: string) => `Some(${v})` : (v: string) => `Ok(${v})`;
        const none = s.result === "Option" ? "None" : 'Err(String::from("early"))';
        const body = s.steps.map((step) => {
          if (step.op === "early") return `${indent}    if ${show(step.c)} {\n${indent}        return ${step.value ? out(show(step.value)) : none};\n${indent}    }`;
          const checked = `${show(step.a)}.checked_${step.how}(${show(step.b)})`;
          const tried = s.result === "Option" ? `${checked}?` : `${checked}.ok_or(format!("${step.how} {} {}", ${p}, ${q}))?`;
          return `${indent}    let ${step.name} = ${tried};`;
        });
        const result = s.result === "Option" ? `Option<${s.ty}>` : `Result<${s.ty}, String>`;
        return [`${indent}fn ${s.name}(${p}: ${s.ty}, ${q}: ${s.ty}) -> ${result} {`, ...body, `${indent}    ${out(show(s.tail))}`, `${indent}}`];
      }
      case "let-pair":
        return [
          s.from === "Pair"
            ? `${indent}let (mut ${s.x}, mut ${s.y}) = ${show(s.value)};`
            : `${indent}let S { a: mut ${s.x}, c: mut ${s.y}, .. } = ${show(s.value)};`,
        ];
      case "assign-two":
        return [`${indent}(${s.x}, ${s.y}) = (${show(s.a)}, ${show(s.b)});`];
      case "slice-pat":
        return [`${indent}if let &[${s.x}, .., ${s.y}] = ${s.name}.as_slice() {`, ...nested(s.then), `${indent}} else {`, ...nested(s.else), `${indent}}`];
      case "shared-write":
        return [`${indent}*${s.name}.borrow_mut() ${s.op} ${show(s.value)};`];
      case "vec-sort": {
        const k = `id(${s.k}${s.elem})`;
        if (s.how === "key") return [`${indent}${s.name}.sort_by_key(|x| *x % ${k});`];
        if (s.how === "by") return [`${indent}${s.name}.sort_by(|a, b| (*a % ${k}).cmp(&(*b % ${k})).then(b.cmp(a)));`];
        if (s.how === "dedup") return [`${indent}${s.name}.dedup_by_key(|x| *x % ${k});`];
        return [`${indent}${s.name}.retain(|&${s.x}| ${show(s.keep!)});`];
      }
      case "shape-push":
        return [`${indent}${s.name}.push(Box::new(${s.sq ? `Sq(${show(s.a)})` : `Rect(${show(s.a)}, ${show(s.b)})`}));`];
      case "shape-grow":
        return [`${indent}for s in ${s.name}.iter_mut() {`, `${indent}    s.grow(${show(s.by)});`, `${indent}}`];
      case "while":
        return [
          `${indent}let mut ${s.counter}: u32 = 0;`,
          `${indent}while ${s.counter} < id(${s.n}u32) && ${show(s.c)} {`,
          `${indent}    ${s.counter} += 1;`,
          ...nested(s.body),
          `${indent}}`,
        ];
      case "labeled":
        return [
          `${indent}'${s.label}: for ${s.outer} in 0..id(${s.n}u32) {`,
          `${indent}    for ${s.inner} in 0..id(${s.m}u32) {`,
          `${indent}        if ${show(s.brk)} {`,
          `${indent}            break '${s.label};`,
          `${indent}        }`,
          `${indent}        if ${show(s.cont)} {`,
          `${indent}            continue '${s.label};`,
          `${indent}        }`,
          ...lines(s.body, indent + "        "),
          `${indent}    }`,
          `${indent}}`,
        ];
      case "dlet":
        return [s.bound ? `${indent}let mut ${s.name} = D(id(${s.tag}u32));` : `${indent}let _ = D(id(${s.tag}u32));`];
      case "dset":
        return [`${indent}${s.name} = D(id(${s.tag}u32));`];
      case "dmove":
        if (s.how === "drop") return [`${indent}drop(${s.name});`];
        if (s.how === "take") return [`${indent}println!("took {}", take(${s.name}));`];
        return [`${indent}let mut ${s.into} = ${s.name};`];
    }
  });
}

/** The program as Rust: its statements in `main`, `id`, which hides a
 * literal's value from rustc's checks, and the struct `S`. */
export function print(program: Program, seed?: number): string {
  return [
    ...(seed === undefined ? [] : [`// Generated from seed ${seed}.`]),
    "fn id<T>(x: T) -> T {",
    "    x",
    "}",
    "",
    "fn note<T>(tag: u32, x: T) -> T {",
    '    println!("note {tag}");',
    "    x",
    "}",
    "",
    "fn bump(s: &mut S) -> i32 {",
    "    s.a = s.a.wrapping_add(1);",
    "    s.a",
    "}",
    "",
    "impl S {",
    "    fn get(&self) -> i32 {",
    "        self.a",
    "    }",
    "",
    "    fn add(&mut self, n: i32) -> i32 {",
    "        self.a = self.a.wrapping_add(n);",
    "        self.a",
    "    }",
    "",
    "    fn scaled(self, k: i32) -> S {",
    "        S { a: self.a.wrapping_mul(k), b: self.b, c: self.c }",
    "    }",
    "}",
    "",
    "trait Shape {",
    "    fn area(&self) -> i64;",
    "",
    "    fn name(&self) -> String {",
    '        String::from("shape")',
    "    }",
    "",
    "    fn grow(&mut self, by: i64);",
    "}",
    "",
    "struct Sq(i64);",
    "",
    "struct Rect(i64, i64);",
    "",
    "impl Shape for Sq {",
    "    fn area(&self) -> i64 {",
    "        self.0.wrapping_mul(self.0)",
    "    }",
    "",
    "    fn grow(&mut self, by: i64) {",
    "        self.0 = self.0.wrapping_add(by);",
    "    }",
    "}",
    "",
    "impl Shape for Rect {",
    "    fn area(&self) -> i64 {",
    "        self.0.wrapping_mul(self.1)",
    "    }",
    "",
    "    fn name(&self) -> String {",
    '        format!("rect {}x{}", self.0, self.1)',
    "    }",
    "",
    "    fn grow(&mut self, by: i64) {",
    "        self.0 = self.0.wrapping_add(by);",
    "        self.1 = self.1.wrapping_sub(by);",
    "    }",
    "}",
    "",
    "fn total<T: Shape>(items: &[T]) -> i64 {",
    "    items.iter().map(|s| s.area()).fold(0, |a, b| a.wrapping_add(b))",
    "}",
    "",
    "fn describe(s: &dyn Shape) -> String {",
    '    format!("{} {}", s.name(), s.area())',
    "}",
    "",
    "type Shapes = Vec<Box<dyn Shape>>;",
    "",
    "use std::cell::RefCell;",
    "use std::rc::Rc;",
    "",
    "type Shared = Rc<RefCell<i64>>;",
    "",
    "type Pair = (i32, u8);",
    "",
    "struct D(u32);",
    "",
    "impl Drop for D {",
    "    fn drop(&mut self) {",
    '        println!("drop {}", self.0);',
    "    }",
    "}",
    "",
    "fn take(d: D) -> u32 {",
    "    d.0",
    "}",
    "",
    "use std::collections::BTreeMap;",
    "",
    "type Map = BTreeMap<u8, i32>;",
    "",
    "#[derive(Debug, Clone, Copy, PartialEq)]",
    "enum E {",
    "    A,",
    "    B(i32),",
    "    C { x: u8, y: bool },",
    "}",
    "",
    "#[derive(Debug, Clone, Copy, PartialEq)]",
    `struct S {`,
    ...struct.fields.map(([name, ty]) => `    ${name}: ${ty},`),
    "}",
    "",
    "#[allow(unused, arithmetic_overflow, unconditional_panic)]",
    "fn main() {",
    ...lines(program, "    "),
    "}",
    "",
  ].join("\n");
}

// Smaller programs, each one change from `program`: a statement taken away,
// an `if` or a loop made what's inside it, an expression made one of its
// parts or a literal. Some are no longer valid Rust, which the caller finds.
function* smaller(program: Program): Generator<Program> {
  function* inBlock(stmts: Stmt[], rebuild: (next: Stmt[]) => Program): Generator<Program> {
    for (let i = 0; i < stmts.length; i++) {
      const s = stmts[i];
      const put = (replacement: Stmt[]) => rebuild([...stmts.slice(0, i), ...replacement, ...stmts.slice(i + 1)]);
      yield put([]);
      if (s.kind === "if" || s.kind === "if-let" || s.kind === "if-let-b" || s.kind === "slice-pat") {
        yield put(s.then);
        yield put(s.else);
        yield* inBlock(s.then, (next) => put([{ ...s, then: next }]));
        yield* inBlock(s.else, (next) => put([{ ...s, else: next }]));
      } else if (s.kind === "for" || s.kind === "for-each" || s.kind === "for-map" || s.kind === "while" || s.kind === "labeled") {
        yield put(s.body);
        yield* inBlock(s.body, (next) => put([{ ...s, body: next }]));
      }
      if (s.kind === "set" && s.write) yield put([{ ...s, write: undefined }]);
      if (s.kind === "set" && s.note !== undefined) yield put([{ ...s, note: undefined }]);
      if (s.kind === "set" && s.op !== "=") yield put([{ ...s, op: "=" }]);
      if ("value" in s && s.value) for (const value of simpler(s.value)) yield put([{ ...s, value } as Stmt]);
      if (s.kind === "closure") for (const body of simpler(s.body)) yield put([{ ...s, body }]);
      if (s.kind === "call-mut") {
        if (s.twice) yield put([{ ...s, twice: false }]);
        for (const body of simpler(s.body)) yield put([{ ...s, body }]);
        for (const result of simpler(s.result)) yield put([{ ...s, result }]);
      }
    }
  }
  yield* inBlock(program, (next) => next);
}

// Simpler expressions of the same type: its parts that have it, and a
// literal; then each part made simpler.
function* simpler(e: Expr): Generator<Expr> {
  if (e.kind === "lit" || e.kind === "var" || e.kind === "zero") return;
  const parts = Object.entries(e).filter(([, v]) => v && typeof v === "object" && "kind" in v) as [string, Expr][];
  for (const [, p] of parts) if (p.ty === e.ty) yield p;
  yield e.ty === "bool" ? { kind: "lit", ty: e.ty, value: false } : isInt(e.ty) ? { kind: "lit", ty: e.ty, value: 0n } : { kind: "zero", ty: e.ty };
  if (e.kind === "vec") {
    for (let i = 0; i < e.items.length; i++) yield { ...e, items: [...e.items.slice(0, i), ...e.items.slice(i + 1)] };
  }
  for (const [key, p] of parts) for (const q of simpler(p)) yield { ...e, [key]: q } as Expr;
}

/** `program` made as small as it still `fails`: each change that keeps it
 * failing is kept, until none does, or until `until`, a time, has passed,
 * when it's the smallest so far and not `complete`. A change is kept only
 * if it makes the program shorter, so there's an end to them, and each one
 * kept is given to `kept`, so what's found so far is never lost. */
export async function reduce(
  program: Program,
  fails: (p: Program) => Promise<boolean>,
  { until = Infinity, kept = () => {} }: { until?: number; kept?: (p: Program) => void } = {},
): Promise<{ program: Program; complete: boolean }> {
  let current = program;
  let changed = true;
  while (changed) {
    changed = false;
    const length = print(current).length;
    for (const candidate of smaller(current)) {
      if (Date.now() > until) return { program: current, complete: false };
      if (print(candidate).length < length && (await fails(candidate))) {
        current = candidate;
        kept(current);
        changed = true;
        break;
      }
    }
  }
  return { program: current, complete: true };
}

/** How many statements a program has, at every depth. */
export function size(program: Program): number {
  return program.reduce((n, s) => {
    const inner =
      s.kind === "if" || s.kind === "if-let" || s.kind === "if-let-b" || s.kind === "slice-pat"
        ? size(s.then) + size(s.else)
        : s.kind === "for" || s.kind === "for-each" || s.kind === "for-map" || s.kind === "while" || s.kind === "labeled"
          ? size(s.body)
          : 0;
    return n + 1 + inner;
  }, 0);
}
