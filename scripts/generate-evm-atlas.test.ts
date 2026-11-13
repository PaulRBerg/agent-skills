import { expect, test } from "bun:test";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

type FixtureOptions = {
  blockscout?: Record<string, unknown>;
  categories?: Array<string | undefined>;
  etherscanApiUnsupportedFrom?: string;
  etherscanSupport?: "free" | "paid" | "unsupported";
  explorerApiUrl?: string;
  schemaVersion?: number;
};

function createFixture({
  blockscout,
  categories = ["mainnet", "zk"],
  etherscanApiUnsupportedFrom,
  etherscanSupport = "unsupported",
  explorerApiUrl,
  schemaVersion = 3,
}: FixtureOptions = {}) {
  const root = mkdtempSync(path.join(tmpdir(), "atlas-generator-"));
  const script = path.join(root, "scripts/generate-evm-atlas.ts");
  const references = path.join(root, "skills/evm-atlas/references");
  const registry = path.join(root, "registry");
  for (const directory of [
    path.dirname(script),
    references,
    path.join(root, "skills/evm-atlas/scripts"),
    path.join(registry, "data"),
  ]) {
    mkdirSync(directory, { recursive: true });
  }
  copyFileSync(path.join(import.meta.dir, "generate-evm-atlas.ts"), script);
  symlinkSync(path.resolve(import.meta.dir, "../node_modules"), path.join(root, "node_modules"));
  writeFileSync(path.join(registry, "package.json"), JSON.stringify({ name: "@prb/crypto-registry" }));

  const chains = [
    { slug: "standard", addressUrl: "https://explorer.example/address/{address}" },
    { slug: "split", addressUrl: "https://portfolio.example/{address}/history?chain=split" },
  ].map(({ slug, addressUrl }, index) => ({
    slug,
    name: slug,
    aliases: [],
    chainId: index + 1,
    ...(categories[index] === undefined ? {} : { category: categories[index] }),
    accountActivityModel: "ethereum-eoa",
    ...(slug === "standard" && etherscanApiUnsupportedFrom ? { etherscanApiUnsupportedFrom } : {}),
    nativeCurrency: { symbol: "ETH" },
    explorer: {
      addressUrl,
      txUrl: "https://explorer.example/tx/{tx_hash}",
      ...(slug === "standard" && explorerApiUrl ? { apiUrl: explorerApiUrl } : {}),
    },
    ...(slug === "split" ? { defunct: { finalStateBlock: 7, since: "2026-08-15" } } : {}),
  }));
  writeFileSync(path.join(registry, "data/chains.json"), JSON.stringify({ schemaVersion, chains }));
  writeFileSync(
    path.join(references, "atlas-overlays.json"),
    JSON.stringify({
      chains: Object.fromEntries(
        chains.map(({ slug }) => [
          slug,
          {
            primaryPublicRpc: "https://rpc.example",
            fallbackPublicRpcs: ["https://fallback.example"],
            routeMesh: false,
            etherscan: { support: etherscanSupport },
            blockscout: (slug === "standard" && blockscout) || { status: "absent" },
          },
        ]),
      ),
    }),
  );

  return {
    references,
    root,
    run: (mode = "--write") =>
      Bun.spawnSync([process.execPath, script, mode], {
        cwd: root,
        env: { ...process.env, CRYPTO_REGISTRY_DIR: registry },
      }),
  };
}

test("generator preserves chain categories, explorer templates, and defunct status", () => {
  const fixture = createFixture();
  try {
    for (const mode of ["--write", "--check"]) {
      const result = fixture.run(mode);
      expect(result.stderr.toString()).toBe("");
      expect(result.exitCode).toBe(0);
    }
    const output = JSON.parse(readFileSync(path.join(fixture.references, "generated/target-mainnets.json"), "utf8"));
    expect(output.chains).toMatchObject([
      {
        slug: "standard",
        category: "mainnet",
        explorerUrl: "https://explorer.example",
        explorerAddressUrl: "https://explorer.example/address/{address}",
        explorerTxUrl: "https://explorer.example/tx/{tx_hash}",
      },
      {
        slug: "split",
        category: "zk",
        explorerUrl: "https://explorer.example",
        explorerAddressUrl: "https://portfolio.example/{address}/history?chain=split",
        explorerTxUrl: "https://explorer.example/tx/{tx_hash}",
        defunct: { finalStateBlock: 7, since: "2026-08-15" },
      },
    ]);
    expect(output.chains[0]).not.toHaveProperty("defunct");
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("generator projects canonical Etherscan retirement into targets and dated table notes", () => {
  const fixture = createFixture({
    etherscanApiUnsupportedFrom: "2026-11-30",
    etherscanSupport: "free",
    explorerApiUrl: "https://native-api.example/api",
  });
  try {
    const result = fixture.run();
    expect(result.stderr.toString()).toBe("");
    expect(result.exitCode).toBe(0);
    const output = JSON.parse(readFileSync(path.join(fixture.references, "generated/target-mainnets.json"), "utf8"));
    expect(output.chains[0].etherscanApiUnsupportedFrom).toBe("2026-11-30");
    expect(output.chains[0].explorerApiUrl).toBe("https://native-api.example/api");
    expect(output.chains[1]).not.toHaveProperty("etherscanApiUnsupportedFrom");
    const table = readFileSync(path.join(fixture.references, "generated/etherscan-chains.md"), "utf8");
    expect(table).toContain("Etherscan API unsupported from 2026-11-30 (all plans and actions).");
    expect(table.split("## Target Mainnets (Paid Plan Required)")[0]).toContain("| standard | `1`");
    expect(table).toContain("current UTC execution date, not the historical checkpoint");
    const script = readFileSync(path.join(fixture.root, "skills/evm-atlas/scripts/resolve-chain.sh"), "utf8");
    expect(script).not.toContain("https://native-api.example/api");
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("generator retains registry API overrides for observed Blockscout instances", () => {
  const fixture = createFixture({
    blockscout: { status: "observed", hostedBy: "self", instanceUrl: "https://explorer.example/" },
    explorerApiUrl: "https://api-blockscout.example/api",
  });
  try {
    const result = fixture.run();
    expect(result.stderr.toString()).toBe("");
    expect(result.exitCode).toBe(0);
    const script = readFileSync(path.join(fixture.root, "skills/evm-atlas/scripts/resolve-chain.sh"), "utf8");
    expect(script).toContain("  1) instance='https://explorer.example/'; api='https://api-blockscout.example/api' ;;");
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("generator routes a Blockscout overlay API host into resolve-chain.sh", () => {
  const fixture = createFixture({
    blockscout: {
      status: "observed",
      hostedBy: "self",
      instanceUrl: "https://blockscout.example/",
      apiUrl: "https://api-blockscout.example/api",
    },
  });
  try {
    const result = fixture.run();
    expect(result.stderr.toString()).toBe("");
    expect(result.exitCode).toBe(0);
    const script = readFileSync(path.join(fixture.root, "skills/evm-atlas/scripts/resolve-chain.sh"), "utf8");
    expect(script).toContain(
      "  1) instance='https://blockscout.example/'; api='https://api-blockscout.example/api' ;;",
    );
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("generator rejects a missing or unknown chain category", () => {
  for (const category of [undefined, "sidechain"]) {
    const fixture = createFixture({ categories: [category, "zk"] });
    try {
      const result = fixture.run();
      expect(result.exitCode).toBe(1);
      expect(result.stderr.toString()).toContain(
        "standard.category must be mainnet, alt-l1, op-stack, nitro, zk, or alt-l2.",
      );
    } finally {
      rmSync(fixture.root, { recursive: true, force: true });
    }
  }
});

test("generator rejects obsolete chain artifact schemas", () => {
  const fixture = createFixture({ schemaVersion: 2 });
  try {
    const result = fixture.run();
    expect(result.exitCode).toBe(1);
    expect(result.stderr.toString()).toContain("Unsupported crypto-registry chains schema version: 2.");
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});
