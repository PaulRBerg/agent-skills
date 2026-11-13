#!/usr/bin/env bun
// Query the public Vercel AI Gateway catalog and print a compact, filtered table.
// Run: bun run scripts/gateway-models.ts [options]
import process from "node:process";

const CATALOG_URL = "https://ai-gateway.vercel.sh/v1/models";
const PER_MILLION = 1_000_000;

type Model = {
  id: string;
  name: string;
  type: string;
  context_window?: number;
  deprecated_at?: number | null;
  description?: string;
  max_tokens?: number;
  modalities?: { input?: string[]; output?: string[] };
  pricing?: { input?: string; output?: string; input_cache_read?: string };
  regions?: string[] | null;
  released?: number;
  tags?: string[];
  zdr?: string;
};
type Percentiles = { p50?: number; p95?: number };
type Endpoint = {
  provider_name: string;
  status: number;
  context_length: number;
  max_completion_tokens?: number;
  pricing?: { prompt?: string; completion?: string };
  uptime_last_1d?: number;
  latency_last_1h?: Percentiles | null;
  throughput_last_1h?: Percentiles | null;
};

type Options = Record<string, string | boolean>;

type Filters = {
  type: string;
  match: string[];
  creators: string[];
  inputs: string[];
  tags: string[];
  region: string | null;
  zdr: boolean;
  includeDeprecated: boolean;
  maxInput: number | null;
  maxOutput: number | null;
};

const FLAGS = new Set(["zdr", "include-deprecated", "describe", "json"]);
const VALUES = new Set(["type", "match", "creator", "input", "tag", "region", "max-input", "max-output", "endpoints"]);

const USAGE = `Usage: bun run scripts/gateway-models.ts [options]

Lists Vercel AI Gateway models (public catalog, no auth). Default: non-deprecated language models,
newest first.

  --type <t>            language (default) | evaluation | embedding | reranking | image | ...
  --match <a,b>         Keep ids or names containing any listed substring (case-insensitive)
  --creator <a,b>       Keep only these creators (the part before "/")
  --input <a,b>         Require every listed input modality: text, image, pdf, audio, video
  --tag <a,b>           Require every listed tag, e.g. tool-use,structured-output,vision,reasoning
  --region <r>          Require the region in the model's regions list, e.g. eu
  --zdr                 Require zero data retention "all" or "some"
  --max-input <usd>     Max input price in USD per 1M tokens
  --max-output <usd>    Max output price in USD per 1M tokens
  --include-deprecated  Keep models with a deprecated_at date
  --describe            Add the catalog description to each row
  --endpoints <id>      Show per-provider uptime, latency, throughput, and price for one model id
  --json                Print the filtered models as JSON instead of a table
  -h, --help            Show this help`;

function fail(message: string, code: number): never {
  console.error(message);
  process.exit(code);
}

function parseArgs(argv: string[]): Options {
  const opts: Options = {};
  for (let i = 0; i < argv.length; i += 1) {
    const arg = argv[i] ?? "";
    if (arg === "-h" || arg === "--help") {
      console.log(USAGE);
      process.exit(0);
    }
    if (!arg.startsWith("--")) {
      fail(`Unexpected argument: ${arg}\n\n${USAGE}`, 2);
    }
    const key = arg.slice(2);
    if (FLAGS.has(key)) {
      opts[key] = true;
      continue;
    }
    if (!VALUES.has(key)) {
      fail(`Unknown option: --${key}\n\n${USAGE}`, 2);
    }
    const value = argv[i + 1];
    if (value === undefined || value.startsWith("--")) {
      fail(`Missing value for --${key}\n\n${USAGE}`, 2);
    }
    opts[key] = value;
    i += 1;
  }
  return opts;
}

function list(value: string | boolean | undefined): string[] {
  if (typeof value !== "string") {
    return [];
  }
  return value
    .split(",")
    .map((item) => item.trim().toLowerCase())
    .filter(Boolean);
}

function text(value: string | boolean | undefined): string | null {
  return typeof value === "string" ? value : null;
}

function price(value: string | boolean | undefined): number | null {
  const raw = text(value);
  if (raw === null) return null;
  const amount = Number(raw);
  if (raw.trim() === "" || !Number.isFinite(amount) || amount < 0) {
    fail(`Invalid price: ${raw}. Expected a finite, nonnegative USD amount per 1M tokens.`, 2);
  }
  return amount;
}

function perMillion(value: string | undefined): string {
  if (value === undefined) {
    return "-";
  }
  const n = Number(value) * PER_MILLION;
  if (!Number.isFinite(n)) {
    return "-";
  }
  if (n >= 100) {
    return n.toFixed(0);
  }
  if (n >= 10) {
    return n.toFixed(1);
  }
  return n.toFixed(2);
}

function isoDate(unix: number | undefined | null): string {
  return unix ? new Date(unix * 1000).toISOString().slice(0, 10) : "-";
}

// Validate every consumed external field without stripping unknown catalog fields.
function object(value: unknown, path: string): asserts value is Record<string, unknown> {
  if (typeof value !== "object" || value === null || Array.isArray(value)) {
    throw new Error(`Invalid catalog response: ${path} must be an object`);
  }
}
function field(value: unknown, kind: "string" | "number" | "strings" | "price", path: string) {
  const valid =
    kind === "strings"
      ? Array.isArray(value) && value.every((item) => typeof item === "string")
      : kind === "price"
        ? typeof value === "string" && value.trim() !== "" && Number.isFinite(Number(value)) && Number(value) >= 0
        : typeof value === kind && (kind !== "number" || Number.isFinite(value));
  if (!valid) throw new Error(`Invalid catalog response: ${path} must be ${kind}`);
}
function optionalFields(
  value: Record<string, unknown>,
  fields: Record<string, "string" | "number" | "strings" | "price">,
  path: string,
) {
  for (const [key, kind] of Object.entries(fields)) {
    if (value[key] !== undefined) field(value[key], kind, `${path}.${key}`);
  }
}
function pricing(value: unknown, keys: string[], path: string) {
  object(value, path);
  for (const key of keys) if (value[key] !== undefined) field(value[key], "price", `${path}.${key}`);
}
function model(value: unknown, path: string): asserts value is Model {
  object(value, path);
  for (const key of ["id", "name", "type"]) field(value[key], "string", `${path}.${key}`);
  optionalFields(
    value,
    {
      context_window: "number",
      description: "string",
      max_tokens: "number",
      released: "number",
      tags: "strings",
      zdr: "string",
    },
    path,
  );
  if (value.deprecated_at !== undefined && value.deprecated_at !== null)
    field(value.deprecated_at, "number", `${path}.deprecated_at`);
  if (value.regions !== undefined && value.regions !== null) field(value.regions, "strings", `${path}.regions`);
  if (value.pricing !== undefined) pricing(value.pricing, ["input", "output", "input_cache_read"], `${path}.pricing`);
  if (value.modalities !== undefined) {
    object(value.modalities, `${path}.modalities`);
    optionalFields(value.modalities, { input: "strings", output: "strings" }, `${path}.modalities`);
  }
}
function endpoint(value: unknown, path: string): asserts value is Endpoint {
  object(value, path);
  field(value.provider_name, "string", `${path}.provider_name`);
  for (const key of ["status", "context_length"]) field(value[key], "number", `${path}.${key}`);
  optionalFields(value, { max_completion_tokens: "number", uptime_last_1d: "number" }, path);
  if (value.pricing !== undefined) pricing(value.pricing, ["prompt", "completion"], `${path}.pricing`);
  for (const key of ["latency_last_1h", "throughput_last_1h"]) {
    const metrics = value[key];
    if (metrics !== undefined && metrics !== null) {
      object(metrics, `${path}.${key}`);
      optionalFields(metrics, { p50: "number", p95: "number" }, `${path}.${key}`);
    }
  }
}
async function fetchJson(url: string): Promise<unknown> {
  const response = await fetch(url, {
    headers: { accept: "application/json" },
    signal: AbortSignal.timeout(20_000),
  });
  if (!response.ok) throw new Error(`${url} returned HTTP ${response.status}`);
  return response.json();
}
function modelsResponse(value: unknown): Model[] {
  object(value, "response");
  if (!Array.isArray(value.data)) throw new Error("Invalid catalog response: data must be an array");
  return value.data.map((entry: unknown, i: number) => {
    model(entry, `data[${i}]`);
    return entry;
  });
}
function endpointsResponse(value: unknown): { name: string; endpoints: Endpoint[] } {
  object(value, "response");
  object(value.data, "data");
  const name = value.data.name;
  if (typeof name !== "string") throw new Error("Invalid catalog response: data.name must be string");
  if (!Array.isArray(value.data.endpoints))
    throw new Error("Invalid catalog response: data.endpoints must be an array");
  const endpoints = value.data.endpoints.map((entry: unknown, i: number) => {
    endpoint(entry, `data.endpoints[${i}]`);
    return entry;
  });
  return { name, endpoints };
}

function printTable(rows: string[][]) {
  const widths = (rows[0] ?? []).map((_, col) => Math.max(...rows.map((row) => (row[col] ?? "").length)));
  for (const row of rows) {
    console.log(
      row
        .map((cell, col) => cell.padEnd(widths[col] ?? 0))
        .join("  ")
        .trimEnd(),
    );
  }
}

async function showEndpoints(id: string) {
  const [creator, ...rest] = id.split("/");
  if (!creator || rest.length === 0 || rest.some((part) => !part) || !/^[a-zA-Z0-9._/-]+$/u.test(id)) {
    fail(`Model id must look like creator/model: ${id}`, 2);
  }
  const data = endpointsResponse(await fetchJson(`${CATALOG_URL}/${creator}/${rest.join("/")}/endpoints`));
  console.log(`${id}  ${data.name}`);
  const rows = [
    ["provider", "status", "ctx", "max_out", "in$/M", "out$/M", "uptime1d", "ttft_p50ms", "ttft_p95ms", "tok/s_p50"],
  ];
  for (const endpoint of data.endpoints) {
    rows.push([
      endpoint.provider_name,
      endpoint.status === 0 ? "active" : String(endpoint.status),
      String(endpoint.context_length ?? "-"),
      String(endpoint.max_completion_tokens ?? "-"),
      perMillion(endpoint.pricing?.prompt),
      perMillion(endpoint.pricing?.completion),
      endpoint.uptime_last_1d === undefined ? "-" : `${endpoint.uptime_last_1d}%`,
      String(endpoint.latency_last_1h?.p50 ?? "-"),
      String(endpoint.latency_last_1h?.p95 ?? "-"),
      String(endpoint.throughput_last_1h?.p50 ?? "-"),
    ]);
  }
  printTable(rows);
}

function filtersFrom(opts: Options): Filters {
  return {
    creators: list(opts.creator),
    includeDeprecated: opts["include-deprecated"] === true,
    inputs: list(opts.input),
    match: list(opts.match),
    maxInput: price(opts["max-input"]),
    maxOutput: price(opts["max-output"]),
    region: text(opts.region)?.toLowerCase() ?? null,
    tags: list(opts.tag),
    type: text(opts.type) ?? "language",
    zdr: opts.zdr === true,
  };
}

function withinPrice(value: string | undefined, ceiling: number | null): boolean {
  return ceiling === null || Number(value ?? Number.POSITIVE_INFINITY) * PER_MILLION <= ceiling;
}

function matches(m: Model, f: Filters): boolean {
  const haystack = `${m.id} ${m.name}`.toLowerCase();
  return (
    m.type === f.type &&
    (f.includeDeprecated || !m.deprecated_at) &&
    (f.match.length === 0 || f.match.some((s) => haystack.includes(s))) &&
    (f.creators.length === 0 || f.creators.includes((m.id.split("/")[0] ?? "").toLowerCase())) &&
    f.inputs.every((i) => (m.modalities?.input ?? []).includes(i)) &&
    f.tags.every((t) => (m.tags ?? []).includes(t)) &&
    (f.region === null || (m.regions ?? []).includes(f.region)) &&
    (!f.zdr || m.zdr === "all" || m.zdr === "some") &&
    withinPrice(m.pricing?.input, f.maxInput) &&
    withinPrice(m.pricing?.output, f.maxOutput)
  );
}

function toRow(m: Model, describe: boolean): string[] {
  const row = [
    m.id,
    isoDate(m.released),
    String(m.context_window ?? "-"),
    String(m.max_tokens ?? "-"),
    perMillion(m.pricing?.input),
    perMillion(m.pricing?.output),
    (m.modalities?.input ?? []).join("+") || "-",
    m.zdr ?? "-",
    (m.regions ?? []).join("+") || "-",
    (m.tags ?? []).join(",") || "-",
  ];
  if (describe) {
    row.push((m.description ?? "").replaceAll(/\s+/gu, " "));
  }
  return row;
}

async function listModels(opts: Options) {
  const filters = filtersFrom(opts);
  const data = modelsResponse(await fetchJson(CATALOG_URL));
  const models = data.filter((m) => matches(m, filters)).toSorted((a, b) => (b.released ?? 0) - (a.released ?? 0));

  if (opts.json) {
    console.log(JSON.stringify(models, null, 2));
    return;
  }
  if (models.length === 0) {
    console.log(`No ${filters.type} models match the filters.`);
    return;
  }
  const describe = opts.describe === true;
  const header = ["id", "released", "ctx", "max_out", "in$/M", "out$/M", "inputs", "zdr", "regions", "tags"];
  if (describe) {
    header.push("description");
  }
  printTable([header, ...models.map((m) => toRow(m, describe))]);
  console.log(`\n${models.length} ${filters.type} model(s). Prices are USD per 1M tokens from the public catalog.`);
}

const opts = parseArgs(process.argv.slice(2));
const endpoints = text(opts.endpoints);
// Validate price options before either network branch.
filtersFrom(opts);
try {
  if (endpoints === null) await listModels(opts);
  else await showEndpoints(endpoints);
} catch (error) {
  fail(error instanceof Error ? error.message : String(error), 1);
}
