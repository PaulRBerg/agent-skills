import { expect, test } from "bun:test";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

type FixtureOptions = {
  blockscout?: Record<string, unknown>;
  categories?: Array<string | undefined>;
  schemaVersion?: number;
};

function createFixture({ blockscout, categories = ["mainnet", "zk"], schemaVersion = 3 }: FixtureOptions = {}) {
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
    nativeCurrency: { symbol: "ETH" },
    explorer: { addressUrl, txUrl: "https://explorer.example/tx/{tx_hash}" },
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
            etherscan: { support: "unsupported" },
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

test("generator preserves chain categories and independent explorer templates", () => {
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
      },
    ]);
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

test("generator rejects a missing chain category", () => {
  const fixture = createFixture({ categories: [undefined, "zk"] });
  try {
    const result = fixture.run();
    expect(result.exitCode).toBe(1);
    expect(result.stderr.toString()).toContain(
      "standard.category must be mainnet, alt-l1, op-stack, nitro, zk, or alt-l2.",
    );
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
  }
});

test("generator rejects an unknown chain category", () => {
  const fixture = createFixture({ categories: ["sidechain", "zk"] });
  try {
    const result = fixture.run();
    expect(result.exitCode).toBe(1);
    expect(result.stderr.toString()).toContain(
      "standard.category must be mainnet, alt-l1, op-stack, nitro, zk, or alt-l2.",
    );
  } finally {
    rmSync(fixture.root, { recursive: true, force: true });
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
