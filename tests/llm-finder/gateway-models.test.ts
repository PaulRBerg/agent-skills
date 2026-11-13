import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { after, test } from "node:test";

const scratch = mkdtempSync(path.join(tmpdir(), "llm-finder-test-"));
const script = path.resolve(import.meta.dirname, "../../skills/llm-finder/scripts/gateway-models.ts");
const preload = path.join(scratch, "provider.mjs");
writeFileSync(
  preload,
  `
const base = {
  name: "Fixture", type: "language", regions: ["eu"], tags: ["vision", "structured-output"],
  modalities: { input: ["image", "text"], extra: "preserved" },
  pricing: { input: "0.000001", output: "0.000002", extra: "preserved" },
  extra: "preserved"
};
const catalog = [
  { ...base, id: "fixture/new", released: 20, zdr: "all" },
  { ...base, id: "fixture/older", released: 10, zdr: "some" },
  { ...base, id: "other/none", released: 30, zdr: "none" },
  { ...base, id: "fixture/old", released: 5, deprecated_at: 15 },
  { ...base, id: "fixture/embed", type: "embedding" },
  { ...base, id: "fixture/unpriced", pricing: undefined },
  { ...base, id: "fixture/decision", type: "evaluation", pricing: { input: "0.000001", output: "0" } }
];
globalThis.fetch = async (input, options) => {
  if (!options.signal) throw new Error("Missing bounded network signal");
  const url = String(input);
  if (process.env.SCENARIO === "http") return new Response("unavailable", { status: 503 });
  if (process.env.SCENARIO === "invalid-json") return new Response("{");
  let value = url.endsWith("/endpoints")
    ? { data: { name: "Fixture", endpoints: [{ provider_name: "fixture", status: 0, context_length: 1000,
        pricing: { prompt: "0.000001", completion: "0.000002" },
        latency_last_1h: null, throughput_last_1h: null }] } }
    : { data: catalog };
  if (process.env.SCENARIO === "malformed") value = { data: "invalid" };
  if (process.env.SCENARIO === "wrong-field") value = { data: [{ ...base, id: "fixture/bad", modalities: { input: "image" } }] };
  if (process.env.SCENARIO === "bad-price") value = { data: [{ ...base, id: "fixture/bad", pricing: { input: "NaN" } }] };
  if (process.env.SCENARIO === "bad-endpoint") value = { data: { name: "Fixture", endpoints: [{ provider_name: "fixture", status: "active", context_length: 1000 }] } };
  if (process.env.SCENARIO === "metrics") value.data.endpoints[0].latency_last_1h = { p50: 12, p95: 22 };
  return Response.json(value);
};
`,
);
after(() => rmSync(scratch, { force: true, recursive: true }));

function run(args: string[], scenario = "", runtime = "node") {
  return spawnSync(runtime, ["--import", preload, script, ...args], {
    cwd: scratch,
    encoding: "utf8",
    env: { ...process.env, SCENARIO: scenario },
    timeout: 10_000,
  });
}

test("filters catalog data, includes all/some ZDR and preserves unknown JSON fields", () => {
  const result = run([
    "--region",
    "eu",
    "--input",
    "image,text",
    "--tag",
    "vision,structured-output",
    "--zdr",
    "--creator",
    "fixture",
    "--match",
    "new,older",
    "--max-input",
    "1",
    "--max-output",
    "2",
    "--json",
  ]);
  assert.equal(result.status, 0, result.stderr);
  const rows = JSON.parse(result.stdout);
  assert.deepEqual(
    rows.map((row: { id: string }) => row.id),
    ["fixture/new", "fixture/older"],
  );
  assert.equal(rows[0].extra, "preserved");
  assert.equal(rows[0].pricing.extra, "preserved");
  assert.equal(rows[0].modalities.extra, "preserved");
});

test("defaults to nondeprecated language models sorted newest first", () => {
  const result = run(["--json"]);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(
    JSON.parse(result.stdout).map((row: { id: string }) => row.id),
    ["other/none", "fixture/new", "fixture/older", "fixture/unpriced"],
  );
  const deprecated = run(["--match", "fixture/old", "--include-deprecated", "--json"]);
  assert.equal(JSON.parse(deprecated.stdout).length, 2);
});

test("missing prices display unknown and fail a ceiling while zero prices remain valid", () => {
  const table = run(["--match", "unpriced"]);
  assert.equal(table.status, 0, table.stderr);
  assert.match(table.stdout, /fixture\/unpriced\s+-\s+-\s+-\s+-\s+-/u);
  const filtered = run(["--match", "unpriced", "--max-input", "2", "--json"]);
  assert.deepEqual(JSON.parse(filtered.stdout), []);
  const zero = run(["--type", "evaluation", "--max-output", "0", "--json"]);
  assert.equal(JSON.parse(zero.stdout)[0].id, "fixture/decision");
});

test("endpoint output preserves prices and accepts null or populated metrics", () => {
  const result = run(["--endpoints", "fixture/new"]);
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout, /fixture\s+active\s+1000\s+-\s+1.00\s+2.00/u);
  const metrics = run(["--endpoints", "fixture/new"], "metrics");
  assert.equal(metrics.status, 0, metrics.stderr);
  assert.match(metrics.stdout, /12\s+22/u);
});

test("bad CLI options and invalid prices exit 2 before fetching", () => {
  for (const args of [
    ["--unknown", "x"],
    ["--region"],
    ["positional"],
    ["--endpoints", "invalid"],
    ...["NaN", "Infinity", "-1", "", "1x"].map((value) => ["--max-input", value]),
  ]) {
    const result = run(args, "http");
    assert.equal(result.status, 2, `${args.join(" ")}: ${result.stderr}`);
    assert.equal(result.stdout, "");
  }
  const result = run(["--max-output", "-2"]);
  assert.equal(result.status, 2, result.stderr);
});

test("malformed responses and HTTP failures exit 1 without a partial report", () => {
  for (const scenario of ["malformed", "wrong-field", "bad-price", "invalid-json", "http"]) {
    const result = run(["--json"], scenario);
    assert.equal(result.status, 1, scenario);
    assert.equal(result.stdout, "");
    assert.ok(result.stderr.length > 0);
  }
  const endpoints = run(["--endpoints", "fixture/new"], "bad-endpoint");
  assert.equal(endpoints.status, 1);
  assert.equal(endpoints.stdout, "");
});

test("Bun CLI also runs from outside a repository without dependencies", () => {
  const result = run(["--type", "embedding", "--json"], "", "bun");
  assert.equal(result.status, 0, result.stderr);
  assert.equal(JSON.parse(result.stdout)[0].id, "fixture/embed");
});
