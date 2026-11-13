import { execFileSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";

export type CommittedSkillSource = { files: string[]; head: string; root: string };

// Read objects directly: archive attributes and checkout filters must not change committed content.
export function withCommittedSkillSource<T>(repository: string, inspect: (source: CommittedSkillSource) => T): T {
  const git = (args: string[], input?: string): Buffer =>
    execFileSync("git", args, {
      cwd: repository,
      input,
      maxBuffer: 256 * 1024 * 1024,
      stdio: ["pipe", "pipe", "pipe"],
    });
  const head = git(["rev-parse", "--verify", "HEAD^{commit}"]).toString("utf8").trim();
  const entries = git(["ls-tree", "-rz", "--full-tree", head, "--", "skills"])
    .toString("utf8")
    .split("\0")
    .filter(Boolean)
    .map((entry) => {
      const match = /^(100644|100755|120000) blob ([a-f0-9]+)\t([\s\S]+)$/.exec(entry);
      if (!match) throw new Error("Unsupported committed skill entry.");
      const mode = match[1]!;
      const oid = match[2]!;
      const name = match[3]!;
      if (!name.startsWith("skills/") || name.split("/").some((part) => part === ".." || part === ".")) {
        throw new Error("Unsafe committed skill path.");
      }
      return { mode, name, oid };
    });
  const root = fs.mkdtempSync(path.join(os.tmpdir(), "committed-skill-source-"));
  try {
    fs.mkdirSync(path.join(root, "skills"));
    const objects = git(["cat-file", "--batch"], entries.map(({ oid }) => `${oid}\n`).join(""));
    let offset = 0;
    for (const { mode, name, oid } of entries) {
      const end = objects.indexOf(10, offset);
      const header = objects.subarray(offset, end).toString("utf8");
      const match = /^([a-f0-9]+) blob ([0-9]+)$/.exec(header);
      if (end < offset || !match || match[1] !== oid) throw new Error("Invalid committed skill object.");
      const size = Number(match[2]);
      const content = objects.subarray(end + 1, end + 1 + size);
      offset = end + 1 + size + 1;
      if (content.length !== size || objects[offset - 1] !== 10) throw new Error("Truncated committed skill object.");
      const destination = path.join(root, name);
      fs.mkdirSync(path.dirname(destination), { recursive: true });
      if (mode === "120000") fs.symlinkSync(content.toString("utf8"), destination);
      else {
        fs.writeFileSync(destination, content);
        fs.chmodSync(destination, mode === "100755" ? 0o755 : 0o644);
      }
    }
    return inspect({ files: entries.map(({ name }) => name), head, root });
  } finally {
    fs.rmSync(root, { force: true, recursive: true });
  }
}
