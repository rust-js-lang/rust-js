// Runs a module's JS, a crate that exports `entry`, calling the program's
// `main` (`compileJs`'s `module` and `library` builds, #2447), as a binary's
// JS runs its `main`: by the runtime's `$main`, as std's runtime runs it
// (#2292), its status the process's, std's hook's text of each panic on
// stderr, `process::exit` its code. What follows the module, where given,
// is the program's own arguments, `env::args()`, its path first, as a
// binary's JS has them (ADR 0391): the rustc suite gives its native
// binary's path, which a test that prints `args()` prints; else its path
// is the module's. Node runs it (it strips types); so can Bun.
//
//   node test/corpus-run.ts module/case.js [path args..]

import { pathToFileURL } from "node:url";

import { helpersModule } from "./packages-of.ts";

const [module, ...given] = process.argv.slice(2);
process.argv = [process.argv[0], ...(given.length > 0 ? given : [module])];
// The runtime the module imports, one module, so its `$main` is the one
// whose hook its panics run.
const { $main } = (await import(pathToFileURL(helpersModule).href)) as { $main: (entry: () => unknown) => void };
// What its module throws as it loads, a constant it makes as code (ADR
// 0096), ends it as `main` would.
const entry = await import(pathToFileURL(module).href).then(
  ({ entry }) => entry as () => unknown,
  (error) => () => {
    throw error;
  },
);
$main(entry);
