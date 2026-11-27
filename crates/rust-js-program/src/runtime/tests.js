
// A test file's `test` (ADR 1076), its runner's `register` asked to run
// each test as `$testRuns` does. An ignored one, `test.skip`, is its
// runner's skip.
function $tests(register) {
  const test = (name, f) => register(name, () => $testRuns(name, f));
  test.skip = register.skip;
  return test;
}

// A test, `f`, by its name: run, unless a refusal at
// run time came before it in this process, after which nothing of the
// program runs as Rust's would (`$refused`, ADR 0692, #2496), as no drop
// ran past it: refused too, and, where `rust-js test` runs it, its name
// written to the file `RUST_JS_TEST_ALONE` names, a line of JSON each, for
// `rust-js test` to run it again in a process of its own (#2625).
function $testRuns(name, f) {
  if (!$refused) return f();
  const alone = globalThis.process?.env?.RUST_JS_TEST_ALONE;
  if (alone) $nodeFs("`rust-js test`").appendFileSync(alone, JSON.stringify({ name }) + "\n");
  throw new $Unsupported("rust-js does not support running a test after one refused at run time in its process: `rust-js test` runs it again in a process of its own");
}
