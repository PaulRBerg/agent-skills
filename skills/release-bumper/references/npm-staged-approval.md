# npm Staged Publish Approval

Approve the staged npm versions that this execution's tag pushes created. Use the npmjs.com web UI in the shared
Chromium browser through `$chromium-browser`. Do not use `npm stage approve`, because its 2FA prompt needs a person at
the terminal.

## Scope

- Approve only a staged version that matches a tag this execution pushed. Never approve another staged version.
- Never reject a staged version. Never run `npm publish`, `npm stage approve`, or `npm stage reject`.
- Do not change npm account, package access, token, or trusted-publisher settings.

## Collect Stage Facts

For each pushed tag, find the tag-triggered run of the staged-publish workflow. The run's `headBranch` is the tag name.

```sh
gh run list --workflow <workflow-file> --event push -L 10 \
  --json databaseId,headBranch,status,conclusion
gh run watch <run-id> --exit-status
gh run view <run-id> --log | rg --no-config 'npm notice (name|version|shasum):|staged with id'
```

From the log, record the package name, version, shasum, and stage ID. The stage line has this form:
`+ <package>@<version> (staged with id <stage-id>)`. If the run fails or the log has no stage line, stop. Report the run
URL and the missing fact.

## Open the Staged Packages Page

1. Open `https://www.npmjs.com/` in a task-owned page. Bring the page to the front with `select_page` and
   `bringToFront: true`. 1Password shows its inline menu and passkey prompt only on the foreground tab.
2. If npmjs.com shows `Sign In`, sign in with the `$chromium-browser` 1Password workflow. npm can ask for a security key
   as its second factor. Click `Use security key`, then select the matching npmjs.com passkey in the 1Password prompt.
   If the agent cannot activate a 1Password control, use the human-gate procedure below.
3. Open the `Staged Packages` link from the account settings navigation. Its URL has the form
   `https://www.npmjs.com/settings/<npm-user>/staged-packages`. Read the npm user from the link. Do not guess it.

## Verify and Approve

For each staged version, take a fresh snapshot of the Staged Packages page. Find the entry for the package and version.
Approve the entry only when all of these facts match the CI log and the pushed tag:

- Package name and version.
- Dist-tag, which is `latest` for a stable release. A beta release uses the tag that the workflow passed.
- `SHASUM`.
- `TRUSTED PUBLISHER`, which must be the CI provider that ran the workflow.

If a fact does not match, or the entry is absent, do not approve it. Report the difference and stop for that package.

When every fact matches, click `Approve <package>@<version>`. Do not ask the user for permission first. npm can ask for
2FA before it publishes. Complete the 2FA with the 1Password passkey prompt, as in the sign-in step. If a confirmation
dialog appears, read it and confirm only the same package and version.

A locked or signed-out 1Password, a missing passkey, a hardware security-key touch, or biometric verification is a human
gate. So is a 1Password prompt that the agent cannot activate. Follow the `$chromium-browser` human-gate procedure. Tell
the user the exact page and the one action they must take, such as "click Sign in in the 1Password passkey prompt". Then
continue the workflow from the same page.

## Verify Publication

After each approval, confirm that the registry serves the version:

```sh
npm view <package>@<version> version
```

The registry can take a short time to show a new version. Poll with a bounded background loop, not a manual sleep loop.
Report the version as published only after this command returns it. Then close the task-owned page.
