# Historical Page Fallback

Open CoinGecko's historical-data page only when `cli-coingecko` routes a request here after a genuine CLI failure and a
visual result can satisfy the request.

## Workflow

1. Require a CoinGecko coin ID and an ISO date (`YYYY-MM-DD`). If `cg search` still works, use it to resolve a supplied
   name or ambiguous symbol. If the entire CLI is unavailable, resolve the ID through CoinGecko's website search in
   Chromium and confirm the page slug; never infer an ID from a symbol.

2. Build the validated ±1-day URL:

   ```sh
   uv run <skill-dir>/scripts/build-url.py <coin-id> <date>
   ```

   Invalid IDs or calendar dates exit nonzero without opening a page.

3. Pass the returned URL to `mcp__chrome-devtools__new_page` with `background: false`. Do not use the macOS `open`
   command.

4. Complete with `### 🌐 CoinGecko history opened — <coin-id> · <date> (±1 day)`, the linked page URL, and a concise
   disclosure of the CLI failure that triggered the fallback. Keep the helper's bare-URL stdout and validation errors
   undecorated.

## Boundaries

- Do not use this route for authentication, tier, rate-limit, invalid-input, or ambiguity errors.
- A browser page is not machine-readable evidence. If the user requested JSON, CSV, or an exact export, report the CLI
  failure without claiming completion.
