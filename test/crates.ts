// What the crates tests share: the toolchain, Cargo for rust-js's target,
// and what a module's `main` prints under Node.

import { readFileSync } from "node:fs";
import { join } from "node:path";
import { cargoBuild, cargoEnv, checkCargo } from "../tooling/cargo.js";
import { node } from "./programs";
import { compiler, root, run } from "./support";
import { channel } from "../scripts/toolchain.ts";

export const cargo = (manifest: string, ...args: string[]) => ["cargo", `+${channel}`, ...args, "--offline", "--quiet", "--manifest-path", manifest];
/** Cargo's environment of `manifest`'s build through rust-js, as `rust-js build` gives it (ADR 0414). */
export const throughRustJs = async (manifest: string) => cargoEnv(compiler, undefined, await cargoBuild({ manifestPath: manifest, toolchain: channel, offline: true }));
/** `packageName` checked for rust-js's target, with rust-js as Cargo's wrapper. */
export const check = (manifestPath: string, options: { packageName?: string, features?: string[], react?: string } = {}) =>
  checkCargo({ manifestPath, toolchain: channel, compiler, offline: true, packageName: "frontend", ...options });

export const printed = (app: string) => run([node ?? "node", "--input-type=module", "--eval", `(await import(${JSON.stringify(app)})).main();`]);
