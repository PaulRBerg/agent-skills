import { expect, test } from "bun:test";
import { copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, symlinkSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";

test("generator preserves independent address and transaction explorer templates", () => {
  const root = mkdtempSync(path.join(tmpdir(), "atlas-generator-"));
  try {
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
      accountActivityModel: "ethereum-eoa",
      nativeCurrency: { symbol: "ETH" },
      explorer: { addressUrl, txUrl: "https://explorer.example/tx/{tx_hash}" },
    }));
    writeFileSync(path.join(registry, "data/chains.json"), JSON.stringify({ schemaVersion: 2, chains }));
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
              blockscout: { status: "absent" },
            },
          ]),
        ),
      }),
    );
    for (const mode of ["--write", "--check"]) {
      const result = Bun.spawnSync([process.execPath, script, mode], {
        cwd: root,
        env: { ...process.env, CRYPTO_REGISTRY_DIR: registry },
      });
      expect(result.stderr.toString()).toBe("");
      expect(result.exitCode).toBe(0);
    }
    const output = JSON.parse(readFileSync(path.join(references, "generated/target-mainnets.json"), "utf8"));
    expect(output.chains).toMatchObject([
      {
        slug: "standard",
        explorerUrl: "https://explorer.example",
        explorerAddressUrl: "https://explorer.example/address/{address}",
        explorerTxUrl: "https://explorer.example/tx/{tx_hash}",
      },
      {
        slug: "split",
        explorerUrl: "https://explorer.example",
        explorerAddressUrl: "https://portfolio.example/{address}/history?chain=split",
        explorerTxUrl: "https://explorer.example/tx/{tx_hash}",
      },
    ]);
  } finally {
    rmSync(root, { recursive: true, force: true });
  }
});
