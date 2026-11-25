// Part 5 of the export net (test/export-net-parts.ts, ADR 1204).
import { beforeAll } from "bun:test";
import { exportNetTests } from "./export-net-parts";
import { buildCompiler } from "./support";

beforeAll(buildCompiler, 600_000);

exportNetTests(5);
