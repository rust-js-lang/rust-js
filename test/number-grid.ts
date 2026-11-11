// The numbers grid (#940): every integer and float operation of each type,
// of its boundary values, a case. Integer arithmetic in each of its forms,
// plain, `wrapping_`, `checked_`, `saturating_` and `overflowing_`; `/`,
// `%`, their `_euclid`s; shifts and rotations; `pow`; negation and `abs`;
// the bitwise operators; bit counts, `ilog`s and `isqrt`; `as` between
// every pair of the 14 number types, of a value and of a literal a `let`
// binds, which rustc gives as a constant cast, `let cast =
// 18446744073709551615_u64 as u32;` (#1380); `parse`, `from_str_radix` and
// `to_string`. Float arithmetic, `%`, the `_euclid`s, rounding, `sqrt`,
// `powi`, `mul_add`, `to_bits` and the class of a float. The values are each
// type's 0, ±1, 2, MIN, MIN+1, MAX-1 and MAX, and those about 2^31, 2^32,
// 2^53 and 2^63 that it holds; of a float, ±0, ±inf, NaN, its subnormals,
// `MIN_POSITIVE`, `EPSILON`, ties and the integers about 2^24 and 2^53.
// Each case prints what it makes by `{:?}`, or the panic it makes, its
// message. test/iterator-matrix.test.ts runs it (`ITERATOR_TABLE=numbers`)
// in both builds against native Rust, as it runs the iterator tables.
//
// Overflow wraps, as a release build's does (the guide's semantics), and
// native Rust's run is built so, `-Coverflow-checks=off`: what panics in
// either is what panics in a release build, a division by zero, `MIN /
// -1`, an `ilog` of zero. `isize` and `usize` are wasm32's, 32 bits (ADR
// 0090): a case writes them `Isz` and `Usz`, `isize` and `usize` where its
// pointers are 32 bits, rust-js's, and `i32` and `u32` where they aren't,
// native Rust's, so the oracle's are exactly as wide as rust-js's. Each
// value is given through `black_box`, so rustc folds none of them.

import type { Row } from "./iterator-matrix";

/** A number type: its name in a case, whether it's signed, and its bits;
 * a float's are its own. */
type Num = { ty: string; signed: boolean; bits: number; float?: boolean };

const ints: Num[] = [
  { ty: "i8", signed: true, bits: 8 },
  { ty: "i16", signed: true, bits: 16 },
  { ty: "i32", signed: true, bits: 32 },
  { ty: "i64", signed: true, bits: 64 },
  { ty: "i128", signed: true, bits: 128 },
  { ty: "Isz", signed: true, bits: 32 },
  { ty: "u8", signed: false, bits: 8 },
  { ty: "u16", signed: false, bits: 16 },
  { ty: "u32", signed: false, bits: 32 },
  { ty: "u64", signed: false, bits: 64 },
  { ty: "u128", signed: false, bits: 128 },
  { ty: "Usz", signed: false, bits: 32 },
];
const floats: Num[] = [
  { ty: "f32", signed: true, bits: 32, float: true },
  { ty: "f64", signed: true, bits: 64, float: true },
];
export const nums = [...ints, ...floats];

/** An integer type's boundary values, as expressions of it. */
function intValues(n: Num): string[] {
  const { ty, signed, bits } = n;
  const out = ["0", "1", "2", `${ty}::MAX`, `${ty}::MAX - 1`, `${ty}::MIN`];
  if (signed) out.push("-1", `${ty}::MIN + 1`, "-2");
  // About each power of two it holds past its own bounds' neighbours.
  for (const p of [31, 32, 53, 63]) {
    if (p >= bits - (signed ? 1 : 0)) continue;
    out.push(`(1 << ${p}) - 1`, `1 << ${p}`, `(1 << ${p}) + 1`);
    if (signed) out.push(`-(1 << ${p})`, `-(1 << ${p}) - 1`);
  }
  return out.map((v) => `${v}`);
}

/** A float type's values, as expressions of it. */
function floatValues(n: Num): string[] {
  const { ty } = n;
  const subnormal = ty === "f32" ? "1e-45" : "5e-324";
  const exact = ty === "f32" ? "16777216.0" : "9007199254740992.0";
  return [
    "0.0",
    "-0.0",
    "1.0",
    "-1.0",
    "0.1",
    "0.5",
    "1.5",
    "2.5",
    "-2.5",
    "1e10",
    "3.0e9",
    "-3.0e9",
    "1e20",
    "1e40",
    exact,
    `${exact} + 1.0`,
    `${exact} + 2.0`,
    `${ty}::MAX`,
    `${ty}::MIN`,
    `${ty}::MIN_POSITIVE`,
    `${ty}::EPSILON`,
    subnormal,
    `${ty}::INFINITY`,
    `${ty}::NEG_INFINITY`,
    `${ty}::NAN`,
  ].filter((v) => !(ty === "f32" && v === "1e40"));
}

const valuesOf = (n: Num) => (n.float ? floatValues(n) : intValues(n));

/** An operation: its name, the types it applies to, the second operands
 * it's given, `b`'s, of their type, `by`, and its text of `a` and `b`. */
type Op = { name: string; of: (n: Num) => boolean; b?: (n: Num) => string[]; by?: (n: Num) => string; text: (a: string, b: string, n: Num) => string };

const isInt = (n: Num) => !n.float;
const isSigned = (n: Num) => isInt(n) && n.signed;
const isUnsigned = (n: Num) => isInt(n) && !n.signed;
const isFloat = (n: Num) => n.float === true;

/** The second operands of an integer arithmetic operation. */
const intB = (n: Num) => ["0", "1", "2", "3", `${n.ty}::MAX`, `${n.ty}::MIN`, ...(n.signed ? ["-1", "-2"] : [])];
/** A shift's or a `pow`'s, a `u32`. */
const amounts = (n: Num) => [...new Set(["0", "1", "2", "3", "7", String(n.bits - 1), String(n.bits), String(n.bits + 1), "31", "32", "63", "64", "127", "128"])];
const floatB = (n: Num) => ["0.0", "-0.0", "1.0", "-1.0", "2.0", "0.5", "3.0", "-3.5", `${n.ty}::INFINITY`, `${n.ty}::NAN`, `${n.ty}::MAX`, `${n.ty}::MIN_POSITIVE`];

const u32 = () => "u32";

/** `method(b)` of each of `prefixes`, `wrapping_add` of `wrapping_` and
 * `add`. */
function named(base: string, prefixes: string[], of: (n: Num) => boolean, b: (n: Num) => string[] = intB, by?: (n: Num) => string): Op[] {
  return prefixes.map((p) => {
    const m = `${p}${base}`;
    return { name: m, of, b, by, text: (a: string, b: string) => `${a}.${m}(${b})` };
  });
}

const plainOp = (op: string, of: (n: Num) => boolean, b: (n: Num) => string[], by?: (n: Num) => string): Op => ({ name: `a ${op} b`, of, b, by, text: (a, b) => `${a} ${op} ${b}` });
const unary = (name: string, of: (n: Num) => boolean, text: (a: string) => string): Op => ({ name, of, text: (a) => text(a) });
const method = (m: string, of: (n: Num) => boolean) => unary(m, of, (a) => `${a}.${m}()`);

const arithmetic = ["wrapping_", "checked_", "saturating_", "overflowing_"];

export const ops: Op[] = [
  // Integers.
  plainOp("+", isInt, intB),
  ...named("add", arithmetic, isInt),
  plainOp("-", isInt, intB),
  ...named("sub", arithmetic, isInt),
  plainOp("*", isInt, intB),
  ...named("mul", arithmetic, isInt),
  plainOp("/", isInt, intB),
  ...named("div", arithmetic, isInt),
  ...named("div_euclid", ["", "wrapping_", "checked_", "overflowing_"], isInt),
  plainOp("%", isInt, intB),
  ...named("rem", ["wrapping_", "checked_", "overflowing_"], isInt),
  ...named("rem_euclid", ["", "wrapping_", "checked_", "overflowing_"], isInt),
  ...named("pow", ["", ...arithmetic], isInt, amounts, u32),
  plainOp("<<", isInt, amounts, u32),
  plainOp(">>", isInt, amounts, u32),
  ...named("shl", ["wrapping_", "checked_", "overflowing_"], isInt, amounts, u32),
  ...named("shr", ["wrapping_", "checked_", "overflowing_"], isInt, amounts, u32),
  ...named("rotate_left", [""], isInt, amounts, u32),
  ...named("rotate_right", [""], isInt, amounts, u32),
  plainOp("&", isInt, intB),
  plainOp("|", isInt, intB),
  plainOp("^", isInt, intB),
  ...named("abs_diff", [""], isInt),
  unary("!a", isInt, (a) => `!${a}`),
  unary("-a", isSigned, (a) => `-${a}`),
  ...["wrapping_neg", "checked_neg", "overflowing_neg"].map((m) => method(m, isInt)),
  ...["abs", "wrapping_abs", "checked_abs", "overflowing_abs", "saturating_abs", "saturating_neg", "unsigned_abs", "signum", "is_positive", "is_negative"].map((m) =>
    method(m, isSigned),
  ),
  ...["count_ones", "count_zeros", "leading_zeros", "trailing_zeros", "leading_ones", "trailing_ones", "swap_bytes", "reverse_bits"].map((m) => method(m, isInt)),
  ...["is_power_of_two", "next_power_of_two", "checked_next_power_of_two"].map((m) => method(m, isUnsigned)),
  ...["ilog2", "checked_ilog2", "ilog10", "checked_ilog10", "isqrt"].map((m) => method(m, isInt)),
  unary("to_string", isInt, (a) => `${a}.to_string()`),
  // Floats.
  plainOp("+", isFloat, floatB),
  plainOp("-", isFloat, floatB),
  plainOp("*", isFloat, floatB),
  plainOp("/", isFloat, floatB),
  plainOp("%", isFloat, floatB),
  ...named("div_euclid", [""], isFloat, floatB),
  ...named("rem_euclid", [""], isFloat, floatB),
  ...named("copysign", [""], isFloat, floatB),
  { name: "mul_add", of: isFloat, b: floatB, text: (a, b) => `${a}.mul_add(${b}, 1.0)` },
  { name: "powi", of: isFloat, b: () => ["0", "1", "2", "3", "-1", "-2", "31", "-31"], by: () => "i32", text: (a, b) => `${a}.powi(${b})` },
  unary("-a", isFloat, (a) => `-${a}`),
  ...["abs", "floor", "ceil", "round", "round_ties_even", "trunc", "fract", "sqrt", "signum", "recip", "to_bits", "is_nan", "is_finite", "is_infinite", "is_normal", "is_subnormal", "is_sign_negative", "is_sign_positive", "classify"].map(
    (m) => method(m, isFloat),
  ),
  unary("to_string", isFloat, (a) => `${a}.to_string()`),
];

/** The strings each type parses, and those `from_str_radix` does. */
const parsed = (n: Num): string[] =>
  n.float
    ? ["0", "-0", "1.5", "+1.5", ".5", "5.", "1e3", "1E-3", "-1e400", "1e-400", "inf", "-inf", "infinity", "NaN", "nan", "1_0", " 1", "", "-", "0x10", "3.4028236e38", "9007199254740993", "2.5e-324"]
    : ["0", "1", "-1", "+1", "-0", "007", "", "-", "+", " 1", "1 ", "1_0", "1.0", "0x10", "255", "256", "-128", "-129", "65536", "4294967295", "4294967296", "-2147483649", "18446744073709551616", "170141183460469231731687303715884105728", "999x"];
const radixed = ["0", "1", "-1", "+1", "7f", "FF", "-80", "zz", "Zz", "", "-", "10000000000000000000000000000000000000000", "1_0", "z1"];

export type NumberCase = Row & { group: string; ty: string; text: string; tys: string[] };

const cast = (from: Num, to: Num) => `${from.ty} as ${to.ty}`;

/** An integer type's boundary value, `intValues`' expression, as the typed
 * literal it is, `(-128_i8)`, exact: of what a type alias names, `Isz`
 * and `Usz`, none, as no literal's suffix is one; they're `i32`'s and
 * `u32`'s width, whose literals are. */
function literal(n: Num, expr: string): string | undefined {
  if (n.float || n.ty === "Isz" || n.ty === "Usz") return undefined;
  const top = n.signed ? (1n << BigInt(n.bits - 1)) - 1n : (1n << BigInt(n.bits)) - 1n;
  const bottom = n.signed ? -(1n << BigInt(n.bits - 1)) : 0n;
  const js = expr
    .replaceAll(`${n.ty}::MAX`, `(${top})`)
    .replaceAll(`${n.ty}::MIN`, `(${bottom})`)
    .replace(/\b(\d+)\b/g, "$1n");
  const value: bigint = new Function(`return ${js};`)();
  return value < 0n ? `(-${-value}_${n.ty})` : `${value}_${n.ty}`;
}

/** Each operation of each type it applies to, of each of its values and
 * each second operand it's given; each cast between every pair; each
 * string parsed as each type, and in each radix. */
export function numberCases(): NumberCase[] {
  const out: Omit<NumberCase, "n">[] = [];
  const v = (expr: string, ty: string) => `v::<${ty}>(${expr})`;
  for (const n of nums) {
    for (const op of ops) {
      if (!op.of(n)) continue;
      for (const a of valuesOf(n)) {
        for (const b of op.b?.(n) ?? [""]) {
          const text = op.text(v(a, n.ty), v(b, op.by?.(n) ?? n.ty), n);
          out.push({ id: `${op.name} | ${n.ty} | ${a}${b === "" ? "" : ` | ${b}`}`, group: op.name, ty: n.ty, text, tys: [n.ty] });
        }
      }
    }
  }
  for (const from of nums) {
    for (const to of nums) {
      for (const a of valuesOf(from)) {
        out.push({ id: `${cast(from, to)} | ${a}`, group: "as", ty: from.ty, text: `${v(a, from.ty)} as ${to.ty}`, tys: [from.ty, to.ty] });
        const lit = literal(from, a);
        if (lit !== undefined) {
          // Bound by a `let`, which rustc gives as the cast of a constant;
          // a cast in `println!`'s arguments it promotes, and folds itself.
          const text = `{ let cast = ${lit} as ${to.ty}; cast }`;
          out.push({ id: `literal ${cast(from, to)} | ${a}`, group: "as literal", ty: from.ty, text, tys: [from.ty, to.ty] });
        }
      }
    }
  }
  for (const n of nums) {
    for (const s of parsed(n)) {
      out.push({ id: `parse | ${n.ty} | ${JSON.stringify(s)}`, group: "parse", ty: n.ty, text: `said(${v(JSON.stringify(s), "&str")}.parse::<${n.ty}>())`, tys: [n.ty] });
    }
    if (n.float) continue;
    for (const radix of [2, 8, 16, 36]) {
      for (const s of radixed) {
        out.push({
          id: `from_str_radix ${radix} | ${n.ty} | ${JSON.stringify(s)}`,
          group: "from_str_radix",
          ty: n.ty,
          text: `said(${n.ty}::from_str_radix(${v(JSON.stringify(s), "&str")}, ${radix}))`,
          tys: [n.ty],
        });
      }
    }
  }
  // Interleaved, by a hash of each id, so each program holds many
  // operations of many types, and a slice of a few holds them all.
  const order = (id: string) => Number(Bun.hash(id) % 1_000_000_007n);
  return out
    .map((c) => ({ c, k: order(c.id) }))
    .sort((x, y) => x.k - y.k || (x.c.id < y.c.id ? -1 : 1))
    .map(({ c }, n) => ({ ...c, n }));
}

const prelude = `// Generated by test/number-grid.ts (#940). Each case prints its name,
// then what it makes, or its panic.
use std::fmt::Debug;
use std::hint::black_box;
use std::panic::catch_unwind;

/** \`isize\` and \`usize\` as rust-js has them, wasm32's, 32 bits: native
 * Rust's \`i32\` and \`u32\`. */
#[cfg(target_pointer_width = "32")]
type Isz = isize;
#[cfg(not(target_pointer_width = "32"))]
type Isz = i32;
#[cfg(target_pointer_width = "32")]
type Usz = usize;
#[cfg(not(target_pointer_width = "32"))]
type Usz = u32;

/** A value rustc can't see, so folds nothing of. */
fn v<T>(x: T) -> T {
    black_box(x)
}

/** What a parse made, and of an error, what it says. */
fn said<T: Debug, E: Debug + std::fmt::Display>(r: Result<T, E>) -> String {
    match r {
        Ok(v) => format!("Ok({v:?})"),
        Err(e) => format!("Err({e:?}: {e})"),
    }
}

fn run(name: &str, case: fn()) {
    println!("case {name}");
    if let Err(payload) = catch_unwind(case) {
        let message = payload
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "?".to_string());
        println!("panicked: {message}");
    }
}`;

/** A program of `cases`, which runs each in turn. */
export function numberProgram(cases: NumberCase[]): string {
  const fns = cases.map((c, i) => `fn case_${i}() {\n    println!("{:?}", ${c.text});\n}`);
  const main = `fn main() {\n${cases.map((c, i) => `    run(${JSON.stringify(c.id)}, case_${i});`).join("\n")}\n}`;
  return [prelude, ...fns, main].join("\n\n") + "\n";
}

/** What a run without `ITERATOR_MATRIX` covers, of a case (`slice()` in
 * test/iterator-matrix.ts): its operation, its types, and its operation of
 * its type's kind, signed, unsigned or float. */
export const numberStrata = (c: NumberCase) => {
  const n = nums.find((n) => n.ty === c.ty)!;
  const kind = n.float ? "float" : n.signed ? "signed" : "unsigned";
  return [c.group, ...c.tys, `${c.group} | ${kind}`];
};
