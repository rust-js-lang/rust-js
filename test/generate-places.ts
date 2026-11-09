// The generator's places (ADR 0092, amended, #1085): a `Pl`, a `Copy`
// struct of an `i32`, a tuple `(u8, i64)`, an array `[i16; 3]` and an enum
// `Va`, each of its integer places read and written as a variable is, as
// MIR's projections reach them, and the statements that lend, copy, match
// and show it, as Rustlantis generates places.

import type { Expr, Random, Scope, Stmt, Ty } from "./generate";
import type { Counter } from "./generate-swarm";

/** A `Va`, the enum a `Pl` holds: `X`, `Y(u16)` or `Z { w: i32 }`. */
export type Variant = { v: "X" } | { v: "Y"; n: Expr } | { v: "Z"; w: Expr };

export type PlaceStmt =
  // A `Pl`, a `Copy` aggregate of nested places, as MIR's projections reach
  // them (#1085): `p.a`, `p.t.1`, `p.arr[2]`, `p.e`; made, and each read
  // and written through its places by the statements after it.
  | { kind: "plet"; name: string; a: Expr; t: [Expr, Expr]; arr: [Expr, Expr, Expr]; e: Variant }
  // An item lent `&mut` and written through, `{ let r = &mut p.arr[1]; *r = .. }`,
  // or lent to `poke`, which writes it and gives it.
  | { kind: "pref"; name: string; k: number; value: Expr; call: boolean }
  // A copy, `let mut q = p;`, one of its places written, both shown: a
  // `Copy` value is a copy, which a write to one doesn't change.
  | { kind: "pcopy"; name: string; into: string; value: Expr }
  // The variant `p.e` holds set, or matched by `ref mut` and written in place.
  | { kind: "pvariant"; name: string; e: Variant }
  | { kind: "pmatch"; name: string; n: Expr; w: Expr }
  // `p.arr.swap(i, j)`, `mem::replace(&mut p.t.1, v)` shown, `p = Pl { a, ..p }`.
  | { kind: "pswap"; name: string; i: number; j: number }
  | { kind: "preplace"; name: string; value: Expr }
  | { kind: "pupdate"; name: string; value: Expr }
  // A place given back by a function, `*pick(&mut p, k) = v`; a copy given
  // to one that writes its own, `summed(p)`; an array and a `Vec` of copies,
  // one written; a `move` closure's copy of a place, which a write after
  // doesn't change; a variant tested by `&mut`; a pattern's `ref mut` of
  // a place, which writes the `Pl` itself.
  | { kind: "ppick"; name: string; k: number; value: Expr }
  | { kind: "ptotal"; name: string }
  | { kind: "parray"; name: string; k: number; value: Expr }
  | { kind: "pvec"; name: string; k: number; value: Expr; w: Expr }
  | { kind: "pclosure"; name: string; value: Expr }
  | { kind: "piflet"; name: string; n: Expr }
  | { kind: "ppattern"; name: string }
  // What a block made of them, shown as it ends: each's state, as
  // Rustlantis dumps its locals.
  | { kind: "pdump"; name: string };

const placeKinds = new Set(["plet", "pref", "pcopy", "pvariant", "pmatch", "pswap", "preplace", "pupdate", "ppick", "ptotal", "parray", "pvec", "pclosure", "piflet", "ppattern", "pdump"]);

export const isPlace = (s: Stmt): s is PlaceStmt => placeKinds.has(s.kind);

type Make = (r: Random, ty: Ty, scope: Scope, depth: number) => Expr;

/** A statement of a `Pl`: one made, where none is or by a coin, its places
 * added to `inner` and its name to `places`, which its block shows as it
 * ends; else one of each way the statements read and write one. */
export function placeStatement(r: Random, inner: Scope, places: string[], counter: Counter, expr: Make): PlaceStmt {
  const pls = inner.filter((v) => v.ty === "Pl");
  const variant = (scope: Scope): Variant => {
    const v = r.pick(["X", "Y", "Z"] as const);
    if (v === "X") return { v };
    return v === "Y" ? { v, n: expr(r, "u16", scope, 1) } : { v, w: expr(r, "i32", scope, 1) };
  };
  // A `Pl` made here, its places what follows may read and write.
  const made = (name: string) => {
    inner.push(...placesOf(name));
    places.push(name);
  };
  if (pls.length === 0 || r.chance(0.25)) {
    const name = `p${counter.n++}`;
    const plet: PlaceStmt = {
      kind: "plet",
      name,
      a: expr(r, "i32", inner, 2),
      t: [expr(r, "u8", inner, 2), expr(r, "i64", inner, 2)],
      arr: [expr(r, "i16", inner, 1), expr(r, "i16", inner, 1), expr(r, "i16", inner, 1)],
      e: variant(inner),
    };
    made(name);
    return plet;
  }
  const name = r.pick(pls).name;
  // What it's written of reads none of the `Pl` it writes, whose places
  // borrowck holds borrowed.
  const free = inner.filter((v) => v.name !== name && !v.name.startsWith(`${name}.`));
  switch (r.int(15)) {
    case 0:
      return { kind: "pref", name, k: r.int(3), value: expr(r, "i16", free, 2), call: false };
    case 1:
      return { kind: "pref", name, k: r.int(3), value: expr(r, "i16", free, 2), call: true };
    case 2: {
      const into = `p${counter.n++}`;
      const copy: PlaceStmt = { kind: "pcopy", name, into, value: expr(r, "i32", free, 2) };
      made(into);
      return copy;
    }
    case 3:
      return { kind: "pvariant", name, e: variant(free) };
    case 4:
      return { kind: "pmatch", name, n: expr(r, "u16", free, 1), w: expr(r, "i32", free, 1) };
    case 5:
      return { kind: "pswap", name, i: r.int(3), j: r.int(3) };
    case 6:
      return { kind: "preplace", name, value: expr(r, "i64", free, 2) };
    case 7:
      return { kind: "pupdate", name, value: expr(r, "i32", free, 2) };
    case 8:
      return { kind: "ppick", name, k: r.int(3), value: expr(r, "i16", free, 2) };
    case 9:
      return { kind: "ptotal", name };
    case 10:
      return { kind: "parray", name, k: r.int(3), value: expr(r, "i16", free, 2) };
    case 11:
      return { kind: "pvec", name, k: r.int(3), value: expr(r, "i16", free, 2), w: expr(r, "i32", free, 2) };
    case 12:
      return { kind: "pclosure", name, value: expr(r, "i32", free, 2) };
    case 13:
      return { kind: "piflet", name, n: expr(r, "u16", free, 1) };
    default:
      return { kind: "ppattern", name };
  }
}

/** A `Pl` named `name`, and each of its places an integer is: what an
 * expression reads and a statement writes, `p.t.1` as a variable is. */
function placesOf(name: string): Scope {
  return [
    { name, ty: "Pl", mutable: false },
    { name: `${name}.a`, ty: "i32", mutable: true },
    { name: `${name}.t.0`, ty: "u8", mutable: true },
    { name: `${name}.t.1`, ty: "i64", mutable: true },
    ...[0, 1, 2].map((k) => ({ name: `${name}.arr[${k}]`, ty: "i16", mutable: true })),
  ];
}

function showVariant(e: Variant, show: (e: Expr) => string): string {
  if (e.v === "X") return "Va::X";
  return e.v === "Y" ? `Va::Y(${show(e.n)})` : `Va::Z { w: ${show(e.w)} }`;
}

/** A `Pl`'s statement's lines, `show` each expression's text. */
export function placeLines(s: PlaceStmt, indent: string, show: (e: Expr) => string): string[] {
  switch (s.kind) {
    case "plet":
      return [
        `${indent}let mut ${s.name} = Pl { a: ${show(s.a)}, t: (${show(s.t[0])}, ${show(s.t[1])}), arr: [${s.arr.map(show).join(", ")}], e: ${showVariant(s.e, show)} };`,
      ];
    case "pref":
      return s.call
          ? [`${indent}println!("{}", poke(&mut ${s.name}.arr[${s.k}], ${show(s.value)}));`]
          : [`${indent}{`, `${indent}    let r = &mut ${s.name}.arr[${s.k}];`, `${indent}    *r = r.wrapping_add(${show(s.value)});`, `${indent}}`];
    case "pcopy":
      return [
        `${indent}let mut ${s.into} = ${s.name};`,
        `${indent}${s.into}.a = ${show(s.value)};`,
        `${indent}println!("{} {:?}", ${s.name} == ${s.into}, ${s.name});`,
      ];
    case "pvariant":
      return [`${indent}${s.name}.e = ${showVariant(s.e, show)};`];
    case "pmatch":
      return [
        `${indent}match ${s.name}.e {`,
        `${indent}    Va::X => println!("x"),`,
        `${indent}    Va::Y(ref mut n) => *n = n.wrapping_add(${show(s.n)}),`,
        `${indent}    Va::Z { ref mut w } => *w ^= ${show(s.w)},`,
        `${indent}}`,
      ];
    case "pswap":
      return [`${indent}${s.name}.arr.swap(${s.i}, ${s.j});`];
    case "preplace":
      return [`${indent}println!("{}", std::mem::replace(&mut ${s.name}.t.1, ${show(s.value)}));`];
    case "pupdate":
      return [`${indent}${s.name} = Pl { a: ${show(s.value)}, ..${s.name} };`];
    case "pdump":
      return [`${indent}println!("{:?}", ${s.name});`];
    case "ppick":
      return [`${indent}*pick(&mut ${s.name}, ${s.k}) ^= ${show(s.value)};`];
    case "ptotal":
      return [`${indent}println!("{} {:?}", summed(${s.name}), ${s.name});`];
    case "parray":
      return [
        `${indent}{`,
        `${indent}    let mut pa = [${s.name}; 2];`,
        `${indent}    pa[1].arr[${s.k}] = ${show(s.value)};`,
        `${indent}    println!("{} {:?}", pa[0] == pa[1], pa[0]);`,
        `${indent}}`,
      ];
    case "pvec":
      return [
        `${indent}{`,
        `${indent}    let mut pv = vec![${s.name}, ${s.name}];`,
        `${indent}    pv[0].arr[${s.k}] = ${show(s.value)};`,
        `${indent}    let q = pv[1];`,
        `${indent}    pv[1].a = ${show(s.w)};`,
        `${indent}    println!("{:?} {:?}", q, pv);`,
        `${indent}}`,
      ];
    case "pclosure":
      return [
        `${indent}{`,
        `${indent}    let snap = move || ${s.name}.a;`,
        `${indent}    ${s.name}.a = ${show(s.value)};`,
        `${indent}    println!("{} {}", snap(), ${s.name}.a);`,
        `${indent}}`,
      ];
    case "piflet":
      return [
        `${indent}if let Va::Y(n) = &mut ${s.name}.e {`,
        `${indent}    *n = n.wrapping_add(${show(s.n)});`,
        `${indent}} else {`,
        `${indent}    ${s.name}.e = Va::Y(${show(s.n)});`,
        `${indent}}`,
      ];
    case "ppattern":
      return [
        `${indent}{`,
        `${indent}    let Pl { a, t: (ref mut x, _), .. } = ${s.name};`,
        `${indent}    *x = x.wrapping_add(1);`,
        `${indent}    println!("{} {}", a, x);`,
        `${indent}}`,
      ];
  }
}
