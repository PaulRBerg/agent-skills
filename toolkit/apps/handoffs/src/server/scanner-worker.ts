import { scanTarget, type ScanTarget } from "./scanner";

const target = JSON.parse(Bun.argv[2] ?? "") as ScanTarget;
const records = scanTarget(target, (message, error) => console.error(`[ai-handoffs] ${message}`, error));

process.stdout.write(JSON.stringify(records));
