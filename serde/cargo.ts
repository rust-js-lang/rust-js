// Cargo, run as `rust-js build` runs it (tooling/cargo.js's `cargoEnv`):
// rust-js as rustc's wrapper, compiling none of these crates, each of which
// it reads as rustc builds it, so each check notes beside its metadata what
// a consumer's compile reads of it, the sites a registry crate's runtime
// model raises its panics at (#2325), the one way a host builds them. The
// compiler the tests run, `RUST_JS_COMPILER`, or the debug one.
//
//   bun serde/cargo.ts build --message-format=json ..
import { spawnSync } from "node:child_process";
import { resolve } from "node:path";
import { cargoEnv } from "../tooling/cargo.js";

const compiler = process.env.RUST_JS_COMPILER ?? resolve(import.meta.dir, "..", "target", "debug", "rust-js");
const cargo = spawnSync("cargo", process.argv.slice(2), {
  stdio: "inherit",
  env: cargoEnv(compiler, undefined, { apps: [], crates: [] }),
});
process.exit(cargo.status ?? 1);
