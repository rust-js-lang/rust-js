// The differential oracle: what native Rust did with a call, and what the
// generated JS did, in one shape, compared exactly.
//
//   native Rust ── JSON line ──► decode ──► { value } | { panic: message } ─┐
//   generated JS ─── observe ──► { value } | { panic } | { error } ─────────┴─► same?
//
// A Rust panic is what the runtime's `$isPanic` says is one, a plain `Error`
// with Rust's message (ADRs 0012, 0660). Any other exception, a `TypeError`
// say, is the generated JS going wrong, so it's an `error`, which no native
// outcome is.

import { join } from "node:path";
import { pathToFileURL } from "node:url";

import { helpersModule } from "./packages-of.ts";

// The runtime the generated JS imports, a runtime mutant's own in its
// target (#720): one module, so a panic it throws is one its `$isPanic`
// knows, an `$Unwinding` of its own class.
const { $isPanic, $raisedAt, $unwindsOn } = (await import(pathToFileURL(helpersModule).href)) as {
  $isPanic: (e: unknown) => boolean;
  $raisedAt: symbol;
  $unwindsOn: (e: unknown) => unknown;
};

/** `run`, a case's `main`, as Rust's runtime runs it: what it throws is
 * thrown on through the runtime's `$unwindsOn`, which, of a stack overflow,
 * is Rust's abort, its lines and exit 134. rust-js compiles no binary, so
 * nothing in its JS does this; an export's caller, as this host, does
 * (#1193). */
export function asMain(run: () => unknown): () => unknown {
  return () => {
    try {
      return run();
    } catch (e) {
      throw $unwindsOn(e);
    }
  };
}

// `{ exit }` is a run that ended the process: one that aborted, as Rust does
// where a drop panics while a panic unwinds, `{ exit: 134 }` (#847). A
// panic's `at` is where it was raised, `file:line:col`, as its hook is told,
// where the run says (`observe`'s `located`, a native program's harness,
// #2292); `<unknown>` where the JS was given none.
export type Outcome = { value: unknown } | { panic: string; at?: string } | { error: string } | { exit: number };

// JSON has no -0, NaN, infinities or BigInts, so the native side tags them:
// `{"$f64":"-0"}`, `{"$bigint":"18446744073709551615"}`.
const floats: Record<string, number> = { "-0": -0, NaN: NaN, inf: Infinity, "-inf": -Infinity };

/** A line of JSON native Rust printed, with its tagged values restored. */
export function decode(line: string): any {
  return JSON.parse(line, (_, v) => {
    if (v === null || typeof v !== "object" || Array.isArray(v)) return v;
    const keys = Object.keys(v);
    if (keys.length === 1 && keys[0] === "$f64" && v.$f64 in floats) return floats[v.$f64];
    if (keys.length === 1 && keys[0] === "$bigint") return BigInt(v.$bigint);
    return v;
  });
}

/** Values as JSON, tagged as native Rust tags them, for `decode` to restore
 * in another process. What no Rust value is, a `Map` or a function, is
 * tagged by its kind, so it can't come back as one that is. */
export function encode(value: unknown): string {
  const tagged = (v: unknown): unknown => {
    if (typeof v === "bigint") return { $bigint: String(v) };
    if (typeof v === "number") {
      if (Object.is(v, -0)) return { $f64: "-0" };
      if (Number.isNaN(v)) return { $f64: "NaN" };
      if (v === Infinity) return { $f64: "inf" };
      if (v === -Infinity) return { $f64: "-inf" };
      return v;
    }
    if (v === undefined || v === null || typeof v === "string" || typeof v === "boolean") return v ?? null;
    if (Array.isArray(v)) return v.map(tagged);
    if (typeof v === "object" && Object.getPrototypeOf(v) === Object.prototype) {
      return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, tagged(x)]));
    }
    return { $js: typeof v === "object" ? (v.constructor?.name ?? "object") : typeof v };
  };
  return JSON.stringify(tagged(value));
}

/** Run generated JS as native Rust ran the Rust, and say what happened:
 * `located`, a panic's `at` too. */
export function observe(run: () => unknown, located = false): Outcome {
  try {
    return { value: normalize(run()) };
  } catch (e) {
    if ($isPanic(e)) return located ? { panic: (e as Error).message, at: (e as Record<symbol, string | undefined>)[$raisedAt] ?? "<unknown>" } : { panic: (e as Error).message };
    return { error: e instanceof Error ? `${e.name}: ${e.message}` : `a thrown ${typeof e}: ${String(e)}` };
  }
}

/** `None` is `undefined` in JS and `null` in JSON; everything else is kept. */
export function normalize(v: unknown): unknown {
  if (v === undefined) return null;
  if (Array.isArray(v)) return v.map(normalize);
  if (v !== null && typeof v === "object" && Object.getPrototypeOf(v) === Object.prototype) {
    return Object.fromEntries(Object.entries(v).map(([k, x]) => [k, normalize(x)]));
  }
  return v;
}

/** What native Rust recorded: `{ value }`, or `{ panic }` with its message,
 * and where it was raised where it says. */
export function expected(c: { value?: unknown; panic?: string; at?: string }): Outcome {
  if (c.panic === undefined) return { value: c.value };
  return c.at === undefined ? { panic: c.panic } : { panic: c.panic, at: c.at };
}

/** Strictly the same: -0 isn't 0, 1n isn't 1, a `TypeError` isn't a panic.
 * Two panics' `at`s are compared where both say one: a harness that
 * records no location, `compiler.test`'s, and a case `located-otherwise`
 * says differs there, whose native run leaves it out (`runNative`), are
 * compared by the rest (#2292). */
export function same(a: Outcome, b: Outcome): boolean {
  if ("panic" in a && "panic" in b && (a.at === undefined) !== (b.at === undefined)) {
    return Bun.deepEquals({ panic: a.panic }, { panic: b.panic }, true);
  }
  return Bun.deepEquals(a, b, true);
}
