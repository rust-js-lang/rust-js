// Runs a native Rust case built for `wasm32-wasip1` (`//@ native: wasm32`):
// its stdout and stderr as the program writes them, and its outcome, written
// where the host's wrapper writes it, `{"value":null}` where `main` returns.
// wasm32-wasip1 aborts on a panic, so a panicking one ends without one.
import { WASI } from "node:wasi";
import { readFileSync, writeFileSync } from "node:fs";

const [wasm, outcomeFile] = process.argv.slice(2);
const wasi = new WASI({ version: "preview1", args: [wasm], env: {} });
const module = await WebAssembly.compile(readFileSync(wasm));
const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
const code = wasi.start(instance);
if (code === 0 || code === undefined) writeFileSync(outcomeFile, '{"value":null}');
process.exit(code ?? 0);
