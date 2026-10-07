import tsParser from "@typescript-eslint/parser";
import { Linter } from "eslint";
import { describe, expect, test } from "vitest";

import config from "../eslint.config.js";
import classNamesPlugin from "./eslint-class-names.js";

const ruleId = "local/no-classname-concatenation";
const interpolationStart = "${";

function lint(code: string) {
  const linter = new Linter();
  return linter.verify(
    code,
    {
      files: ["**/*.tsx"],
      languageOptions: {
        ecmaVersion: "latest",
        parser: tsParser,
        parserOptions: { ecmaFeatures: { jsx: true } },
        sourceType: "module",
      },
      plugins: { local: classNamesPlugin },
      rules: { [ruleId]: "error" },
    },
    { filename: "fixture.tsx" }
  );
}

function messageIds(code: string): (string | undefined)[] {
  return lint(code).map((message) => message.messageId);
}

describe("local/no-classname-concatenation", () => {
  test("rejects the class interpolation missed by better-tailwindcss", () => {
    const messages = lint(`
      function Example({ className }: { className: string }) {
        return <div className={\`flex justify-center \${className}\`} />;
      }
    `);

    expect(messages.map((message) => message.messageId)).toEqual(["noInterpolation"]);
    expect(messages[0]?.fix).toBeUndefined();
    expect(messages[0]?.suggestions).toBeUndefined();
  });

  test("rejects nested concatenation, partial tokens, and class callbacks", () => {
    expect(
      messageIds(`
        const color = "red";
        const nested = true ? "text-red-500" : "text-" + color;
        const columns = { cellClassName: "tabular-" + "nums" };
        const fallback = "bg-" + color || "bg-red-500";
        const view = (
          <>
            <div class={nested} />
            <div className={fallback} />
            <NavLink className={({ isActive }) => \`nav-item \${isActive ? "active" : ""}\`} />
            <Table cellClassName={columns.cellClassName} />
          </>
        );
      `)
    ).toEqual(["noConcatenation", "noConcatenation", "noConcatenation", "noInterpolation"]);
  });

  test("follows local bindings and checks conventional class-helper inputs", () => {
    expect(
      messageIds(`
        const tone = "red";
        const generated = \`text-\${tone}\`;
        const alias = generated;
        const joined = "items-" + "center";
        const view = <div className={alias} />;
        cn("flex", joined);
        twJoin(["grid", \`grid-\${tone}\`]);
        twMerge("p-2", \`p-\${tone}\`);
        clsx({ [\`text-\${tone}\`]: true, hidden: \`label-\${tone}\` });
        cx("block", \`display-\${tone}\`);
      `)
    ).toEqual([
      "noInterpolation",
      "noConcatenation",
      "noInterpolation",
      "noInterpolation",
      "noInterpolation",
      "noInterpolation",
    ]);
  });

  test("checks only class-bearing tailwind-variants options", () => {
    expect(
      messageIds(`
        const size = "4";
        tv({
          base: \`base \${size}\`,
          slots: { icon: "size-" + size },
          variants: { tone: { red: \`text-\${size}\` } },
          compoundVariants: [
            { tone: \`condition-\${size}\`, class: "gap-" + size },
          ],
          defaultVariants: { tone: \`default-\${size}\` },
        });
      `)
    ).toEqual(["noInterpolation", "noConcatenation", "noInterpolation", "noConcatenation"]);
  });

  test.each([
    [`const styles = \`flex ${interpolationStart}extra}\`; <div className={styles || "block"} />;`],
    [`const styles = \`flex ${interpolationStart}extra}\`; <div className={styles ?? "block"} />;`],
    [`let styles; styles = \`flex ${interpolationStart}extra}\`; <div className={styles} />;`],
    [
      `const styles = ["flex", \`text-${interpolationStart}color}\`].join(" "); <div className={styles} />;`,
    ],
    [`cn(active && { [\`bg-${interpolationStart}color}\`]: true });`],
  ])("rejects class-producing local flow: %s", (code) => {
    expect(messageIds(code)).toEqual(["noInterpolation"]);
  });

  test("selects only the requested property from local class maps", () => {
    expect(
      lint(`
        const props = { className: "flex", title: \`Hello \${name}\` };
        const view = <div className={props.className} />;
      `)
    ).toEqual([]);
  });

  test.each([
    [
      `const classes = { active: \`text-${interpolationStart}color}\`, inactive: "text-muted" }; <div className={classes[state]} />;`,
    ],
    [
      `const classes = { active: \`text-${interpolationStart}color}\`, inactive: "text-muted" } as const; <div className={classes.active} />;`,
    ],
  ])("checks the selected local class-map values: %s", (code) => {
    expect(messageIds(code)).toEqual(["noInterpolation"]);
  });

  test("allows static tokens, complete-token choices, maps, and helper conditions", () => {
    expect(
      lint(`
        const state = "active";
        const label = \`Account \${state}\`;
        const url = \`/accounts/\${state}\`;
        const classes = {
          active: "text-mint-ink",
          inactive: state === "active" ? "text-muted" : "text-coral",
        };
        const first = second;
        const second = first;
        const view = (
          <div
            aria-label={label}
            className={label === \`Account \${state}\` ? classes[state] : \`text-muted\`}
            data-url={url}
          />
        );
        cn("flex", state === "active" && "items-center", { hidden: label });
        twJoin("grid", state === "active" ? "grid-cols-1" : "grid-cols-2");
        twMerge("p-2", classes[state]);
        clsx(["block", { invisible: url }]);
        cx("font-medium");
        const cyclic = <div className={first} />;
      `)
    ).toEqual([]);
  });

  test("is enabled alongside React hooks for TypeScript and TSX files", () => {
    const localConfig = config.find((entry) => entry.rules?.[ruleId] !== undefined);

    expect(localConfig?.files).toEqual(["**/*.{ts,tsx}"]);
    expect(localConfig?.plugins?.["local"]).toBe(classNamesPlugin);
    expect(localConfig?.rules?.[ruleId]).toBe("error");

    const messages = new Linter().verify(
      `import { useEffect } from "react";
       function Example({ active }) {
         if (active) { useEffect(() => {}, []); }
         return null;
       }`,
      config.filter((entry) => entry.plugins?.["better-tailwindcss"] === undefined),
      { filename: "fixture.tsx" }
    );
    expect(messages.map((message) => message.ruleId)).toContain("react-hooks/rules-of-hooks");
  });
});
