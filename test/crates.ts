// What the crates tests share: the toolchain, Cargo for rust-js's target,
// and what a module's `main` prints under Node.

import { expect } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { cargoBuild, cargoEnv, checkCargo } from "../tooling/cargo.js";
import { node } from "./programs";
import { compiler, fixture, nativeCrates, root, run } from "./support";
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

/** `crates` of `cases/name`, each a library of the ones before it but the
 * last, the app, compiled natively and by rust-js: the app's JS prints what
 * the native program does; where each crate's JS is. A crate's file names
 * it, `dep.rs` the crate `dep`, but where its first line says otherwise,
 * `//@ crate-name: core`; one named `std` is given to a `#![no_std]` crate
 * alone, as another's `std` is std's (ADR 1054). Where `refused` is given,
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
  const given = (i: number) => crates.slice(0, i)
    .filter((used) => used !== "std" || readFileSync(source(files[i]), "utf8").includes("#![no_std]"));
  // Natively, each crate a library and a program of the last, kept by what
  // they're made of (`nativeCrates`).
  const app = crates[crates.length - 1];
  const built = nativeCrates(
    crates.map((crate, i) => ({ name: crate, file: source(files[i]), uses: given(i), edition: editions[i] })),
    `fn main() {\n    ${app}::main();\n}\n`,
    ["--edition=2024", "-Coverflow-checks=off"],
  );
  if ("error" in built) throw new Error(built.error);
  const program = built.binary;
  const compile = (crate: string, i: number) => {
    const used = crates.slice(0, i);
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
    return run([program]);
  }
  crates.forEach(compile);
  // Where `throws` is given, the app's JS ends by an error saying it, as
  // rust-js refuses at run time what it can't run as Rust does: what the
  // native program prints.
  if (throws !== undefined) {
    expect(() => printed(join(out, app, "lib.js"))).toThrow(throws);
    return run([program]);
  }
  expect(printed(join(out, app, "lib.js"))).toBe(run([program]));
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
