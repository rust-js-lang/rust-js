// Runs a corpus case's JS as its native binary runs: what `main` prints is
// this process's output, and how it ended, `{ value }` or `{ panic }`, goes
// to the file named second; or, if it ended the process, `process::exit`,
// `{ exit }` of its code. Node runs it (it strips types); so can Bun. What
// follows is the program's own arguments, `env::args()`, as its native
// binary was given them, its path first (ADR 0391).
//
//   bun test/corpus-run.ts case.js outcome.json case.js outcome.json
//   node test/corpus-run.ts case.js outcome.json /tmp/native

import { writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { observe } from "./oracle.ts";

const [module, outcomeFile, ...program] = process.argv.slice(2);
process.argv = [process.argv[0], ...program];
let ended = false;
process.once("exit", (code) => {
  if (!ended) writeFileSync(outcomeFile, JSON.stringify({ exit: code }));
});
// What its module throws as it loads, a constant it makes as code (ADR
// 0096), ends it as `main` would.
const run = await import(pathToFileURL(module).href).then(
  ({ entry }) => entry,
  (error) => () => {
    throw error;
  },
);
const outcome = observe(run);
ended = true;
writeFileSync(outcomeFile, JSON.stringify(outcome));
