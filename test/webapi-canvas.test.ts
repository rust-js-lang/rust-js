// A canvas's context by its id's type (#1698), as TypeScript's `getContext`
// overloads type it: `canvas.get_context(contexts::Context2d, None)` is a
// `CanvasRenderingContext2D`, `canvas.getContext("2d")`, no `instanceof`
// of a class another realm's context isn't; `None` of JS's `null`, where
// the canvas has another kind of context already. An `OffscreenCanvas`'s
// the same, its own context. Another id is `get_context_named`'s, the
// union it was.
import { beforeAll, expect, test } from "bun:test";
import { readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { buildReact, compiler, fixture, run, target } from "./support";

beforeAll(buildReact, 600_000);

const withWeb = ["--", "--extern", `webapi=${join(target, "libwebapi.rmeta")}`, "--extern", `js=${join(target, "libjs.rmeta")}`, "-L", target];

test("a canvas's context is typed by its id, and is JS's getContext as it is", async () => {
  const dir = fixture("webapi-canvas");
  writeFileSync(
    join(dir, "lib.rs"),
    `use webapi::contexts::{Context2d, Webgl2, Webgpu};
use webapi::{CanvasRenderingContext2DSettings, HTMLCanvasElement, OffscreenCanvas};

pub fn draw(canvas: &'static HTMLCanvasElement) -> bool {
    if let Some(ctx) = canvas.get_context(Context2d, None) {
        ctx.fill_rect(0.0, 0.0, 100.0, 100.0);
        return true;
    }
    false
}

pub fn opaque(canvas: &'static HTMLCanvasElement) -> bool {
    let options = CanvasRenderingContext2DSettings {
        alpha: Some(false),
        ..Default::default()
    };
    canvas.get_context(Context2d, Some(options)).is_some()
}

pub fn three_d(canvas: &'static HTMLCanvasElement) -> bool {
    canvas.get_context(Webgl2, None).is_some()
}

pub fn gpu(canvas: &'static HTMLCanvasElement) -> bool {
    canvas.get_context(Webgpu, None).is_some()
}

pub fn offscreen(canvas: &'static OffscreenCanvas) -> bool {
    match canvas.get_context(Context2d, None) {
        Some(ctx) => {
            ctx.fill_rect(1.0, 2.0, 3.0, 4.0);
            true
        }
        None => false,
    }
}
`,
  );
  run([compiler, join(dir, "lib.rs"), "-o", join(dir, "lib.js"), ...withWeb]);
  const js = readFileSync(join(dir, "lib.js"), "utf8");
  expect(js).toContain('const ctx = canvas.getContext("2d");');
  expect(js).toContain('  const options = { alpha: false };\n  return canvas.getContext("2d", options) !== null;\n');
  expect(js).toContain('canvas.getContext("webgl2")');
  expect(js).toContain('canvas.getContext("webgpu")');
  expect(js).not.toContain("instanceof");
  // A canvas of no constructor global, as another realm's is: what it gives
  // is what Rust gets, its context or `null`.
  const lib = await import(join(dir, "lib.js"));
  const calls: unknown[][] = [];
  const canvas = (gives: unknown) => ({
    getContext: (...args: unknown[]) => {
      calls.push(args);
      return gives;
    },
  });
  const ctx = { fillRect: (...args: number[]) => calls.push(["fillRect", ...args]) };
  expect([lib.draw(canvas(ctx)), lib.draw(canvas(null)), lib.opaque(canvas(ctx)), lib.threeD(canvas(null)), lib.gpu(canvas({})), lib.offscreen(canvas(ctx))]).toEqual([true, false, true, false, true, true]);
  expect(calls).toEqual([["2d"], ["fillRect", 0, 0, 100, 100], ["2d"], ["2d", { alpha: false }], ["webgl2"], ["webgpu"], ["2d"], ["fillRect", 1, 2, 3, 4]]);
});
