import { scanTarget } from "./scanner.js";
import type { ScanTarget } from "./scanner.js";

const target = JSON.parse(Bun.argv[2] ?? "") as ScanTarget;
const records = scanTarget(target, (message, error) =>
  console.error(`[ai-coord-dashboard] ${message}`, error)
);

process.stdout.write(JSON.stringify(records));
