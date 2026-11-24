// Runs a native Rust case built for `wasm32-wasip1` (`//@ native: wasm32`):
// its stdout and stderr as the program writes them, and its status the
// process's, `0` where `main` returns. wasm32-wasip1 aborts on a panic, so
// a panicking one ends otherwise.
import { WASI } from "node:wasi";
import { readFileSync } from "node:fs";

const [wasm] = process.argv.slice(2);
const wasi = new WASI({ version: "preview1", args: [wasm], env: {} });
const module = await WebAssembly.compile(readFileSync(wasm));
const instance = await WebAssembly.instantiate(module, { wasi_snapshot_preview1: wasi.wasiImport });
const code = wasi.start(instance);
process.exit(code ?? 0);
