// A WebIDL promise's payload that may be `null`, `Promise<GPUAdapter?>`,
// is an `Option` of it awaited (#1768): its `None` the `null` JS gives, so
// a program asks it, `.is_some()`, as it would of any nullable result
// (ADR 0030), where the payload was typed as always there.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a promise's payload that may be null is an Option awaited", async () => {
  expect(readFileSync(join(import.meta.dir, "..", "webapi", "src", "lib.rs"), "utf8")).toContain(
    "pub fn request_adapter(&self, options: Option<GPURequestAdapterOptions<'_>>) -> Promise<Option<&'static GPUAdapter>> {",
  );
  const dir = fixture("webapi-promises");
  writeFileSync(
    join(dir, "lib.rs"),
    `pub async fn available(gpu: &'static webapi::GPU) -> bool {
    gpu.request_adapter(None).await.is_some()
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const lib = await import(join(dir, "lib.js"));
  const gpu = (adapter: unknown) => ({ requestAdapter: () => Promise.resolve(adapter) });
  expect([await lib.available(gpu({ features: new Set() })), await lib.available(gpu(null))]).toEqual([true, false]);
});
