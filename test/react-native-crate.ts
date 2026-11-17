// The react-native crate's metadata for the tests that use it, a module of its own: in test/support.ts, which every test
// imports, its reads of react-native/ and builtins/'s sources and build scripts made each test a reader of them, so the
// selector ran the whole suite for a change to them (ADR 1101, amended).

import { join } from "node:path";

import { buildCompiler, compiler, once, root, run, target, thisRun, unlessUnchanged } from "./support";

/** The react-native crate's metadata, and the js crate's it uses beside
 * it, in `target/react-native`, built once for the run and not again while
 * what they're made from is what it was, as the node crate's are
 * (`buildNode`). Where it is: `-L` it, and `--extern
 * react_native=<it>/libreact_native.rmeta`. */
let reactNative: string | undefined;
export function buildReactNative(): string {
  const dir = join(target, "react-native");
  if (reactNative === undefined) {
    buildCompiler();
    const inputs = ["react-native/src", "react-native/build.sh", "builtins/src", "builtins/build.sh", "rust-toolchain.toml"]
      .map((path) => join(root, path))
      .concat(compiler);
    const outputs = ["react_native", "js"].map((lib) => join(dir, `lib${lib}.rmeta`));
    once(join(target, "tests-react-native", thisRun()), () =>
      unlessUnchanged(join(dir, "libreact_native.stamp"), inputs, outputs, () =>
        run(["react-native/build.sh", "-o", join(dir, "libreact_native.rmeta")]),
      ),
    );
    reactNative = dir;
  }
  return reactNative;
}
