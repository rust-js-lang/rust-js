// The metamorphic net (#939): each corpus case rewritten in a way that keeps
// its meaning, by `scripts/metamorph`, run natively and as JS in both builds
// and compared. Native Rust's run of the rewrite is its oracle, so no
// snapshot is needed; a rewrite rustc rejects is no variant. Each case runs
// as native Rust does as it's written (the corpus); one whose rewrite
// doesn't is a shape around a value that the lowering reads otherwise:
// lesson 2 of docs/finding-wrong-answers.md.
//
//   case.rs ─► metamorph <rewrite> ─┬─ rustc ──► native ─────┐
//                                   └─ rust-js ─► node ──────┴─► the same?

import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

import { cases, corpus, directives } from "./corpus-cases";
import { root, target } from "./support";
import { runSync } from "./child";

/** Each rewrite, `metamorph`'s: `main`'s body outlined; each `let` of a
 * name bound once moved into a box, an `Option`, an `Rc<RefCell>` and a
 * struct's field; each reference parameter of a free function taken as
 * `&dyn` and as `&impl` of an identity trait; each owned one as `impl` of
 * one; a free function made generic over a type it takes; each closure
 * that captures nothing made a `fn`, and each function given as a value a
 * closure; every item but `main` moved into a library crate. */
export const rewrites = ["outline", "boxed", "dyn-ref", "impl-ref", "impl-owned", "generic", "option", "rc-refcell", "field", "closure-fn", "library"] as const;
export type Rewrite = (typeof rewrites)[number];

/** Where the rewriter's crate is, and what it builds. */
const manifest = join(root, "scripts", "metamorph", "Cargo.toml");
const binary = join(target, "metamorph", "debug", "metamorph");

/** The rewriter, built once: its own crate, apart from the compiler's. */
export function buildRewriter(): void {
  const p = runSync(["cargo", "build", "--quiet", "--manifest-path", manifest, "--target-dir", join(target, "metamorph")], root, 600_000);
  if (p.code !== 0) throw new Error(`the rewriter doesn't build:\n${p.stderr}`);
}

/** `rewrite` of the case `file`, of so many of the places it finds,
 * `limit`'s, or `undefined` where it changes nothing: the program, and of
 * `library`, the library it uses. `closure-fn` asks rustc for a closure's
 * types, at the case's `edition`. */
export function rewritten(rewrite: Rewrite, file: string, edition: string, limit?: number): { program: string; library?: string } | undefined {
  const p = runSync([binary, rewrite, file, ...(limit === undefined ? [] : [String(limit)])], root, 60_000, { METAMORPH_EDITION: edition });
  if (p.code === 2) return undefined;
  if (p.code !== 0) throw new Error(`metamorph ${rewrite} ${file}: ${p.stderr}`);
  if (rewrite !== "library") return { program: p.stdout };
  const [library, program] = p.stdout.split(libraryEnds);
  if (program === undefined) throw new Error(`metamorph library ${file}: no \`${libraryEnds.trim()}\` line`);
  return { program, library };
}

/** The line `library` prints between the library and the program
 * (`scripts/metamorph/src/library.rs`'s `MAIN`). */
const libraryEnds = "\n// metamorph: main\n";

/** The library crate `library`'s program uses, by its name. */
export const libraryCrate = "metamorph_lib";

/** Whether a native run of `rewrite` ended in a panic the rewrite made
 * itself, not the case: two borrows of the `RefCell` an `rc-refcell` local
 * is held in that overlap, which the case's own place never checked. Such
 * a rewrite is no variant, as one rustc rejects is not. */
export function overlapping(rewrite: Rewrite, outcome: unknown): boolean {
  if (rewrite !== "rc-refcell" || typeof outcome !== "object" || outcome === null || !("panic" in outcome)) return false;
  const message = String((outcome as { panic: unknown }).panic);
  return message === "RefCell already borrowed" || message === "RefCell already mutably borrowed";
}

/** Why a case is no case of the net, or `undefined`: what its directives
 * expect that a rewrite's run can't carry, a compile error, a refusal, an
 * abort, another target's `usize`. */
export function apart(source: string): string | undefined {
  const want = directives(source);
  if (typeof want === "string") return want;
  if (want.kind !== "run-pass" && want.kind !== "run-fail") return want.kind;
  if (want.native !== "host") return "native wasm32";
  return undefined;
}

/** The cases a rewrite may run, in name order, with their editions. */
export function rewritable(): { name: string; edition: string }[] {
  return cases.flatMap((name) => {
    const source = readFileSync(join(corpus, name), "utf8");
    const want = directives(source);
    return apart(source) === undefined && typeof want !== "string" ? [{ name, edition: want.edition }] : [];
  });
}

/** A seeded generator of numbers in [0, 1): mulberry32. */
function random(seed: number): () => number {
  let a = seed >>> 0;
  return () => {
    a = (a + 0x6d2b79f5) >>> 0;
    let t = a;
    t = Math.imul(t ^ (t >>> 15), t | 1);
    t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

/** Whether `name` is in a push's sample, one case in `of` by its name's
 * hash, so a case added changes only whether it is (as the neighbour
 * shuffle's groups are, #853). */
export function sampled(name: string, of = 128): boolean {
  return Math.floor(random(Number(BigInt.asUintN(32, Bun.hash(name) as bigint)))() * of) === 0;
}

/** A rewrite of a case the shuffle found going wrong, listed under its
 * issue: its builds, its rewrite and its case. */
export type Known = { issue: string; builds: string[]; rewrite: string; name: string };

export const knownFile = join(import.meta.dir, "metamorphic-known.txt");

/** test/metamorphic-known.txt's rewrites, or why a line isn't one:
 * `<builds> <rewrite> <case>` under an `#<issue> <title>`. */
export function readKnown(text = existsSync(knownFile) ? readFileSync(knownFile, "utf8") : ""): Known[] {
  const known: Known[] = [];
  let issue: string | undefined;
  for (const line of text.split("\n")) {
    if (line === "" || line.startsWith("//")) continue;
    const heading = /^#(\d+) /.exec(line);
    if (heading) {
      issue = heading[1];
      continue;
    }
    const words = line.split(" ");
    const [name, rewrite] = [words.at(-1)!, words.at(-2)!];
    const builds = words.slice(0, -2);
    if (issue === undefined || !(rewrites as readonly string[]).includes(rewrite) || !name.endsWith(".rs") || builds.length === 0 || builds.some((b) => b !== "plain" && b !== "std")) {
      throw new Error(`test/metamorphic-known.txt: \`${line}\` isn't \`<builds> <rewrite> <case.rs>\` under an \`#<issue>\``);
    }
    known.push({ issue, builds, rewrite, name });
  }
  return known;
}
