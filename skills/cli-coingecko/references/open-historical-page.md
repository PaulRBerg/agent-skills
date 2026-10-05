# Historical Page Fallback

Open CoinGecko's historical-data page only when both conditions apply:

- `cli-coingecko` routes the request here after a genuine CLI failure.
- A visual result can satisfy the request.

## Workflow

1. Require a CoinGecko coin ID and an ISO date (`YYYY-MM-DD`). If `cg search` still works, use it to resolve a supplied
   name or ambiguous symbol. If the entire CLI is unavailable, resolve the ID through CoinGecko's website search in
   Chromium. Under that condition, confirm the page slug. Never infer an ID from a symbol.

2. Build the validated ±1-day URL:

   ```sh
   uv run <skill-dir>/scripts/build-url.py <coin-id> <date>
   ```

   For invalid IDs or calendar dates, the helper exits nonzero without opening a page.

3. Pass the returned URL to Chrome DevTools `new_page` with `background: false`. Do not use the macOS `open` command.

4. Complete with `### 🌐 CoinGecko history opened — <coin-id> · <date> (±1 day)`, the linked page URL, and a concise
   disclosure of the CLI failure that triggered the fallback. Keep the helper's bare-URL stdout and validation errors
   undecorated.

## Boundaries

- Do not use this route for authentication, tier, rate-limit, invalid-input, or ambiguity errors.
- A browser page is not machine-readable evidence. If the user requested JSON, CSV, or an exact export, report the CLI
  failure without claiming completion.
