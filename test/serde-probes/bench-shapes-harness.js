// bench-shapes-harness.js <shape> <n> <mode> [reps]: bench-shapes.rs's JS,
// one operation per process, timed as bench-harness.js times #1542's
// document; peak memory is read by /usr/bin/time -l (#1532's S0).
//   emit: writes shape-<shape>-<n>.json
//   readTyped, writeTyped, readValue, writeValue: the best of `reps` runs
import { make, readTyped, writeTyped, readValue, writeValue } from "./bench-shapes.js";
import { readFileSync, writeFileSync } from "node:fs";
const [shape, n, mode] = [process.argv[2], +process.argv[3], process.argv[4]];
const reps = +(process.argv[5] ?? 1);
const file = `shape-${shape}-${n}.json`;
const timed = (f) => {
  let best = Infinity, out;
  for (let i = 0; i < reps; i++) {
    const t = performance.now();
    out = f();
    best = Math.min(best, performance.now() - t);
  }
  return [best, out];
};
let ms = 0, note = "";
if (mode === "emit") {
  const text = make(shape, n);
  writeFileSync(file, text);
  note = `${text.length} chars`;
} else {
  const text = readFileSync(file, "utf8");
  if (mode === "readTyped") [ms, note] = timed(() => String(readTyped(shape, text)));
  else if (mode === "writeTyped") [ms, note] = timed(() => String(writeTyped(shape, text)));
  else if (mode === "readValue") [ms, note] = timed(() => typeof readValue(text));
  else if (mode === "writeValue") {
    const value = readValue(text);
    [ms, note] = timed(() => String(writeValue(value).length));
  } else throw new Error(mode);
}
console.log(`${shape} ${mode} n=${n} ${ms.toFixed(1)} ms (${note})`);
