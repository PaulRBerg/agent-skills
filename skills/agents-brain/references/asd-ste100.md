# STE Authoring and Review

Use this STE-based profile for agent-facing prose. It adapts Simplified Technical English for skill instructions and
repository guidance. It does not establish compliance with the official ASD-STE100 dictionary or certify a document
against the standard.

Read this reference completely before writing or changing prose. Apply the rules during writing, then complete the
review below.

## Contents

- [Rule profile](#rule-profile)
- [Meaning and protected content](#meaning-and-protected-content)
- [Meaning-preserving examples](#meaning-preserving-examples)
- [Completion review](#completion-review)
- [Attribution and license](#attribution-and-license)

## Rule Profile

- Use active voice. Name the actor when the reader could confuse who performs the action.
- Use simple verbs and familiar words. Use one term consistently for each concept.
- State conditions before the actions they control. Keep each condition attached to every affected action.
- Give one instruction per sentence. Keep simultaneous actions together when splitting them would imply a sequence.
- Target 20 words for an instruction and 25 words for a description.
- Keep one topic per paragraph, with at most six sentences. Use lists when they clarify steps, conditions, or
  alternatives.
- Avoid ambiguous phrasal verbs, unnecessary noun forms of actions, long noun clusters, and promotional claims.
- Do not use semicolons in prose. Retain required subjects, verbs, articles, and other words that make the meaning
  explicit.
- Do not use contractions. Prefer simple verb forms when they preserve the meaning.

Meaning and safety take priority over a length target. Keep a longer sentence when shortening it would lose necessary
precision. Review each such exception explicitly. Do not count a length exception as proof of compliance.

Keep necessary technical terms and proper names. Define an unfamiliar term when the intended reader needs the
definition. Keep compound verb forms when they convey uncertainty or current relevance, such as “may have failed.”

## Meaning and Protected Content

Read the source for meaning before changing its style. Preserve these parts of every instruction and claim:

- Obligation strength, permission, prohibition, and uncertainty.
- Conditions, exceptions, quantifiers, actor, and scope.
- Sequence, concurrency, fallback behavior, and success criteria.

A recommendation with “should” remains a recommendation. Permission with “may” does not become a requirement with
“must.” Do not turn a possible outcome into a fact. Do not add checks, causes, guarantees, or other facts while
rewriting.

Keep command and code payloads, paths, IDs, schema keys, exact-match text, copied requirements, and quotations
unchanged. Keep required literal declarations unchanged, including coordination-exemption declarations. Treat these as
protected content, even when they differ from this profile. Preserve headings and anchors unless the task authorizes a
change.

Apply this profile within the task's existing authority and file scope. A style rule does not authorize factual,
behavioral, or structural changes. Honor instructions to preserve accurate user-authored prose and structure. Review
protected prose without forcing a rewrite.

## Meaning-Preserving Examples

| Original                                                           | Rewrite                                                     | Meaning retained                                  |
| ------------------------------------------------------------------ | ----------------------------------------------------------- | ------------------------------------------------- |
| The agent should perform an inspection of the log before retrying. | The agent should inspect the log before retrying.           | Recommendation and sequence.                      |
| If validation fails, the agent may make a retry of the request.    | If validation fails, the agent may retry the request.       | Condition and permission.                         |
| The request may have failed.                                       | The request may have failed.                                | Uncertainty requires the compound form.           |
| While the job runs, monitor its output and report failures.        | While the job runs, monitor its output and report failures. | The condition applies to both concurrent actions. |

## Completion Review

1. Compare the changed prose with the source and the authoritative requirements.
2. Check every obligation, permission, uncertainty, condition, exception, quantifier, actor, and scope.
3. Check sequence, concurrency, fallback behavior, and success criteria.
4. Check protected content byte-for-byte. Check that existing headings, anchors, and reference links still work.
5. Check the rule profile sentence by sentence. Review every retained length or verb-form exception for necessary
   precision.
6. Correct meaning or style defects within the authorized scope. Report unresolved ambiguity or a defect that requires
   separate authority.

Report the reviewed scope and material exceptions when they affect the completion claim. Describe the result as reviewed
against this profile. Do not claim official dictionary compliance. A formatter or metadata validator does not perform
this semantic review.

## Attribution and License

Adapted from Dustin Yuchen Teng's
[asd-ste100-skill v0.4.0](https://github.com/danyuchn/asd-ste100-skill/tree/32511c6992ecb5f1971e46a2943f2e6adceedafe).
Sources: [SKILL.md](https://github.com/danyuchn/asd-ste100-skill/blob/32511c6992ecb5f1971e46a2943f2e6adceedafe/SKILL.md)
and
[writing-rules.md](https://github.com/danyuchn/asd-ste100-skill/blob/32511c6992ecb5f1971e46a2943f2e6adceedafe/references/writing-rules.md).
This adaptation includes no official ASD dictionary or PDF content.

```text
MIT License

Copyright (c) 2026 Dustin Yuchen Teng

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```
