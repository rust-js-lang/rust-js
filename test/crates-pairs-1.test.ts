// Part 1 of the crate pairs' table (test/crates-pairs.ts, #2465).
import { beforeAll } from "bun:test";
import { pairTests } from "./crates-pairs";
import { buildCompiler } from "./support";

beforeAll(buildCompiler, 600_000);

pairTests(1);
