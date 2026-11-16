// What scripts/mutations.ts holds each mutant's build to, built as it is
// incrementally, as on the Mac and, with `CARGO_INCREMENTAL=1`, on CI
// (docs/dev-loop.md): a build that reused what it shouldn't would judge
// another mutant, or the compiler as it is, by the mutation's name.
//
// - The mutant's files are in the crate as built, each as the mutation
//   wrote it (`unapplied`).
// - Cargo compiled the crate again (`builtBinary`'s `rebuilt`).
// - Its binary is no other build's of the run, the compiler's as it is
//   among them (`binaryHash`).
// - `--fresh=<share>`, the nightly's: that share of the builds built again
//   with no incremental cache, and each of their mutations judged by both,
//   the run failing where the two say other things (`verdictKind`).

import { createHash } from "node:crypto";
import { existsSync, readFileSync } from "node:fs";
import { join } from "node:path";

/** The binary cargo said it built, of its `--message-format=json` output:
 * rust-js's, or, `unit`, its unit tests' (`cargo test --no-run`); and
 * whether it compiled the crate again for it, not `fresh` as cargo says
 * of one it left as it was. */
export function builtBinary(stdout: string, unit: boolean): { binary: string; rebuilt: boolean } | undefined {
  for (const line of stdout.split("\n")) {
    if (!line.startsWith("{")) continue;
    const message = JSON.parse(line);
    if (message.reason !== "compiler-artifact" || typeof message.executable !== "string") continue;
    if (message.target?.name !== "rust-js" || (message.profile?.test === true) !== unit) continue;
    return { binary: message.executable, rebuilt: message.fresh === false };
  }
  return undefined;
}

/** Of a mutant's `files`, written over the crate at `crate`, the first that
 * isn't there as written, or is the crate's own as `source` has it, which
 * the mutation would have changed; none where each is its mutant's. */
export function unapplied(files: Map<string, string>, crate: string, source: string): string | undefined {
  for (const [path, text] of files) {
    if (!existsSync(join(crate, path)) || readFileSync(join(crate, path), "utf8") !== text) return `${path} isn't as the mutation wrote it`;
    if (existsSync(join(source, path)) && readFileSync(join(source, path), "utf8") === text) return `${path} is as the crate has it`;
  }
  return undefined;
}

/** The SHA-256 of the file at `path`, in hex. */
export function binaryHash(path: string): string {
  return createHash("sha256").update(readFileSync(path)).digest("hex");
}

/** What a result says of its mutation, its first word: `caught`,
 * `SURVIVED`, `INCONCLUSIVE`, or a problem's, as `doesn't build`. A fresh
 * build's verdict is held to its incremental build's by it. */
export function verdictKind(result: string): string {
  return result.split(/[\s:]/)[0];
}

/** The share of the builds built again fresh, of `--fresh=<share>`, 0 to
 * 1; 0 where it isn't given. */
export function freshShare(args: string[]): number {
  const said = args.find((arg) => arg.startsWith("--fresh="))?.slice("--fresh=".length);
  if (said === undefined) return 0;
  const share = Number(said);
  if (!(share >= 0 && share <= 1)) throw new Error(`--fresh=${said}: a share of the builds, 0 to 1`);
  return share;
}
