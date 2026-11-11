// Rustlantis (Wang and Jung, OOPSLA 2024), the net's generator of custom
// MIR programs (ADR 1125): its source at a pinned commit with rust-js's
// wasm32 patch (test/rustlantis/wasm32.patch: a usize and a pointer of 4
// bytes, as rust-js's target's), built once a machine by the pinned
// toolchain, kept in `.cache/rustlantis/<commit>/` (#1213); and the net's
// lists of what it knows, test/rustlantis-known.txt.
import { existsSync, mkdirSync, readdirSync, readFileSync, renameSync, rmSync } from "node:fs";
import { join } from "node:path";

import { rustlantisStore as store } from "./kept";
import { root } from "./support";

/** Rustlantis's commit the net generates by. */
export const commit = "2be7c5a6763831dc0f908f0e05723ebbbc2a62cc";
const patch = join(root, "test", "rustlantis", "wasm32.patch");
/** Its config, read from where it runs: small programs. */
export const configDir = join(root, "test", "rustlantis");

const sh = (cmd: string[], cwd: string, env: Record<string, string> = {}) => {
  const p = Bun.spawnSync(cmd, { cwd, env: { ...process.env, ...env }, stdout: "pipe", stderr: "pipe" });
  if (p.exitCode !== 0) throw new Error(`${cmd.join(" ")} failed in ${cwd}:\n${p.stderr.toString()}`);
};

/** The generator, built where no build of this commit and patch is: in a
 * directory of its own, then moved into place, so parts building at once
 * each finish with one; another commit's removed. */
export function generator(): string {
  const key = `${commit.slice(0, 12)}-${Bun.hash(readFileSync(patch)).toString(36)}`;
  const entry = join(store, key);
  const binary = join(entry, "target", "release", "generate");
  if (existsSync(binary)) return binary;
  mkdirSync(store, { recursive: true });
  for (const old of readdirSync(store)) if (old !== key && !old.startsWith(".")) rmSync(join(store, old), { recursive: true, force: true });
  const building = join(store, `.${key}-${process.pid}`);
  rmSync(building, { recursive: true, force: true });
  mkdirSync(building, { recursive: true });
  sh(["git", "init", "-q"], building);
  sh(["git", "fetch", "-q", "--depth=1", "https://github.com/cbeuw/rustlantis.git", commit], building);
  sh(["git", "checkout", "-q", "FETCH_HEAD"], building);
  sh(["git", "apply", patch], building);
  // Its crates ask nightly features: the pinned toolchain's, by its
  // bootstrap, as rust-js's own programs (ADR 0109).
  sh(["cargo", "build", "--release", "--quiet", "-p", "generate"], building, { RUSTC_BOOTSTRAP: "1" });
  rmSync(join(building, ".git"), { recursive: true, force: true });
  try {
    renameSync(building, entry);
  } catch {
    // Another part's build is in place.
    rmSync(building, { recursive: true, force: true });
  }
  return binary;
}

/** What the net knows: a refusal of an open issue's, or of a form only
 * custom MIR writes (`custom-mir`); a wrong answer or a throw an open
 * issue holds; each by its pattern, of a refusal's message, a program's
 * source, or the JS's error. */
export type Known = { kind: "refused" | "wrong" | "threw" | "crashed"; by: string; pattern: RegExp };

export function known(file = join(root, "test", "rustlantis-known.txt")): Known[] {
  return readFileSync(file, "utf8")
    .split("\n")
    .filter((line) => line.trim() !== "" && !line.startsWith("#"))
    .map((line) => {
      const [kind, by, ...pattern] = line.split(" ");
      if (!["refused", "wrong", "threw", "crashed"].includes(kind)) throw new Error(`rustlantis-known.txt: ${kind}: refused, wrong, threw or crashed`);
      if (!/^(#\d+|custom-mir)$/.test(by)) throw new Error(`rustlantis-known.txt: ${by}: an issue, #n, or custom-mir`);
      return { kind: kind as Known["kind"], by, pattern: new RegExp(pattern.join(" ")) };
    });
}

/** The entry of `kinds` whose pattern `text` matches. */
export const knownAs = (list: Known[], kinds: Known["kind"][], text: string) =>
  list.find((k) => kinds.includes(k.kind) && k.pattern.test(text));
