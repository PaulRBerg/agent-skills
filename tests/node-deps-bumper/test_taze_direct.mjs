import assert from "node:assert/strict";
import { mkdtemp, mkdir, writeFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import {
  directOptions,
  checkConfiguration,
  writeDirect,
} from "../../skills/node-deps-bumper/scripts/write-taze-direct.mjs";

test("direct writer preserves selection, recursion, maturity and fixed/peer defaults", async () => {
  const options = directOptions(
    [
      "major",
      "--force",
      "--no-github-actions",
      "--no-node-version",
      "-r",
      "--include",
      "react,effect",
      "--concurrency",
      "4",
      "--maturity-period",
      "7",
      "--maturity-period-exclude",
      "effect",
      "--write",
    ],
    "/fixture",
  );
  assert.deepEqual(options.include, ["react", "effect"]);
  assert.deepEqual(options.maturityPeriodExclude, ["effect"]);
  assert.equal(options.maturityPeriod, 7);
  assert.equal(options.concurrency, 4);
  assert.equal(options.recursive, true);
  assert.equal(options.peer, false);
  assert.equal(options.depFields.peerDependencies, false);
  assert.equal(options.includeLocked, false);
  assert.equal(options.ignoreOtherWorkspaces, true);
  assert.equal(options.githubActions, false);
  assert.equal(options.nodeVersion, false);
  await writeDirect(
    {
      async CheckPackages(received, hooks) {
        assert.equal(received, options);
        assert.equal(hooks.beforePackageWrite({ type: "bun-workspace" }), false);
        assert.equal(hooks.beforePackageWrite({ type: "package.json" }), true);
        return { packages: [] };
      },
    },
    options,
  );
});

test("refuses unknown options, missing selection and resolution failures", async () => {
  for (const args of [[], ["--include", "react", "--peer"], ["--include", "react", "--concurrency", "-1"]])
    assert.throws(() => directOptions(args, "/fixture"));
  await assert.rejects(writeDirect({}, {}), /CheckPackages/);
  await assert.rejects(
    writeDirect(
      {
        async CheckPackages() {
          return { packages: [{ resolved: [{ resolveError: "registry unavailable" }] }] };
        },
      },
      {},
    ),
    /resolution errors/,
  );
});

test("fails closed for local and ancestor configuration inputs", async () => {
  const root = await mkdtemp(join(tmpdir(), "taze-direct-config-"));
  const child = join(root, "child");
  await mkdir(child);
  try {
    await checkConfiguration(child);
    for (const name of ["taze.config.ts", ".tazerc.json", "pnpm-workspace.yaml", ".yarnrc.yml"]) {
      await writeFile(join(root, name), "fixture");
      await assert.rejects(checkConfiguration(child), /cannot honor/);
      await rm(join(root, name));
    }
    await writeFile(join(root, "package.json"), JSON.stringify({ packageManager: "pnpm@11.0.0" }));
    await assert.rejects(checkConfiguration(child), /cannot infer/);
  } finally {
    await rm(root, { recursive: true, force: true });
  }
});
