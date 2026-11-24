// What the crates tests share: the toolchain, Cargo for rust-js's target,
// and what a module's `main` prints under Node.

import { expect } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { cargoBuild, cargoEnv, checkCargo } from "../tooling/cargo.js";
import { nondeterministic } from "./corpus-cases";
import { keepRun, keptRun, recheck, rechecked } from "./native-outcomes";
import { node, type Run } from "./programs";
import { compiler, fixture, nativeCrates, nativeCratesKey, programFlags, root, run, target } from "./support";
import { channel } from "../scripts/toolchain.ts";

export const cargo = (manifest: string, ...args: string[]) => ["cargo", `+${channel}`, ...args, "--offline", "--quiet", "--manifest-path", manifest];
/** Cargo's environment of `manifest`'s build through rust-js, as `rust-js build` gives it (ADR 0414). */
export const throughRustJs = async (manifest: string) => cargoEnv(compiler, undefined, await cargoBuild({ manifestPath: manifest, toolchain: channel, offline: true }));
/** `packageNames` checked for rust-js's target, with rust-js as Cargo's wrapper. */
export const check = (manifestPath: string, options: { packageNames?: string[], features?: string[], react?: string } = {}) =>
  checkCargo({ manifestPath, toolchain: channel, compiler, offline: true, packageNames: ["frontend"], ...options });

export const printed = (app: string) => run([node ?? "node", "--input-type=module", "--eval", `(await import(${JSON.stringify(app)})).main();`]);

// Two crates at a time (`test/crates/pairs/<case>/`): a library, `dep`, and an
// `app` using it, for each thing crossing between them that a consumer must
// be told, or must assume. Three at a time (`test/crates/triples/<case>/`):
// `dep`, `mid` using it, and `app` using both (ADR 0523).
export const pairs = join(root, "test", "crates", "pairs");
export const triples = join(root, "test", "crates", "triples");

/** What the native program of `crates` prints, `nativeCrates`', asked once:
 * its run kept by what it was built from, as a case's is
 * (test/native-outcomes.ts), so a chain unchanged since is neither built
 * nor run again; and of the nightly's share, run again too. */
export function nativeCratesOutput(crates: { name: string; file: string; uses: string[]; edition?: string }[], main: string, flags: string[], free?: () => number): () => string {
  const key = nativeCratesKey(crates, main, flags);
  // Not of a chain one of whose crates reads the world (`nondeterministic`).
  const keeps = !crates.some(({ file }) => nondeterministic(readFileSync(file, "utf8")) !== undefined);
  let said: string | undefined;
  const fresh = (): Run => {
    const built = nativeCrates(crates, main, flags, undefined, free);
    if ("error" in built) throw new Error(built.error);
    const stdout = run([built.binary]);
    const ran: Run = { stdout, stderr: "", bytes: { stdout: Buffer.from(stdout), stderr: Buffer.from("") }, outcome: { exit: 0 } };
    if (keeps && built.kept) keepRun(key, dirname(built.binary), ran, undefined, free);
    return ran;
  };
  return () => {
    if (said !== undefined) return said;
    const kept = keeps ? keptRun(key) : undefined;
    if (kept !== undefined && rechecked()) {
      const problem = recheck(kept, fresh);
      if (problem !== undefined) throw new Error(`${crates.at(-1)!.file}: ${problem}`);
    }
    said = (kept ?? fresh()).stdout;
    return said;
  };
}

/** `crates` of `cases/name`, each a library of the ones before it but the
 * last, the app, compiled natively and by rust-js: the app's JS prints what
 * the native program does; where each crate's JS is. A crate's file names
 * it, `dep.rs` the crate `dep`, but where its first line says otherwise,
 * `//@ crate-name: core`; one named `std` is given to a `#![no_std]` crate
 * alone, as another's `std` is std's (ADR 1054); and each is given those
 * before it, or those its `//@ uses: dep` names. Where `refused` is given,
 * the app's compile is refused instead, saying each of them: what the
 * native program prints. Each rust-js compile is given `env`. */
export function chain(cases: string, name: string, files: string[], refused?: string[], throws?: string, env: Record<string, string> = {}): string {
  const dir = fixture(`${files.length === 2 ? "pair" : "triple"}-${name}`);
  const out = join(dir, "js");
  const source = (file: string) => join(cases, name, `${file}.rs`);
  const crates = files.map((file) => readFileSync(source(file), "utf8").match(/^\/\/@ crate-name: (\w+)\n/)?.[1] ?? file);
  // A crate's edition, `//@ edition: 2018`, as the corpus's directive says
  // a case's: itertools' closures capture as 2018's do (#1681).
  const editions = files.map((file) => readFileSync(source(file), "utf8").match(/^\/\/@ edition: (\d+)\n/m)?.[1]);
  // The crates a crate uses: each before it, or those its directive names,
  // `//@ uses: dep`, none of `//@ uses:`, as Cargo gives a crate its own
  // dependencies alone, a sibling none of another's (#1802).
  const declared = files.map((file) => readFileSync(source(file), "utf8").match(/^\/\/@ uses:(.*)\n/m)?.[1]);
  const before = (i: number) => crates.slice(0, i)
    .filter((crate) => declared[i] === undefined || declared[i].split(",").map((s) => s.trim()).includes(crate));
  const given = (i: number) => before(i)
    .filter((used) => used !== "std" || readFileSync(source(files[i]), "utf8").includes("#![no_std]"));
  // Natively, each crate a library and a program of the last, kept by what
  // they're made of (`nativeCrates`).
  const app = crates[crates.length - 1];
  const native = nativeCratesOutput(
    crates.map((crate, i) => ({ name: crate, file: source(files[i]), uses: given(i), edition: editions[i] })),
    `fn main() {\n    ${app}::main();\n}\n`,
    programFlags("2024"),
  );
  const compile = (crate: string, i: number) => {
    const used = before(i);
    const library = i < crates.length - 1
      ? ["--library", "--manifest", join(out, crate, "lib.manifest.json")]
      : [];
    const metadata = i < crates.length - 1 ? [`--emit=metadata=${join(out, crate, `lib${crate}.rmeta`)}`] : [];
    run([compiler, source(files[i]), "-o", join(out, crate, "lib.js"), ...library,
      ...used.flatMap((u) => ["--dependency", join(out, u, "lib.manifest.json")]),
      "--", "--crate-name", crate, ...metadata, ...(editions[i] ? [`--edition=${editions[i]}`] : []),
      ...given(i).flatMap((u) => ["--extern", `${u}=${join(out, u, `lib${u}.rmeta`)}`]),
      ...used.flatMap((u) => ["-L", `dependency=${join(out, u)}`])], 600_000, env);
  };
  if (refused) {
    crates.slice(0, -1).forEach(compile);
    let said = "";
    try {
      compile(app, crates.length - 1);
    } catch (error) {
      said = String(error);
    }
    for (const words of refused) expect(said).toContain(words);
    return native();
  }
  crates.forEach(compile);
  // Where `throws` is given, the app's JS ends by an error saying it, as
  // rust-js refuses at run time what it can't run as Rust does: what the
  // native program prints.
  if (throws !== undefined) {
    expect(() => printed(join(out, app, "lib.js"))).toThrow(throws);
    return native();
  }
  expect(printed(join(out, app, "lib.js"))).toBe(native());
  return out;
}

/** The library contract of `out`'s `dep`. */
export function contract(out: string) {
  return JSON.parse(readFileSync(join(out, "dep", "lib.manifest.json"), "utf8")).library;
}

/** `source`, a crate using `out`'s `dep`, or the crates `uses` names, compiled. */
export function counting(out: string, source: string, uses = ["dep"]) {
  const file = join(out, "counting.rs");
  writeFileSync(file, source);
  run([compiler, file, "-o", join(out, "counting", "lib.js"), ...uses.flatMap((u) => ["--dependency", join(out, u, "lib.manifest.json")]),
    "--", "--crate-name", "counting", ...uses.flatMap((u) => ["--extern", `${u}=${join(out, u, `lib${u}.rmeta`)}`, "-L", `dependency=${join(out, u)}`])]);
}

export const js = ["--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

/** `dep`, a library, and `app`, a crate that uses it, compiled in `dir`:
 * each one's JS. */
export function compiled(dir: string, dep: string, app: string): { dep: string; app: string } {
  writeFileSync(join(dir, "dep.rs"), dep);
  run([compiler, join(dir, "dep.rs"), "-o", join(dir, "dep", "lib.js"), "--library", "--manifest", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "dep", `--emit=metadata=${join(dir, "libdep.rmeta")}`, ...js]);
  writeFileSync(join(dir, "app.rs"), app);
  run([compiler, join(dir, "app.rs"), "-o", join(dir, "app", "lib.js"), "--dependency", join(dir, "dep", "lib.manifest.json"),
    "--", "--crate-name", "app", "--extern", `dep=${join(dir, "libdep.rmeta")}`, "-L", dir, ...js]);
  return { dep: readFileSync(join(dir, "dep", "lib.js"), "utf8"), app: readFileSync(join(dir, "app", "lib.js"), "utf8") };
}
