// The memory net's programs (#1172): unsafe Rust over plain data, as
// hashbrown, smallvec, bytes and memchr write it, each seed's a program of
// a few integer arrays, `Vec`s and boxed slices changed and read through
// raw pointers: their arithmetic, `offset_from`, `transmute` between plain
// types, a `union`'s other field read, `MaybeUninit` slots, a slice made of
// a cast pointer, a typed array's bytes, `ptr::copy` and `write_bytes`, and
// reads and writes at any byte. Each family of them is in a seed's program
// by a coin, swarm testing's (Groce et al., ISSTA 2012), so a program of
// few families has more of each. Each operation is in a block of its own,
// its pointers made there of the place they point into, so none outlives a
// write to that place by another way, and each pointer stays within its
// allocation: no program is undefined behaviour, which Miri proves of each
// (test/memory-net.test.ts). Floats are made of finite numbers or of
// integers whose exponent isn't all ones, never a NaN, whose payload JS
// doesn't keep (the guide's differences from native Rust).

import { random, type Random } from "./generate";

/** The families of operations, each in a seed's program by a coin. */
export const families = ["arith", "offsets", "transmute", "unions", "uninit", "views", "copies", "unaligned", "helpers"] as const;
export type Family = (typeof families)[number];

const ints = ["u8", "i8", "u16", "i16", "u32", "i32", "u64", "i64"] as const;
type Int = (typeof ints)[number];
const sizeOf: Record<Int, number> = { u8: 1, i8: 1, u16: 2, i16: 2, u32: 4, i32: 4, u64: 8, i64: 8 };
const signed = (t: Int) => t.startsWith("i");
const bits = (t: Int) => sizeOf[t] * 8;

/** A place of items: an array, a `Vec` or a boxed slice, of `n` `ty`s. */
type Buffer = { name: string; ty: Int; n: number; kind: "array" | "vec" | "boxed" };

/** A number of `ty`, as Rust writes it: an edge or any of its values. */
function literal(r: Random, ty: Int): string {
  const width = BigInt(bits(ty));
  const edges = signed(ty) ? ["0", "1", "-1", `${ty}::MIN`, `${ty}::MAX`] : ["0", "1", `${ty}::MAX`];
  if (r.chance(0.3)) return r.pick(edges);
  let v = 0n;
  for (let i = 0n; i < width; i += 16n) v |= BigInt(r.int(0x10000)) << i;
  v &= (1n << width) - 1n;
  if (signed(ty) && v >= 1n << (width - 1n)) v -= 1n << width;
  // Small ones too, which show as themselves.
  if (r.chance(0.4)) v = BigInt(r.int(200)) - (signed(ty) ? 100n : 0n);
  const text = v < 0n ? `-${-v}` : `${v}`;
  return text;
}

/** The plain types of each size a value may be transmuted to or from: its
 * integers, arrays of narrower ones, `#[repr(C)]` structs, and a float,
 * never made a NaN. */
const plainOfSize: Record<number, string[]> = {
  1: ["u8", "i8"],
  2: ["u16", "i16", "[u8; 2]", "(u8, u8)"],
  4: ["u32", "i32", "f32", "[u8; 4]", "[u16; 2]", "[i8; 4]", "Half"],
  8: ["u64", "i64", "f64", "[u8; 8]", "[u16; 4]", "[u32; 2]", "(u32, u32)", "Mixed"],
};

/** A value of plain type `ty`, made of `read`, a number of the program's
 * state, as an `as` cast of it, or of literals: never a NaN of a float. */
function plainValue(r: Random, ty: string, read: () => string): string {
  const array = /^\[(\w+); (\d+)\]$/.exec(ty);
  if (array) return `[${Array.from({ length: Number(array[2]) }, () => plainValue(r, array[1], read)).join(", ")}]`;
  const tuple = /^\((\w+), (\w+)\)$/.exec(ty);
  if (tuple) return `(${plainValue(r, tuple[1], read)}, ${plainValue(r, tuple[2], read)})`;
  if (ty === "Half") return `Half { lo: ${plainValue(r, "u16", read)}, hi: ${plainValue(r, "u16", read)} }`;
  if (ty === "Mixed") return `Mixed { a: ${plainValue(r, "u32", read)}, b: ${plainValue(r, "u16", read)}, c: ${plainValue(r, "u8", read)}, d: ${plainValue(r, "i8", read)} }`;
  if (ty === "f32") return r.pick(["0.0", "-0.0", "1.5", "-2.25", "3.4028235e38", "1e-40", "f32::INFINITY", `(${read()} as f32)`]);
  if (ty === "f64") return r.pick(["0.0", "-0.0", "0.1", "-7.25", "1e300", "5e-324", "f64::NEG_INFINITY", `(${read()} as f64)`]);
  return r.chance(0.5) ? `(${read()} as ${ty})` : literal(r, ty as Int);
}

/** Whether a plain type holds a float, which an integer's bits made into
 * may be a NaN. */
const holdsFloat = (ty: string) => /f32|f64/.test(ty);

/** The integer of `size` bytes whose bits a float of that size is made of,
 * its exponent's top bit cleared, so it's no NaN nor infinity. */
function floatSafeBits(size: number, value: string): string {
  return size === 4 ? `(${value} & 0xBFFF_FFFF)` : `(${value} & 0xBFFF_FFFF_FFFF_FFFF)`;
}

/** A union of each size, `#[repr(C)]`, each of its fields as wide. */
const unions: Record<number, { name: string; fields: [string, string][] }> = {
  4: { name: "W4", fields: [["u", "u32"], ["i", "i32"], ["f", "f32"], ["b", "[u8; 4]"], ["h", "[u16; 2]"], ["p", "Half"]] },
  8: { name: "W8", fields: [["u", "u64"], ["i", "i64"], ["f", "f64"], ["b", "[u8; 8]"], ["w", "[u32; 2]"], ["m", "Mixed"]] },
};

/** What a program uses of the items it may declare. */
type Uses = { half: boolean; mixed: boolean; unions: Set<number>; helpers: Set<string>; mem: boolean; ptr: boolean; slice: boolean; uninit: boolean };

/** Seed `seed`'s program, the families it has, and each operation's form,
 * `forms[op - 1]` of the operation whose lines it prints start `op`. */
export function memoryProgram(seed: number): { text: string; swarm: Family[]; forms: string[] } {
  const r = random(seed);
  const swarm = families.filter(() => r.chance(0.5));
  if (swarm.length === 0) swarm.push(r.pick(families));
  const uses: Uses = { half: false, mixed: false, unions: new Set(), helpers: new Set(), mem: false, ptr: false, slice: false, uninit: false };
  const buffers: Buffer[] = [];
  const lines: string[] = [];
  const forms: string[] = [];
  const count = 2 + r.int(3);
  for (let i = 0; i < count; i++) {
    const ty = i === 0 && swarm.includes("unaligned") ? "u8" : r.pick(ints);
    const n = ty === "u8" && i === 0 ? 8 + r.int(9) : 2 + r.int(7);
    const kind = r.pick(["array", "array", "vec", "boxed"] as const);
    const b: Buffer = { name: `a${i}`, ty, n, kind };
    buffers.push(b);
    const items = Array.from({ length: n }, () => literal(r, ty)).join(", ");
    const made = kind === "array" ? `[${items}]` : kind === "vec" ? `vec![${items}]` : `vec![${items}].into_boxed_slice()`;
    const declared = kind === "array" ? `[${ty}; ${n}]` : kind === "vec" ? `Vec<${ty}>` : `Box<[${ty}]>`;
    lines.push(`    let mut ${b.name}: ${declared} = ${made};`);
  }
  // Unions held from the start, and which field each was written by whole
  // last: a float field is read only after a float was written to it.
  const held: Record<number, string | undefined> = {};
  const read = (b: Buffer = r.pick(buffers)) => `${b.name}[${r.int(b.n)}]`;
  const ops = 6 + r.int(14);
  for (let op = 1; op <= ops; op++) {
    const family = r.pick(swarm);
    const made = operation(r, family, buffers, uses, held, read, op);
    forms.push(made.form);
    lines.push(...made.body.map((line) => `    ${line}`));
    for (const shown of made.shown) lines.push(`    println!("${op} ${shown.replace(/[{}]/g, "")} {:?}", ${shown});`);
  }
  const head: string[] = [];
  const imports = [uses.mem && "mem", uses.ptr && "ptr", uses.slice && "slice"].filter(Boolean);
  if (uses.uninit) head.push("use std::mem::MaybeUninit;");
  if (imports.length > 0) head.push(`use std::{${imports.join(", ")}};`);
  if (uses.half) head.push("", "#[repr(C)]", "#[derive(Clone, Copy, Debug)]", "struct Half {", "    lo: u16,", "    hi: u16,", "}");
  if (uses.mixed) head.push("", "#[repr(C)]", "#[derive(Clone, Copy, Debug)]", "struct Mixed {", "    a: u32,", "    b: u16,", "    c: u8,", "    d: i8,", "}");
  for (const size of [...uses.unions].sort()) {
    const u = unions[size];
    head.push("", "#[repr(C)]", `union ${u.name} {`, ...u.fields.map(([f, t]) => `    ${f}: ${t},`), "}");
  }
  for (const helper of [...uses.helpers].sort()) head.push("", ...helperText[helper]);
  const text = [`// The memory net's seed ${seed} (#1172): ${swarm.join(", ")}.`, ...head, "", "fn main() {", ...lines, "}", ""].join("\n");
  return { text, swarm, forms };
}

/** The helpers an operation may call: generic code given raw pointers, and
 * a cursor that holds two. */
const helperText: Record<string, string[]> = {
  fill: [
    "unsafe fn fill<T: Copy>(p: *mut T, n: usize, v: T) {",
    "    for i in 0..n {",
    "        unsafe { p.add(i).write(v) };",
    "    }",
    "}",
  ],
  each: [
    "unsafe fn each<T: Copy, U>(p: *const T, n: usize, f: impl Fn(T) -> U) -> Vec<U> {",
    "    (0..n).map(|i| f(unsafe { *p.add(i) })).collect()",
    "}",
  ],
  cursor: [
    "struct Cursor<T> {",
    "    at: *mut T,",
    "    end: *mut T,",
    "}",
    "",
    "impl<T: Copy> Cursor<T> {",
    "    fn over(items: &mut [T]) -> Self {",
    "        let range = items.as_mut_ptr_range();",
    "        Cursor { at: range.start, end: range.end }",
    "    }",
    "",
    "    unsafe fn next_with(&mut self, f: impl Fn(T) -> T) -> Option<T> {",
    "        if self.at == self.end {",
    "            return None;",
    "        }",
    "        let old = unsafe { *self.at };",
    "        unsafe {",
    "            *self.at = f(old);",
    "            self.at = self.at.add(1);",
    "        }",
    "        Some(old)",
    "    }",
    "",
    "    fn left(&self) -> usize {",
    "        unsafe { self.end.offset_from(self.at) as usize }",
    "    }",
    "}",
  ],
};

/** An operation: its statements, what it shows after, and its form,
 * `<family>/<name>`, which names a wrong answer it first shows. */
type Made = { body: string[]; shown: string[]; form: string };

/** One of `forms` of `family`, by a coin, its form named by its key: a
 * form that makes another where it can't be made says which. */
function oneOf(r: Random, family: Family, forms: Record<string, () => Omit<Made, "form"> & { form?: string }>): Made {
  const [name, make] = r.pick(Object.entries(forms));
  return { form: `${family}/${name}`, ...make() };
}

/** An operation of `family`: its statements, and what it shows after. */
function operation(
  r: Random,
  family: Family,
  buffers: Buffer[],
  uses: Uses,
  held: Record<number, string | undefined>,
  read: (b?: Buffer) => string,
  op: number,
): Made {
  const b = r.pick(buffers);
  const i = r.int(b.n);
  const j = r.int(b.n);
  const v = () => literal(r, b.ty);
  switch (family) {
    case "arith": {
      const lo = Math.min(i, j);
      const hi = Math.max(i, j);
      return oneOf(r, family, {
        "write-at": () => ({ body: [`unsafe {`, `    let p = ${b.name}.as_mut_ptr();`, `    *p.add(${i}) = ${v()};`, `}`], shown: [b.name] }),
        "write": () => ({ body: [`unsafe { ${b.name}.as_mut_ptr().add(${i}).write(${v()}) };`], shown: [b.name] }),
        "add-of-another": () => ({
          body: [`unsafe {`, `    let p = ${b.name}.as_mut_ptr();`, `    *p.add(${i}) = (*p.add(${j})).wrapping_add(${v()});`, `}`],
          shown: [b.name],
        }),
        "neg-from-its-end": () => ({
          body: [`unsafe {`, `    let end = ${b.name}.as_mut_ptr().add(${hi});`, `    *end.sub(${hi - lo}) = (*end).wrapping_neg();`, `}`],
          shown: [b.name],
        }),
        "swap": () => {
          uses.ptr = true;
          return { body: [`unsafe {`, `    let p = ${b.name}.as_mut_ptr();`, `    ptr::swap(p.add(${i}), p.add(${j}));`, `}`], shown: [b.name] };
        },
        "replace": () => {
          uses.ptr = true;
          return { body: [`let old${op} = unsafe { ptr::replace(${b.name}.as_mut_ptr().add(${i}), ${v()}) };`], shown: [`old${op}`, b.name] };
        },
        "read-at": () => ({ body: [`let x${op} = unsafe { *${b.name}.as_ptr().add(${i}) };`], shown: [`x${op}`] }),
        "wrapping-read": () => ({ body: [`let x${op} = unsafe { ${b.name}.as_ptr().wrapping_add(${i}).read() };`], shown: [`x${op}`] }),
        "offset-read": () => ({ body: [`let x${op} = unsafe { *${b.name}.as_ptr().offset(${i} as isize) };`], shown: [`x${op}`] }),
        "step-each": () => ({
          body: [
            `unsafe {`,
            `    let mut q = ${b.name}.as_mut_ptr().add(${lo});`,
            `    for _ in ${lo}..${hi + 1} {`,
            `        *q = (*q).wrapping_mul(3).wrapping_add(1);`,
            `        q = q.add(1);`,
            `    }`,
            `}`,
          ],
          shown: [b.name],
        }),
        "compare": () => ({
          body: [`let p${op} = ${b.name}.as_ptr();`, `let x${op} = (p${op}.wrapping_add(${i}) < p${op}.wrapping_add(${j}), p${op}.wrapping_add(${i}) == p${op}.wrapping_add(${j}));`],
          shown: [`x${op}`],
        }),
        "read-back": () => {
          const back = r.int(i + 1);
          return { body: [`let x${op} = unsafe { *${b.name}.as_ptr().add(${i}).sub(${back}) };`], shown: [`x${op}`] };
        },
        "walk-to-the-end": () => ({
          body: [
            `let mut x${op} = Vec::new();`,
            `let mut q${op} = ${b.name}.as_ptr();`,
            `let end${op} = unsafe { q${op}.add(${b.name}.len()) };`,
            `while q${op} != end${op} {`,
            `    x${op}.push(unsafe { *q${op} });`,
            `    q${op} = unsafe { q${op}.add(1) };`,
            `}`,
          ],
          shown: [`x${op}`],
        }),
      });
    }
    case "offsets": {
      uses.mem = true;
      return oneOf(r, family, {
        "offset-from": () => ({ body: [`let x${op} = unsafe { ${b.name}.as_ptr().add(${i}).offset_from(${b.name}.as_ptr().add(${j})) };`], shown: [`x${op}`] }),
        "address-difference": () => ({ body: [`let x${op} = ${b.name}.as_ptr().wrapping_add(${i}) as usize - ${b.name}.as_ptr() as usize;`], shown: [`x${op}`] }),
        "byte-offset-from": () => ({ body: [`let x${op} = unsafe { ${b.name}.as_ptr().add(${i}).byte_offset_from(${b.name}.as_ptr()) };`], shown: [`x${op}`] }),
        "ptr-range": () => ({
          body: [`let r${op} = ${b.name}.as_ptr_range();`, `let x${op} = unsafe { r${op}.end.offset_from(r${op}.start) };`],
          shown: [`x${op}`],
        }),
        "offset-from-in-bytes": () => ({
          body: [`let x${op} = unsafe { (${b.name}.as_ptr().add(${i}) as *const u8).offset_from(${b.name}.as_ptr() as *const u8) };`],
          shown: [`x${op}`],
        }),
        "size-and-align": () => ({ body: [`let x${op} = (mem::size_of_val(&${b.name}[..]), mem::align_of_val(&${b.name}[0]));`], shown: [`x${op}`] }),
        "offset-from-stepped-in-bytes": () => ({
          body: [`let x${op} = unsafe { ${b.name}.as_ptr().cast::<u8>().add(${i * sizeOf[b.ty]}).cast::<${b.ty}>().offset_from(${b.name}.as_ptr()) };`],
          shown: [`x${op}`],
        }),
      });
    }
    case "transmute": {
      uses.mem = true;
      const size = r.pick([1, 2, 4, 8]);
      const from = r.pick(plainOfSize[size]);
      const into = r.pick(plainOfSize[size].filter((t) => t !== from));
      noteTypes(uses, from, into);
      let value = plainValue(r, from, () => read());
      // Into a float, of an integer's bits, made no NaN.
      if (holdsFloat(into)) {
        const int = size === 4 ? "u32" : "u64";
        value = `mem::transmute::<${int}, ${from}>(${floatSafeBits(size, `mem::transmute::<${from}, ${int}>(${value})`)})`;
      }
      // Of the whole of a buffer of the size, now and then: its items' bytes.
      const whole = buffers.filter((x) => x.kind === "array" && x.n * sizeOf[x.ty] <= 8 && !holdsFloat(into));
      if (whole.length > 0 && r.chance(0.3)) {
        const w = r.pick(whole);
        const bytes = w.n * sizeOf[w.ty];
        const target = r.pick(plainOfSize[bytes] ?? [`[u8; ${bytes}]`]).replace(/^f(32|64)$/, "u$1");
        noteTypes(uses, target, target);
        return { body: [`let x${op} = unsafe { mem::transmute::<[${w.ty}; ${w.n}], ${target}>(${w.name}) };`], shown: [`x${op}`], form: "transmute/whole-array" };
      }
      return { body: [`let x${op} = unsafe { mem::transmute::<${from}, ${into}>(${value}) };`], shown: [`x${op}`], form: "transmute/value" };
    }
    case "unions": {
      const size = r.pick([4, 8]);
      const u = unions[size];
      uses.unions.add(size);
      noteTypes(uses, ...u.fields.map(([, t]) => t));
      const name = `w${size}`;
      const body: string[] = [];
      if (!(size in held)) {
        body.push(`let mut ${name} = ${u.name} { u: ${literal(r, size === 4 ? "u32" : "u64")} };`);
        held[size] = "u";
      }
      const [field, ty] = r.pick(u.fields);
      if (r.chance(0.6)) {
        let value = plainValue(r, ty, () => read());
        if (holdsFloat(ty) && /\(.* as f\d\d\)/.test(value)) value = ty === "f32" ? "2.5" : "-0.5";
        if (r.chance(0.3) && /^\[/.test(ty)) {
          const item = /^\[(\w+); (\d+)\]$/.exec(ty)!;
          body.push(`unsafe { ${name}.${field}[${r.int(Number(item[2]))}] = ${literal(r, item[1] as Int)} };`);
          held[size] = "part";
        } else {
          body.push(`${name}.${field} = ${value};`);
          held[size] = field;
        }
      }
      const readable = u.fields.filter(([f, t]) => !holdsFloat(t) || held[size] === f);
      const [other] = r.pick(readable);
      body.push(`let x${op} = unsafe { ${name}.${other} };`);
      return { body, shown: [`x${op}`], form: "unions/other-field" };
    }
    case "uninit": {
      uses.uninit = true;
      const n = b.n;
      const values = () => Array.from({ length: n }, (_, k) => r.chance(0.5) ? literal(r, b.ty) : `${b.name}[${k}].wrapping_add(1)`);
      return oneOf(r, family, {
        "slots-transmuted": () => {
          uses.mem = true;
          const vs = values();
          return {
            body: [
              `let mut m${op} = [MaybeUninit::<${b.ty}>::uninit(); ${n}];`,
              ...vs.map((x, k) => `m${op}[${k}].write(${x});`),
              `let x${op} = unsafe { mem::transmute::<[MaybeUninit<${b.ty}>; ${n}], [${b.ty}; ${n}]>(m${op}) };`,
            ],
            shown: [`x${op}`],
          };
        },
        "slots-mapped": () => {
          const vs = values();
          return {
            body: [
              `let mut m${op} = [MaybeUninit::<${b.ty}>::uninit(); ${n}];`,
              `for (k, slot) in m${op}.iter_mut().enumerate() {`,
              `    slot.write([${vs.join(", ")}][k]);`,
              `}`,
              `let x${op} = m${op}.map(|slot| unsafe { slot.assume_init() });`,
            ],
            shown: [`x${op}`],
          };
        },
        "slots-through-their-pointers": () => {
          uses.ptr = true;
          const k = 1 + r.int(n);
          return {
            body: [
              `let mut m${op} = [MaybeUninit::<${b.ty}>::uninit(); ${n}];`,
              `for k in 0..${k} {`,
              `    unsafe { m${op}[k].as_mut_ptr().write(${b.name}[k].wrapping_mul(5)) };`,
              `}`,
              `let x${op}: Vec<${b.ty}> = (0..${k}).map(|k| unsafe { m${op}[k].assume_init_read() }).collect();`,
            ],
            shown: [`x${op}`],
          };
        },
        "array-through-an-item-pointer": () => {
          const vs = values();
          return {
            body: [
              `let mut m${op} = MaybeUninit::<[${b.ty}; ${n}]>::uninit();`,
              `let p${op} = m${op}.as_mut_ptr() as *mut ${b.ty};`,
              ...vs.map((x, k) => `unsafe { p${op}.add(${k}).write(${x}) };`),
              `let x${op} = unsafe { m${op}.assume_init() };`,
            ],
            shown: [`x${op}`],
          };
        },
        "slots-read-as-an-array": () => {
          uses.ptr = true;
          const vs = values();
          return {
            body: [
              `let mut m${op} = [MaybeUninit::<${b.ty}>::uninit(); ${n}];`,
              ...vs.map((x, k) => `m${op}[${k}] = MaybeUninit::new(${x});`),
              `let x${op} = unsafe { ptr::read(m${op}.as_ptr() as *const [${b.ty}; ${n}]) };`,
            ],
            shown: [`x${op}`],
          };
        },
        "zeroed": () => ({ body: [`let x${op} = unsafe { MaybeUninit::<[${b.ty}; ${n}]>::zeroed().assume_init() };`], shown: [`x${op}`] }),
      });
    }
    case "views": {
      uses.slice = true;
      uses.mem = true;
      const narrower = ints.filter((t) => sizeOf[t] <= sizeOf[b.ty]);
      const into = r.pick(narrower);
      const total = b.n * sizeOf[b.ty];
      const len = total / sizeOf[into];
      return oneOf(r, family, {
        "bytes": () => ({
          body: [`let x${op} = unsafe { slice::from_raw_parts(${b.name}.as_ptr() as *const u8, mem::size_of_val(&${b.name}[..])) };`],
          shown: [`x${op}`],
        }),
        "narrower": () => ({ body: [`let x${op} = unsafe { slice::from_raw_parts(${b.name}.as_ptr().cast::<${into}>(), ${len}) };`], shown: [`x${op}`] }),
        "mutable-narrower": () => {
          const k = r.int(len);
          return {
            body: [
              `let view${op} = unsafe { slice::from_raw_parts_mut(${b.name}.as_mut_ptr().cast::<${into}>(), ${len}) };`,
              `view${op}[${k}] = ${literal(r, into)};`,
              ...(r.chance(0.5) ? [`view${op}.reverse();`] : []),
            ],
            shown: [b.name],
          };
        },
        "sub-slice": () => {
          const from = r.int(b.n);
          const n = r.int(b.n - from + 1);
          return {
            body: [`let x${op}: Vec<${b.ty}> = unsafe { slice::from_raw_parts(${b.name}.as_ptr().add(${from}), ${n}) }.iter().map(|x| x.wrapping_add(1)).collect();`],
            shown: [`x${op}`],
          };
        },
        "byte-read": () => {
          const k = r.int(total);
          return { body: [`let x${op} = unsafe { ${b.name}.as_ptr().cast::<u8>().add(${k}).read() };`], shown: [`x${op}`] };
        },
        "byte-write": () => {
          const k = r.int(total);
          return { body: [`unsafe { *${b.name}.as_mut_ptr().cast::<u8>().add(${k}) = ${literal(r, "u8")} };`], shown: [b.name] };
        },
        "bytes-compared": () => ({
          body: [
            `let bytes${op} = unsafe { slice::from_raw_parts(${b.name}.as_ptr().cast::<u8>(), ${total}) };`,
            `let x${op} = ${b.name}.iter().flat_map(|x| x.to_ne_bytes()).collect::<Vec<u8>>() == bytes${op};`,
          ],
          shown: [`x${op}`],
        }),
      });
    }
    case "copies": {
      uses.ptr = true;
      const same = buffers.filter((x) => x !== b && x.ty === b.ty);
      return oneOf(r, family, {
        "copy": () => {
          const n = r.int(b.n - Math.max(i, j) + 1);
          return { body: [`unsafe {`, `    let p = ${b.name}.as_mut_ptr();`, `    ptr::copy(p.add(${i}), p.add(${j}), ${n});`, `}`], shown: [b.name] };
        },
        "write-bytes": () => {
          const n = r.int(b.n - i + 1);
          return { body: [`unsafe { ptr::write_bytes(${b.name}.as_mut_ptr().add(${i}), ${literal(r, "u8")}, ${n}) };`], shown: [b.name] };
        },
        "copy-bytes-between": () => {
          const o = r.pick(buffers.filter((x) => x !== b));
          if (o === undefined) return { body: [`unsafe { ${b.name}.as_mut_ptr().add(${i}).write_bytes(0, 1) };`], shown: [b.name], form: "copies/write-bytes" };
          const from = r.int(b.n * sizeOf[b.ty]);
          const to = r.int(o.n * sizeOf[o.ty]);
          const n = r.int(Math.min(b.n * sizeOf[b.ty] - from, o.n * sizeOf[o.ty] - to) + 1);
          return {
            body: [`unsafe { ptr::copy_nonoverlapping(${b.name}.as_ptr().cast::<u8>().add(${from}), ${o.name}.as_mut_ptr().cast::<u8>().add(${to}), ${n}) };`],
            shown: [o.name],
          };
        },
        "copy-from-nonoverlapping": () => {
          const o = r.pick(same);
          if (o === undefined) return { body: [`unsafe { ${b.name}.as_mut_ptr().add(${i}).copy_from(${b.name}.as_ptr().add(${j}), 1) };`], shown: [b.name], form: "copies/copy-from" };
          // Where in `o`, within its own length.
          const k = r.int(o.n);
          const n = r.int(Math.min(b.n - i, o.n - k) + 1);
          return {
            body: [`unsafe { ${o.name}.as_mut_ptr().add(${k}).copy_from_nonoverlapping(${b.name}.as_ptr().add(${i}), ${n}) };`],
            shown: [o.name],
          };
        },
        "swap-nonoverlapping": () => {
          const o = r.pick(same);
          if (o === undefined) return { body: [`unsafe { ptr::write(${b.name}.as_mut_ptr().add(${i}), ${v()}) };`], shown: [b.name], form: "copies/write" };
          const k = r.int(o.n);
          const n = r.int(Math.min(b.n - i, o.n - k) + 1);
          return {
            body: [`unsafe { ptr::swap_nonoverlapping(${b.name}.as_mut_ptr().add(${i}), ${o.name}.as_mut_ptr().add(${k}), ${n}) };`],
            shown: [b.name, o.name],
          };
        },
      });
    }
    case "unaligned": {
      uses.ptr = true;
      const total = b.n * sizeOf[b.ty];
      const fits = ints.filter((t) => sizeOf[t] <= total);
      const ty = r.pick(fits);
      const at = r.int(total - sizeOf[ty] + 1);
      return oneOf(r, family, {
        "read-unaligned": () => ({ body: [`let x${op} = unsafe { ${b.name}.as_ptr().cast::<u8>().add(${at}).cast::<${ty}>().read_unaligned() };`], shown: [`x${op}`] }),
        "write-unaligned": () => ({
          body: [`unsafe { ${b.name}.as_mut_ptr().cast::<u8>().add(${at}).cast::<${ty}>().write_unaligned(${literal(r, ty)}) };`],
          shown: [b.name],
        }),
        "ptr-read-unaligned": () => ({ body: [`let x${op} = unsafe { ptr::read_unaligned(${b.name}.as_ptr().cast::<u8>().add(${at}) as *const ${ty}) };`], shown: [`x${op}`] }),
      });
    }
    case "helpers": {
      return oneOf(r, family, {
        "fill": () => {
          uses.helpers.add("fill");
          const n = r.int(b.n - i + 1);
          return { body: [`unsafe { fill(${b.name}.as_mut_ptr().add(${i}), ${n}, ${v()}) };`], shown: [b.name] };
        },
        "each": () => {
          uses.helpers.add("each");
          return {
            body: [`let x${op} = unsafe { each(${b.name}.as_ptr(), ${b.name}.len(), |x| x.count_ones() + ${r.int(4)}) };`],
            shown: [`x${op}`],
          };
        },
        "cursor": () => {
          uses.helpers.add("cursor");
          const steps = r.int(b.n + 2);
          return {
            body: [
              `let mut c${op} = Cursor::over(&mut ${b.name}[..]);`,
              `let mut x${op} = Vec::new();`,
              `for _ in 0..${steps} {`,
              `    x${op}.push(unsafe { c${op}.next_with(|x| x.wrapping_sub(${literal(r, b.ty)})) });`,
              `}`,
              `let left${op} = c${op}.left();`,
            ],
            shown: [`x${op}`, `left${op}`, b.name],
          };
        },
      });
    }
  }
}

/** Note the structs a plain type names, which the program declares. */
function noteTypes(uses: Uses, ...types: string[]) {
  for (const ty of types) {
    if (ty.includes("Half")) uses.half = true;
    if (ty.includes("Mixed")) uses.mixed = true;
  }
}
