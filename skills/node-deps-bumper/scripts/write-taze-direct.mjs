#!/usr/bin/env node
// Use Taze's public API write hook to keep Bun catalogs and peer declarations unchanged.
import { readFile, readdir, realpath } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export function directOptions(args, cwd) {
  const options = {
    cwd,
    mode: "major",
    force: true,
    write: true,
    recursive: false,
    githubActions: false,
    nodeVersion: false,
    peer: false,
    includeLocked: false,
    ignoreOtherWorkspaces: true,
    depFields: { peerDependencies: false },
  };
  const ignored = new Set(["major", "--force", "--no-github-actions", "--no-node-version", "--write"]);
  const values = new Map([
    ["--include", "include"],
    ["--concurrency", "concurrency"],
    ["--maturity-period", "maturityPeriod"],
    ["--maturity-period-exclude", "maturityPeriodExclude"],
  ]);
  for (let index = 0; index < args.length; index++) {
    const argument = args[index];
    if (ignored.has(argument)) continue;
    if (argument === "-r") {
      options.recursive = true;
      continue;
    }
    const key = values.get(argument);
    if (!key || !args[index + 1]) throw new Error(`unsupported direct-only option: ${argument}`);
    const value = args[++index];
    if (key === "concurrency" || key === "maturityPeriod") {
      if (!/^\d+$/.test(value) || (key === "concurrency" && Number(value) < 1))
        throw new Error(`invalid direct-only option: ${argument}`);
      options[key] = Number(value);
    } else options[key] = value.split(",").filter(Boolean);
  }
  if (!options.include?.length) throw new Error("direct-only write requires an explicit include selection");
  return options;
}

export async function checkConfiguration(cwd) {
  // CLI config resolution is private. Refuse its inputs instead of silently dropping policies.
  for (let directory = resolve(cwd); ; directory = dirname(directory)) {
    const names = await readdir(directory);
    if (
      names.some(
        (name) =>
          /^taze\.config(?:\.|$)/.test(name) ||
          /^\.tazerc(?:\.|$)/.test(name) ||
          ["pnpm-workspace.yaml", ".yarnrc.yml"].includes(name),
      )
    )
      throw new Error("direct-only write cannot honor Taze or pnpm/Yarn configuration; no files written");
    if (names.includes("package.json")) {
      const manifest = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
      const manager = manifest.packageManager || manifest.devEngines?.packageManager?.name;
      if (/^(pnpm|yarn)(@|$)/.test(manager || ""))
        throw new Error("direct-only write cannot infer pnpm/Yarn policies; no files written");
    }
    if (dirname(directory) === directory) break;
  }
}

export async function writeDirect(api, options) {
  if (typeof api.CheckPackages !== "function") throw new Error("installed Taze does not expose CheckPackages");
  const result = await api.CheckPackages(options, {
    beforePackageWrite: (pkg) => pkg.type !== "bun-workspace",
  });
  const failed = result.packages
    .flatMap((pkg) => pkg.resolved || [])
    .filter((dep) => dep.resolveError || dep.diff === "error");
  if (failed.length) throw new Error("Taze reported dependency resolution errors; inspect selected dependencies");
  return result;
}

export async function main(argv) {
  const [executable, ...args] = argv;
  const options = directOptions(args, process.cwd());
  await checkConfiguration(options.cwd);
  let directory = dirname(await realpath(executable));
  while (true) {
    try {
      const manifest = JSON.parse(await readFile(join(directory, "package.json"), "utf8"));
      if (manifest.name === "taze") {
        // The hook and manifest type are verified against this installed public API.
        const version = /^(\d+)\.(\d+)\.(\d+)$/.exec(manifest.version);
        if (!version || Number(version[1]) !== 21 || Number(version[2]) < 3)
          throw new Error("direct-only write requires Taze >=21.3.0 <22 with the public beforePackageWrite hook");
        const api = await import(pathToFileURL(join(directory, "dist/index.mjs")).href);
        await writeDirect(api, options);
        console.log("Selected direct dependency updates applied; Bun catalogs and peer declarations preserved.");
        return;
      }
    } catch (error) {
      if (error.code !== "ENOENT") throw error;
    }
    if (dirname(directory) === directory) throw new Error("cannot locate installed Taze API from executable");
    directory = dirname(directory);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  main(process.argv.slice(2)).catch((error) => {
    console.error(`ERROR: ${error.message}`);
    process.exitCode = 64;
  });
}
