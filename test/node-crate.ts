// The node crate's metadata for the tests that use it, a module of its own: in test/support.ts, which every test
// imports, its reads of node/, webapi/ and builtins/'s sources and build scripts made each test a reader of them, so the
// selector ran the whole suite for a change to them (ADR 1101, amended).

import { join } from "node:path";

import { buildCompiler, compiler, once, root, run, target, thisRun, unlessUnchanged } from "./support";

/** The node crate's metadata, and the webapi and js crates' it uses beside
 * it, in `target/node`, built once for the run and not again while what
 * they're made from is what it was, as the react crate's are (`buildReact`):
 * in a directory of their own, as node's build writes the webapi crate's
 * metadata too, which another test file may be reading from `target`. Ten
 * tests built it each, 1.5 s each. Where it is: `-L` it, and `--extern
 * node=<it>/libnode.rmeta`. */
let node: string | undefined;
export function buildNode(): string {
  const dir = join(target, "node");
  if (node === undefined) {
    buildCompiler();
    const inputs = ["node/src", "node/build.sh", "webapi/src", "webapi/build.sh", "builtins/src", "builtins/build.sh", "rust-toolchain.toml"]
      .map((path) => join(root, path))
      .concat(compiler);
    const outputs = ["node", "webapi", "js"].map((lib) => join(dir, `lib${lib}.rmeta`));
    once(join(target, "tests-node", thisRun()), () =>
      unlessUnchanged(join(dir, "libnode.stamp"), inputs, outputs, () => run(["node/build.sh", "-o", join(dir, "libnode.rmeta")])),
    );
    node = dir;
  }
  return node;
}
