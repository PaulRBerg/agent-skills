import type { TSESLint, TSESTree } from "@typescript-eslint/utils";
import type { ESLint } from "eslint";

const classPropertyPattern = /^(?:class|className|.+ClassName)$/u;
const classHelpers = new Set(["clsx", "cn", "cx", "twJoin", "twMerge"]);

type MessageId = "noConcatenation" | "noInterpolation";
type Seen = Set<TSESTree.Node>;

function staticPropertyName(node: TSESTree.Node): string | undefined {
  if (node.type === "Identifier") {
    return node.name;
  }
  if (
    node.type === "Literal" &&
    (typeof node.value === "string" || typeof node.value === "number")
  ) {
    return String(node.value);
  }
  return undefined;
}

function isClassProperty(node: TSESTree.PropertyName): boolean {
  const name = staticPropertyName(node);
  return name !== undefined && classPropertyPattern.test(name);
}

function directCalleeName(node: TSESTree.CallExpression): string | undefined {
  return node.callee.type === "Identifier" ? node.callee.name : undefined;
}

function isFunction(
  node: TSESTree.Node
): node is
  | TSESTree.ArrowFunctionExpression
  | TSESTree.FunctionDeclaration
  | TSESTree.FunctionExpression {
  return (
    node.type === "ArrowFunctionExpression" ||
    node.type === "FunctionDeclaration" ||
    node.type === "FunctionExpression"
  );
}

export const noClassNameConcatenation: TSESLint.RuleModule<MessageId, []> = {
  defaultOptions: [],
  meta: {
    docs: {
      description: "Require complete static tokens in React class expressions",
    },
    messages: {
      noConcatenation:
        "Do not build CSS classes with + concatenation; select complete class tokens instead.",
      noInterpolation:
        "Do not interpolate CSS classes in a template literal; select complete class tokens instead.",
    },
    schema: [],
    type: "problem",
  },
  create(context) {
    const reported = new WeakSet<TSESTree.Node>();
    const sourceCode = context.sourceCode;

    function report(node: TSESTree.Node, messageId: MessageId): void {
      if (reported.has(node)) {
        return;
      }
      reported.add(node);
      context.report({ messageId, node });
    }

    function localValues(identifier: TSESTree.Identifier): TSESTree.Node[] {
      for (
        let scope: TSESLint.Scope.Scope | null = sourceCode.getScope(identifier);
        scope !== null;
        scope = scope.upper
      ) {
        const variable = scope.set.get(identifier.name);
        if (!variable) {
          continue;
        }

        const values: TSESTree.Node[] = [];
        for (const definition of variable.defs) {
          if (definition.type === "Variable") {
            if (definition.node.init) {
              values.push(definition.node.init);
            }
          } else if (definition.type === "FunctionName" && isFunction(definition.node)) {
            values.push(definition.node);
          }
        }
        for (const reference of variable.references) {
          if (reference.writeExpr && reference.identifier.range[0] <= identifier.range[0]) {
            values.push(reference.writeExpr);
          }
        }
        return values;
      }
      return [];
    }

    function checkFunctionReturns(
      node:
        | TSESTree.ArrowFunctionExpression
        | TSESTree.FunctionDeclaration
        | TSESTree.FunctionExpression,
      seen: Seen
    ): void {
      if (node.type === "ArrowFunctionExpression" && node.expression) {
        checkClassValue(node.body, seen);
        return;
      }

      const visit = (current: TSESTree.Node): void => {
        if (current !== node.body && isFunction(current)) {
          return;
        }
        if (current.type === "ReturnStatement") {
          if (current.argument) {
            checkClassValue(current.argument, seen);
          }
          return;
        }

        for (const key of sourceCode.visitorKeys[current.type] ?? []) {
          const child = (current as unknown as Record<string, unknown>)[key];
          if (Array.isArray(child)) {
            for (const entry of child) {
              if (entry && typeof entry === "object" && "type" in entry) {
                visit(entry as TSESTree.Node);
              }
            }
          } else if (child && typeof child === "object" && "type" in child) {
            visit(child as TSESTree.Node);
          }
        }
      };

      visit(node.body);
    }

    function checkClassMap(node: TSESTree.Node, seen: Seen): void {
      if (seen.has(node)) {
        return;
      }
      seen.add(node);

      if (node.type === "Identifier") {
        for (const value of localValues(node)) {
          checkClassMap(value, seen);
        }
        return;
      }
      if (node.type === "ObjectExpression") {
        for (const property of node.properties) {
          if (property.type === "SpreadElement") {
            checkClassMap(property.argument, seen);
          } else if (property.kind === "init") {
            checkClassValue(property.value, seen);
          }
        }
        return;
      }
      if (node.type === "ArrayExpression") {
        for (const element of node.elements) {
          if (element) {
            checkClassValue(element, seen);
          }
        }
        return;
      }

      checkClassValue(node, seen, true);
    }

    function checkClassMember(node: TSESTree.MemberExpression, seen: Seen): void {
      let propertyName: string | undefined;
      if (node.computed) {
        propertyName =
          node.property.type === "Literal" ? staticPropertyName(node.property) : undefined;
      } else {
        propertyName = node.property.type === "Identifier" ? node.property.name : undefined;
      }

      function checkObject(value: TSESTree.Node): void {
        if (seen.has(value)) {
          return;
        }
        seen.add(value);

        if (value.type === "Identifier") {
          for (const localValue of localValues(value)) {
            checkObject(localValue);
          }
          return;
        }
        if (
          value.type === "TSAsExpression" ||
          value.type === "TSSatisfiesExpression" ||
          value.type === "TSTypeAssertion"
        ) {
          checkObject(value.expression);
          return;
        }
        if (value.type === "ArrayExpression") {
          if (propertyName === undefined) {
            for (const element of value.elements) {
              if (element) {
                checkClassValue(element, seen);
              }
            }
          } else {
            const index = Number(propertyName);
            const element = Number.isInteger(index) ? value.elements[index] : undefined;
            if (element) {
              checkClassValue(element, seen);
            }
          }
          return;
        }
        if (value.type !== "ObjectExpression") {
          return;
        }

        for (const property of value.properties) {
          if (property.type === "SpreadElement") {
            checkObject(property.argument);
          } else if (
            property.kind === "init" &&
            (propertyName === undefined || staticPropertyName(property.key) === propertyName)
          ) {
            checkClassValue(property.value, seen);
          }
        }
      }

      checkObject(node.object);
    }

    function checkHelperInput(node: TSESTree.Node, seen: Seen): void {
      if (seen.has(node)) {
        return;
      }
      seen.add(node);

      if (node.type === "Identifier") {
        for (const value of localValues(node)) {
          checkHelperInput(value, seen);
        }
        return;
      }
      if (node.type === "SpreadElement") {
        checkHelperInput(node.argument, seen);
        return;
      }
      if (node.type === "ArrayExpression") {
        for (const element of node.elements) {
          if (element) {
            checkHelperInput(element, seen);
          }
        }
        return;
      }
      if (node.type === "ConditionalExpression") {
        checkHelperInput(node.consequent, seen);
        checkHelperInput(node.alternate, seen);
        return;
      }
      if (node.type === "LogicalExpression") {
        if (node.operator !== "&&") {
          checkHelperInput(node.left, seen);
        }
        checkHelperInput(node.right, seen);
        return;
      }
      if (node.type === "ObjectExpression") {
        for (const property of node.properties) {
          if (property.type === "SpreadElement") {
            checkHelperInput(property.argument, seen);
          } else if (property.computed) {
            checkClassValue(property.key, seen);
          }
        }
        return;
      }

      checkClassValue(node, seen, true);
    }

    function checkDirectViolation(node: TSESTree.Node): boolean {
      switch (node.type) {
        case "TemplateLiteral": {
          if (node.expressions.length > 0) {
            report(node, "noInterpolation");
          }
          return true;
        }
        case "BinaryExpression": {
          if (node.operator === "+") {
            report(node, "noConcatenation");
          }
          return true;
        }
        default: {
          return false;
        }
      }
    }

    function checkClassBranches(node: TSESTree.Node, seen: Seen): boolean {
      switch (node.type) {
        case "ConditionalExpression": {
          checkClassValue(node.consequent, seen);
          checkClassValue(node.alternate, seen);
          return true;
        }
        case "LogicalExpression": {
          if (node.operator !== "&&") {
            checkClassValue(node.left, seen);
          }
          checkClassValue(node.right, seen);
          return true;
        }
        case "AssignmentExpression": {
          checkClassValue(node.right, seen);
          return true;
        }
        case "SequenceExpression": {
          const last = node.expressions.at(-1);
          if (last) {
            checkClassValue(last, seen);
          }
          return true;
        }
        case "ChainExpression":
        case "TSAsExpression":
        case "TSInstantiationExpression":
        case "TSNonNullExpression":
        case "TSSatisfiesExpression":
        case "TSTypeAssertion": {
          checkClassValue(node.expression, seen);
          return true;
        }
        case "AwaitExpression": {
          checkClassValue(node.argument, seen);
          return true;
        }
        case "YieldExpression": {
          if (node.argument) {
            checkClassValue(node.argument, seen);
          }
          return true;
        }
        default: {
          return false;
        }
      }
    }

    function checkClassStructure(node: TSESTree.Node, seen: Seen): void {
      switch (node.type) {
        case "Identifier": {
          for (const value of localValues(node)) {
            checkClassValue(value, seen);
          }
          break;
        }
        case "MemberExpression": {
          checkClassMember(node, seen);
          break;
        }
        case "ArrayExpression": {
          for (const element of node.elements) {
            if (element) {
              checkClassValue(element, seen);
            }
          }
          break;
        }
        case "ArrowFunctionExpression":
        case "FunctionDeclaration":
        case "FunctionExpression": {
          checkFunctionReturns(node, seen);
          break;
        }
        case "CallExpression": {
          if (classHelpers.has(directCalleeName(node) ?? "")) {
            for (const argument of node.arguments) {
              checkHelperInput(argument, seen);
            }
          } else if (
            node.callee.type === "MemberExpression" &&
            !node.callee.computed &&
            node.callee.property.type === "Identifier" &&
            node.callee.property.name === "join"
          ) {
            checkClassMap(node.callee.object, seen);
          }
          break;
        }
      }
    }

    function checkClassValue(node: TSESTree.Node, seen: Seen, alreadySeen = false): void {
      if (!alreadySeen) {
        if (seen.has(node)) {
          return;
        }
        seen.add(node);
      }

      if (checkDirectViolation(node) || checkClassBranches(node, seen)) {
        return;
      }
      checkClassStructure(node, seen);
    }

    function checkTvOptions(node: TSESTree.Node): void {
      const seen: Seen = new Set();

      function checkOptions(value: TSESTree.Node): void {
        if (seen.has(value)) {
          return;
        }
        seen.add(value);

        if (value.type === "Identifier") {
          for (const localValue of localValues(value)) {
            checkOptions(localValue);
          }
          return;
        }
        if (value.type !== "ObjectExpression") {
          return;
        }

        for (const property of value.properties) {
          if (property.type === "SpreadElement" || property.kind !== "init") {
            continue;
          }
          const name = staticPropertyName(property.key);
          if (name === "base") {
            checkClassValue(property.value, seen);
          } else if (name === "slots") {
            checkClassMap(property.value, seen);
          } else if (name === "variants") {
            checkVariantGroups(property.value);
          } else if (name === "compoundVariants" || name === "compoundSlots") {
            checkCompoundOptions(property.value);
          }
        }
      }

      function checkVariantGroups(value: TSESTree.Node): void {
        if (value.type !== "ObjectExpression") {
          return;
        }
        for (const group of value.properties) {
          if (group.type === "Property" && group.kind === "init") {
            checkClassMap(group.value, seen);
          }
        }
      }

      function checkCompoundOptions(value: TSESTree.Node): void {
        if (value.type !== "ArrayExpression") {
          return;
        }
        for (const element of value.elements) {
          if (!element || element.type !== "ObjectExpression") {
            continue;
          }
          for (const property of element.properties) {
            if (
              property.type === "Property" &&
              property.kind === "init" &&
              isClassProperty(property.key)
            ) {
              checkClassValue(property.value, seen);
            }
          }
        }
      }

      checkOptions(node);
    }

    return {
      CallExpression(node) {
        const name = directCalleeName(node);
        if (classHelpers.has(name ?? "")) {
          const seen: Seen = new Set();
          for (const argument of node.arguments) {
            checkHelperInput(argument, seen);
          }
        } else if (name === "tv") {
          const options = node.arguments[0];
          if (options) {
            checkTvOptions(options);
          }
        }
      },
      JSXAttribute(node) {
        if (
          node.name.type !== "JSXIdentifier" ||
          !classPropertyPattern.test(node.name.name) ||
          node.value?.type !== "JSXExpressionContainer" ||
          node.value.expression.type === "JSXEmptyExpression"
        ) {
          return;
        }
        checkClassValue(node.value.expression, new Set());
      },
      Property(node) {
        if (node.kind === "init" && isClassProperty(node.key)) {
          checkClassValue(node.value, new Set());
        }
      },
      PropertyDefinition(node) {
        if (node.value && isClassProperty(node.key)) {
          checkClassValue(node.value, new Set());
        }
      },
    };
  },
};

const classNamesPlugin = {
  rules: {
    "no-classname-concatenation": noClassNameConcatenation,
  },
} as unknown as ESLint.Plugin;

export default classNamesPlugin;
