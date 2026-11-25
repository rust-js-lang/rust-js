// Runs a read case's calls (`getter-reads.test.ts`), each given what JS
// made, a counting `Proxy` of each object it's given: every read of a
// property, a getter's as a field's, every write, test of a key, listing
// of keys and call, logged in the order it's made, and what each call gave
// or threw. A `{ "$reads": [..] }` property is a getter whose answer
// changes. Under Node, as the generated JS runs.
//
//   node test/getter-reads-run.ts case.js calls.json log.json

import { readFileSync, writeFileSync } from "node:fs";
import { pathToFileURL } from "node:url";

import { countingProxies } from "./counting-proxies.ts";
import { observe } from "./oracle.ts";

const [module, callsFile, logFile] = process.argv.slice(2);
const calls: { fn: string; args: unknown[] }[] = JSON.parse(readFileSync(callsFile, "utf8"));
const lib = await import(pathToFileURL(module).href);

const { log, show, note, counted } = countingProxies();

const results = calls.map(({ fn, args }, i) => {
  note(`${fn}(..)`);
  const given = args.map((arg, k) => counted(arg, `call${i}.arg${k}`));
  // What it gives, shown quietly, not as `observe` reads it.
  let value: unknown;
  const outcome = observe(() => {
    value = lib[fn](...given);
  });
  return "value" in outcome ? { value: show(value) } : outcome;
});
writeFileSync(logFile, JSON.stringify({ log, results }, null, 1));
