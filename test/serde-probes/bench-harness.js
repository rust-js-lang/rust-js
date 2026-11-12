// harness.js <n> <mode> [reps]: one operation per process, timed; peak memory is read by /usr/bin/time -l.
import { make, write, read, readValue, writeValue, count } from "./bench.js";
import { readFileSync, writeFileSync, existsSync } from "node:fs";
const n = +process.argv[2], mode = process.argv[3], reps = +(process.argv[4] ?? 1);
const file = `doc-${n}.json`;
const timed = (f) => { let best = Infinity, out; for (let i = 0; i < reps; i++) { const t = performance.now(); out = f(); best = Math.min(best, performance.now() - t); } return [best, out]; };
let ms = 0, note = "";
if (mode === "emit") { const text = write(make(n)); writeFileSync(file, text); note = `${text.length} chars`; }
else if (mode === "make") { const [t, doc] = timed(() => make(n)); ms = t; note = String(doc.items.length); }
else if (mode === "write") { const doc = make(n); const [t, text] = timed(() => write(doc)); ms = t; note = String(text.length); }
else {
  const text = readFileSync(file, "utf8");
  if (mode === "text") note = String(text.length);
  else if (mode === "read") { const [t, doc] = timed(() => read(text)); ms = t; note = String(count(doc)); }
  else if (mode === "parse") { const [t, doc] = timed(() => JSON.parse(text)); ms = t; note = String(doc.items.length); }
  else if (mode === "readValue") { const [t, v] = timed(() => readValue(text)); ms = t; note = typeof v; }
  else if (mode === "stringify") { const doc = JSON.parse(text); const [t, s] = timed(() => JSON.stringify(doc)); ms = t; note = String(s.length); }
  else if (mode === "writeValue") { const v = readValue(text); const [t, s] = timed(() => writeValue(v)); ms = t; note = String(s.length); }
  else throw new Error(mode);
}
console.log(`${mode} n=${n} ${ms.toFixed(1)} ms (${note})`);
