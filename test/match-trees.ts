// Generated `match`es whose tree MIR tests through labeled blocks (ADR
// 0364): or-patterns of tuples, slices and options, bindings each
// alternative gives, guards that call a function that counts its calls,
// the arms' values given, assigned, returned or jumped from in a loop; and
// #523's C1 shapes, a guarded `|` whose binding the guard reads, a guard
// made again of each alternative, a guard of a call's `is_some()`, an
// arm's own `match` whose arms give the outer's value. Each program is
// `per` such functions and a `main` calling each over its inputs, which
// prints what each guard was called with, so a chain that calls a guard
// another number of times, or in another order, prints otherwise.
// `test/match-trees.test.ts` runs them against native Rust (#771).

/** The generator's numbers, a seed's alone. */
class Rng {
  constructor(private s: number) {}
  next() {
    this.s = (this.s * 1103515245 + 12345) & 0x7fffffff;
    return this.s / 0x80000000;
  }
  int(n: number) {
    return Math.floor(this.next() * n);
  }
  pick<T>(xs: T[]): T {
    return xs[this.int(xs.length)];
  }
  chance(p: number) {
    return this.next() < p;
  }
}

type Fn = { def: string; call: string };

type Shape = { kind: "tuple"; arity: number } | { kind: "slice" } | { kind: "option" };

function literal(r: Rng) {
  return r.pick(["0", "1", "2", "3", "_", "_", "1..=2"]);
}

/** An alternative of a tuple pattern binding `names`, each at a place of its own. */
function tupleAlt(r: Rng, arity: number, names: string[]) {
  const places = [...Array(arity).keys()];
  const parts = places.map(() => literal(r));
  const free = [...places];
  for (const n of names) {
    const at = free.splice(r.int(free.length), 1)[0];
    parts[at] = n;
  }
  // Most alternatives test something: a literal where nothing is bound.
  if (free.length > 0 && free.every((at) => parts[at] === "_") && r.chance(0.9)) {
    parts[r.pick(free)] = String(r.int(4));
  }
  return `(${parts.join(", ")})`;
}

function sliceAlt(r: Rng, names: string[]) {
  if (names.length === 0) return r.pick(["[]", "[1, ..]", "[.., 1]", "[2, 2]", "[_, 3, ..]", "[0]"]);
  if (names.length === 1) {
    const [x] = names;
    return r.pick([`[${x}, ..]`, `[.., ${x}]`, `[1, ${x}, ..]`, `[${x}]`, `[_, ${x}]`, `[2, .., ${x}]`]);
  }
  const [x, y] = names;
  return r.pick([`[${x}, ${y}, ..]`, `[${x}, .., ${y}]`, `[1, ${x}, ${y}]`, `[${y}, ${x}]`]);
}

function optionAlt(r: Rng, names: string[]) {
  if (names.length === 0) return r.pick(["None", "Some(0)", "Some(1..=2)", "Some(_)"]);
  return `Some(${names[0]})`;
}

function value(r: Rng, names: string[], scrut: string[]): string {
  const atoms = [...names, ...names, ...scrut, String(r.int(4))];
  const a = r.pick(atoms);
  switch (r.int(4)) {
    case 0:
      return a;
    case 1:
      return `${a} + ${r.pick(atoms)}`;
    case 2:
      return `${a} * 2`;
    default:
      return String(r.int(5));
  }
}

function guard(r: Rng, names: string[], scrut: string[], k: number): string {
  const one = (): string => {
    const v = value(r, names, scrut);
    switch (r.int(5)) {
      case 0:
      case 1:
        return `g(c, ${v}, ${k})`;
      case 2:
        return `!g(c, ${v}, ${k})`;
      case 3:
        return `${v} > ${r.int(4)}`;
      default:
        return `${v} % 2 == 0`;
    }
  };
  switch (r.int(4)) {
    case 0:
      return `${one()} && ${one()}`;
    case 1:
      return `${one()} || ${one()}`;
    default:
      return one();
  }
}

/** A `match` of a tuple, a slice or an option, its arms or-patterns of
 * bindings, most guarded. */
function shapedMatch(r: Rng, i: number): Fn {
  const shape: Shape = r.pick<Shape>([
    { kind: "tuple", arity: 2 },
    { kind: "tuple", arity: 2 },
    { kind: "tuple", arity: 3 },
    { kind: "slice" },
    { kind: "option" },
  ]);
  const scrut = shape.kind === "tuple" ? ["a", "b", "d"].slice(0, shape.arity) : [];
  const arms: string[] = [];
  const n = 2 + r.int(3);
  const form = r.pick(["value", "value", "assign", "return", "loop"]);
  let k = 0;
  for (let j = 0; j < n; j++) {
    const nameCount =
      shape.kind === "option" ? r.int(2) : shape.kind === "tuple" && shape.arity === 2 ? r.int(2) : r.int(3);
    const names = ["x", "y"].slice(0, nameCount);
    const alts = 1 + r.int(shape.kind === "option" ? 2 : 3);
    const pats: string[] = [];
    for (let t = 0; t < alts; t++) {
      if (shape.kind === "tuple") pats.push(tupleAlt(r, shape.arity, names));
      else if (shape.kind === "slice") pats.push(`&${sliceAlt(r, names)}`);
      else pats.push(optionAlt(r, names));
    }
    const g = r.chance(0.7) ? ` if ${guard(r, names, scrut, k++)}` : "";
    const v = value(r, names, scrut);
    let body: string;
    switch (form) {
      case "assign":
        body = r.chance(0.3) ? `{}` : `level = ${v}`;
        break;
      case "return":
        body = r.chance(0.4) ? `return ${v} + 1000` : `level = ${v}`;
        break;
      case "loop":
        body = r.pick([`continue`, `{ level += ${v}; }`, `break`, `{}`]);
        break;
      default:
        body = v;
    }
    arms.push(`        ${pats.join(" | ")}${g} => ${body},`);
  }
  const otherwise = form === "value" ? "99" : form === "loop" ? "{ level += 1; }" : "level = 99";
  arms.push(`        _ => ${otherwise},`);
  const params =
    shape.kind === "tuple" ? scrut.map((s) => `${s}: u32`).join(", ") : shape.kind === "slice" ? "s: &[u32]" : "o: Option<u32>";
  const subject = shape.kind === "tuple" ? `(${scrut.join(", ")})` : shape.kind === "slice" ? "s" : "o";
  let def: string;
  if (form === "value") {
    def = `fn m${i}(c: &Cell<u32>, ${params}) -> u32 {\n    match ${subject} {\n${arms.join("\n")}\n    }\n}\n`;
  } else if (form === "loop") {
    def = `fn m${i}(c: &Cell<u32>, ${params}) -> u32 {\n    let mut level = 0;\n    for _ in 0..2 {\n        level += 1;\n        match ${subject} {\n${arms.map((a) => "    " + a).join("\n")}\n        }\n        level += 10;\n    }\n    level\n}\n`;
  } else {
    def = `fn m${i}(c: &Cell<u32>, ${params}) -> u32 {\n    let mut level = 7;\n    match ${subject} {\n${arms.join("\n")}\n    }\n    level\n}\n`;
  }
  let call: string;
  if (shape.kind === "tuple") {
    const loops = scrut.map((s) => `for ${s} in 0..4u32 {`).join(" ");
    call = `    ${loops} println!("m${i} {}", m${i}(c, ${scrut.join(", ")})); ${scrut.map(() => "}").join("")}`;
  } else if (shape.kind === "slice") {
    call = `    for s in SLICES { println!("m${i} {}", m${i}(c, s)); }`;
  } else {
    call = `    for o in [None, Some(0), Some(1), Some(2), Some(3)] { println!("m${i} {}", m${i}(c, o)); }`;
  }
  return { def, call };
}

/** A guard of what's bound, `x`, or of none, of C1's shapes. */
function boundGuard(r: Rng, bound: string | undefined, k: number): string {
  const v = bound && r.chance(0.8) ? bound : r.pick(["a + b", "a", "b", "a * 2"]);
  const call = r.pick([`g(c, ${v}, ${k})`, `g(c, ${v}, ${k})`, `h(c, ${v}).is_some()`, `!g(c, ${v}, ${k})`, `h(c, ${v}).is_none()`]);
  switch (r.int(5)) {
    case 0:
      return `${call} && ${bound ?? "a"} > ${r.int(3)}`;
    case 1:
      return `${bound ?? "b"} % 2 == 0 && ${call}`;
    case 2:
      return `${call} || ${bound ?? "a"} == ${r.int(3)}`;
    default:
      return call;
  }
}

function alternatives(r: Rng, bound: string | undefined): string {
  const n = 2 + r.int(2);
  const alts: string[] = [];
  for (let i = 0; i < n; i++) {
    const at = r.int(2);
    const other = r.chance(0.8) ? String(r.int(4)) : "_";
    const here = bound ?? (r.chance(0.5) ? "_" : String(r.int(4)));
    alts.push(at === 0 ? `(${other}, ${here})` : `(${here}, ${other})`);
  }
  return alts.join(" | ");
}

/** A `match` of a pair whose guarded `|`s bind what their guards read. */
function boundMatch(r: Rng, i: number): Fn {
  const form = r.pick(["value", "assign", "loop", "return"]);
  const arms: string[] = [];
  let k = 0;
  const n = 2 + r.int(3);
  for (let j = 0; j < n; j++) {
    const bound = r.chance(0.6) ? "x" : undefined;
    const guarded = r.chance(0.8);
    const pat = r.chance(0.8) ? alternatives(r, bound) : `(${r.int(4)}, ${bound ?? r.int(4)})`;
    const g = guarded ? ` if ${boundGuard(r, bound, k++)}` : "";
    const v = bound ? r.pick([`${bound} * 10`, `${bound} + a`, bound]) : r.pick(["a + b", String(r.int(9)), "b * 3"]);
    let body: string;
    switch (form) {
      case "value":
        body = v;
        break;
      case "assign":
        body = r.chance(0.2) ? "{}" : `level = ${v}`;
        break;
      case "return":
        body = r.chance(0.4) ? `return ${v} + 1000` : `level = ${v}`;
        break;
      default:
        body = r.pick(["continue", "break", "{}", `out.push(${v})`, `{ out.push(${v}); continue; }`]);
    }
    arms.push(`        ${pat}${g} => ${body},`);
  }
  arms.push(`        _ => ${form === "value" ? "99" : form === "loop" ? r.pick(["{}", "continue", "break"]) : "level = 99"},`);
  let def: string;
  if (form === "value") {
    def = `fn m${i}(c: &Cell<u32>, a: u32, b: u32) -> u32 {\n    match (a, b) {\n${arms.join("\n")}\n    }\n}\n`;
  } else if (form === "loop") {
    def = `fn m${i}(c: &Cell<u32>, a: u32, b: u32) -> u32 {\n    let mut out = Vec::new();\n    for t in 0..3u32 {\n        let (a, b) = (a + t % 2, b);\n        match (a, b) {\n${arms.map((x) => "    " + x).join("\n")}\n        }\n        out.push(a * 100 + t);\n    }\n    out.iter().sum::<u32>() + out.len() as u32\n}\n`;
  } else {
    def = `fn m${i}(c: &Cell<u32>, a: u32, b: u32) -> u32 {\n    let mut level = 7;\n    match (a, b) {\n${arms.join("\n")}\n    }\n    level\n}\n`;
  }
  const call = `    for a in 0..4u32 { for b in 0..4u32 { println!("m${i} {}", m${i}(c, a, b)); } }`;
  return { def, call };
}

/** An arm's own `match`, its arms the outer's value or a return. */
function innerMatch(r: Rng, i: number): Fn {
  const arms: string[] = [];
  let k = 0;
  const n = 2 + r.int(4);
  for (let j = 0; j < n; j++) {
    const op = String(r.int(4));
    const bound = r.chance(0.6) ? "x" : undefined;
    const pat = r.chance(0.7) ? `(${op}, ${bound ?? r.int(4)})` : `(${op}, ${bound ?? "_"}) | (${bound ?? "_"}, ${r.int(2) + 2})`;
    const g = r.chance(0.4) ? ` if ${boundGuard(r, bound, k++)}` : "";
    const ret = r.chance(0.3);
    const v = bound ? `${bound} * 2 + ${r.int(5)}` : String(r.int(50));
    arms.push(`                ${pat}${g} => ${ret ? `return Err(${v})` : v},`);
  }
  arms.push(`                (o, l) => return Err(o * 100 + l),`);
  const def = `fn m${i}(c: &Cell<u32>, e: Option<(u32, u32)>) -> Result<u32, u32> {
    Ok(match e {
        None => 5,
        Some((0, 0)) => 6,
        Some((op, v)) => {
            let l = step(c, v)?;
            let (a, b) = (op, l);
            match (op, l) {
${arms.join("\n")}
            }
        }
    })
}
`;
  const call = `    for op in 0..4u32 { for v in 0..4u32 { println!("m${i} {:?}", m${i}(c, Some((op, v)))); } } println!("m${i} {:?}", m${i}(c, None));`;
  return { def, call };
}

/** The seed's program: `per` functions, `m0` to its last, and its `main`. */
export function program(seed: number, per = 40): string {
  const r = new Rng(seed * 7919 + 17);
  const fns = Array.from({ length: per }, (_, i) =>
    r.pick([shapedMatch, shapedMatch, shapedMatch, boundMatch, boundMatch, innerMatch])(r, i),
  );
  return `use std::cell::Cell;

const SLICES: &[&[u32]] = &[&[], &[1], &[2], &[1, 1], &[2, 2], &[1, 2], &[2, 1], &[3, 1, 2], &[1, 3, 1], &[2, 2, 2], &[0, 1, 2, 3]];

fn g(c: &Cell<u32>, n: u32, k: u32) -> bool {
    let v = c.get();
    c.set(v + 1);
    println!("g{k} {n} {v}");
    (v * 7 + n) % 3 != 0
}

fn h(c: &Cell<u32>, n: u32) -> Option<u32> {
    let v = c.get();
    c.set(v + 2);
    println!("h {n} {v}");
    if (v + n) % 4 == 1 { None } else { Some(v) }
}

fn step(c: &Cell<u32>, v: u32) -> Result<u32, u32> {
    let n = c.get();
    c.set(n + 1);
    if (n + v) % 5 == 4 { Err(v) } else { Ok(v % 4) }
}

${fns.map((f) => f.def).join("\n")}
fn main() {
    let c = &Cell::new(0);
${fns.map((f) => f.call).join("\n")}
}
`;
}

/** The functions of `js`, the JS of a seed's program, that print a label
 * the compiler made, `bb12:`: the Rust writes none. */
export function labeled(js: string): string[] {
  return js
    .split(/\n(?=function )/)
    .filter((f) => /^\s*bb\d+: /m.test(f))
    .map((f) => f.slice("function ".length, f.indexOf("(")));
}
