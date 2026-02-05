---
name: code-simplify
description: This skill should be used when the user asks to "simplify code", "clean up code", "refactor for clarity", "improve code readability", "reduce complexity", "apply coding standards", "make code more maintainable", or mentions code simplification, elegance, or refinement of recently modified code.
---

# Code Simplifier Skill

## Overview

Expert code simplification specialist focused on enhancing code clarity, consistency, and maintainability while preserving exact functionality. Apply project-specific best practices to simplify and improve code without altering its behavior. Prioritize readable, explicit code over overly compact solutions.

## Core Principles

### Preserve Functionality

Never change what the code does—only how it does it. All original features, outputs, and behaviors must remain intact. When uncertain whether a change affects behavior, err on the side of caution.

### Apply Project Standards

Follow established coding standards including:

- **ES Modules**: Use proper import sorting and file extensions
- **Function Declarations**: Prefer `function` keyword over arrow functions for top-level declarations
- **Return Types**: Add explicit return type annotations for top-level functions
- **React Patterns**: Use explicit Props types with proper component patterns
- **Error Handling**: Avoid try/catch when possible; prefer Result types or early returns
- **Naming Conventions**: Maintain consistent naming across the codebase

### Enhance Clarity

Simplify code structure through:

- **Reduce Nesting**: Flatten deeply nested conditionals with early returns or guard clauses
- **Eliminate Redundancy**: Remove duplicate code and unnecessary abstractions
- **Improve Names**: Use clear, descriptive variable and function names that reveal intent
- **Consolidate Logic**: Group related operations together
- **Remove Noise**: Delete comments that describe obvious code behavior
- **Avoid Nested Ternaries**: Prefer switch statements or if/else chains for multiple conditions
- **Choose Explicitness**: Prefer explicit, readable code over overly compact one-liners

### Maintain Balance

Avoid over-simplification that could:

- Reduce code clarity or maintainability
- Create overly clever solutions that are hard to understand
- Combine too many concerns into single functions or components
- Remove helpful abstractions that improve code organization
- Prioritize "fewer lines" over readability (e.g., nested ternaries, dense one-liners)
- Make the code harder to debug or extend

Three similar lines of code is often better than a premature abstraction.

## Scope

Focus on recently modified code unless explicitly instructed to review a broader scope. Run `git diff` to identify what has changed and concentrate refinement efforts there.

## Refinement Process

### Step 1: Identify Modified Code

Run `git diff` to see recent changes. Focus on files and functions that have been touched in the current session.

### Step 2: Analyze for Opportunities

Look for opportunities to improve elegance and consistency:

- Overly complex conditionals that could be simplified
- Repeated patterns that could be consolidated
- Unclear naming that obscures intent
- Unnecessary nesting or indirection
- Violations of project coding standards

### Step 3: Apply Project Standards

Ensure code follows established patterns:

- Correct import ordering and module syntax
- Proper function declaration style
- Explicit type annotations where expected
- Consistent error handling patterns
- Idiomatic React patterns for components

### Step 4: Verify Functionality

Confirm all functionality remains unchanged:

- Same inputs produce same outputs
- Error cases handled identically
- Side effects preserved
- API contracts maintained

### Step 5: Assess Simplicity

Verify the refined code is genuinely simpler:

- Easier to read and understand
- Less cognitive load to maintain
- Clearer intent at each step
- No increase in complexity elsewhere

### Step 6: Document Significant Changes

Note only changes that affect understanding:

- Structural reorganizations
- Renamed abstractions
- Consolidated logic

Skip documenting trivial formatting or style adjustments.

## Anti-Patterns to Avoid

### Don't Over-Compact

```typescript
// Avoid: Dense one-liner
const result = items.filter(x => x.active).map(x => x.value).reduce((a, b) => a + b, 0);

// Prefer: Clear steps
const activeItems = items.filter(item => item.active);
const values = activeItems.map(item => item.value);
const total = values.reduce((sum, value) => sum + value, 0);
```

### Don't Nest Ternaries

```typescript
// Avoid: Nested ternary
const status = count > 100 ? 'high' : count > 50 ? 'medium' : count > 0 ? 'low' : 'none';

// Prefer: Switch or if/else
function getStatus(count: number): string {
  if (count > 100) return 'high';
  if (count > 50) return 'medium';
  if (count > 0) return 'low';
  return 'none';
}
```

### Don't Abstract Prematurely

```typescript
// Avoid: Unnecessary abstraction for one use case
const formatters = {
  date: (d: Date) => d.toISOString(),
  name: (n: string) => n.trim().toLowerCase(),
};
const formatted = formatters[type](value);

// Prefer: Direct code when used once
const formatted = type === 'date'
  ? value.toISOString()
  : value.trim().toLowerCase();
```

### Don't Remove Helpful Structure

Keep abstractions that genuinely improve organization. Not every helper function needs to be inlined. Evaluate whether the abstraction aids comprehension before removing it.

## Output Format

When presenting simplified code:

1. Show the original code snippet
2. Present the refined version
3. Briefly explain the improvement (one line)
4. Confirm functionality is preserved
