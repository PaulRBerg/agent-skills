import { expect, test } from "bun:test";
import { readFileSync } from "node:fs";
import path from "node:path";

const source = readFileSync(path.join(import.meta.dir, "../../skills/evm-atlas/scripts/debank-collect.js"), "utf8");
const install = new Function(
  "window",
  "location",
  "history",
  "dispatchEvent",
  "PopStateEvent",
  `return (${source})();`,
);

const DOC = "0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe";
const DEAD = "0x000000000000000000000000000000000000dEaD";
const USDC = "0xA0b86991c6218b36c1d19D4a2e9Eb0cE3606eB48";
const FAST = { timeoutMs: 500, maxAttempts: 3, cooldownMs: 20 };

type Wallet = { chains: string[]; balances?: Record<string, object[]>; usedChainsStatuses?: number[] };

const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status });

class PopStateEvent {
  constructor(readonly type: string) {}
}

// Simulates a debank.com page: the app fetches a profile's data after each popstate route change,
// ignoring a route to the profile it already shows.
function createPage(wallets: Record<string, Wallet>, initialPath = "/") {
  const location = { pathname: initialPath, href: `https://debank.com${initialPath}` };
  const routes: string[] = [];
  const usedChainsCalls = new Map<string, number>();
  let rendered = initialPath.toLowerCase();

  async function network(input: string) {
    const url = new URL(input);
    if (url.pathname === "/chain/list") {
      const chains = [
        { id: "eth", network_id: "1" },
        { id: "bsc", network_id: "56" },
      ];
      return json({ data: { chains }, error_code: 0 });
    }
    const address = url.searchParams.get("id") ?? url.searchParams.get("user_addr") ?? "";
    const wallet = wallets[address]!;
    if (url.pathname === "/user/used_chains") {
      const call = usedChainsCalls.get(address) ?? 0;
      usedChainsCalls.set(address, call + 1);
      if (wallet.usedChainsStatuses?.[call] === 429) {
        return json({ error_code: 429, error_msg: "Request too fast, please try again later." }, 429);
      }
      return json({ data: { chains: wallet.chains }, error_code: 0 });
    }
    if (url.pathname === "/token/balance_list") {
      return json({ data: wallet.balances?.[url.searchParams.get("chain")!] ?? [], error_code: 0 });
    }
    return json({ data: {}, error_code: 0 });
  }

  const window: any = { fetch: network };

  async function loadProfile(address: string) {
    await window.fetch(`https://api.debank.com/user/config?id=${address}`);
    const res = await window.fetch(`https://api.debank.com/user/used_chains?id=${address}`);
    if (res.status !== 200) return;
    const { data } = await res.json();
    await window.fetch(`https://api.debank.com/token/cache_balance_list?user_addr=${address}`);
    await Promise.all(
      data.chains.map((chain: string) =>
        window
          .fetch(`https://api.debank.com/token/balance_list?user_addr=${address}&chain=${chain}`)
          .then((r: Response) => r.json()),
      ),
    );
  }

  const history = {
    pushState(_state: unknown, _title: string, url: string) {
      location.pathname = url;
      location.href = `https://debank.com${url}`;
      routes.push(url);
    },
  };

  function dispatchEvent(event: PopStateEvent) {
    if (event.type !== "popstate" || location.pathname.toLowerCase() === rendered) return true;
    rendered = location.pathname.toLowerCase();
    const profile = /^\/profile\/(0x[0-9a-f]{40})$/.exec(rendered);
    if (profile) void loadProfile(profile[1]!);
    return true;
  }

  const installed = install(window, location, history, dispatchEvent, PopStateEvent);
  const reinstall = () => install(window, location, history, dispatchEvent, PopStateEvent);
  return { api: window.__debankCollect, installed, reinstall, routes, window };
}

async function finish(api: any) {
  for (let i = 0; i < 400 && api.status().running; i += 1) await Bun.sleep(5);
  const records = api.results().map(({ observedAt, ...record }: any) => {
    expect(new Date(observedAt).toISOString()).toBe(observedAt);
    return record;
  });
  return { status: api.status(), records };
}

test("script stays a pasteable arrow-function expression", () => {
  expect(source.trimStart().startsWith("() => {")).toBe(true);
  expect(source.trimEnd().endsWith("}")).toBe(true);
});

test("install is idempotent and never double-wraps fetch", () => {
  const page = createPage({});
  const wrapped = page.window.fetch;
  expect(page.installed).toEqual({ installed: true, reused: false });
  expect(page.reinstall()).toEqual({ installed: true, reused: true });
  expect(page.window.fetch).toBe(wrapped);
});

test("collects natives and contracts with chain IDs and exact raw amounts", async () => {
  const doc = DOC.toLowerCase();
  const page = createPage({
    [doc]: {
      chains: ["eth", "bsc", "newchain"],
      balances: {
        eth: [
          // 1000 ETH: the raw amount exceeds 2^53.
          {
            chain: "eth",
            id: "eth",
            symbol: "ETH",
            decimals: 18,
            amount: 1000,
            raw_amount_hex_str: "0x3635c9adc5dea00000",
            price: 2500,
          },
          {
            chain: "eth",
            id: USDC,
            symbol: "USDC",
            decimals: 6,
            amount: 12.5,
            raw_amount_hex_str: "0xbebc20",
            price: 1,
          },
        ],
        bsc: [
          {
            chain: "bsc",
            id: "bsc",
            symbol: "BNB",
            decimals: 18,
            amount: 0.5,
            raw_amount_hex_str: "0x6f05b59d3b20000",
            price: 600,
          },
        ],
        newchain: [
          {
            chain: "newchain",
            id: "newchain",
            symbol: "NEW",
            decimals: 18,
            amount: 1e-18,
            raw_amount_hex_str: "0x1",
            price: 0,
          },
        ],
      },
    },
  });

  expect(await page.api.start([DOC, doc], FAST)).toEqual({ queued: 1 });
  const { status, records } = await finish(page.api);

  expect(page.routes).toEqual([`/profile/${doc}`]);
  expect(status).toMatchObject({ running: false, total: 1, done: 1, ok: 1, failed: 0, pending: 0, rateLimited: 0 });
  expect(records).toEqual([
    {
      address: doc,
      status: "ok",
      attempts: 1,
      chains: ["eth", "bsc", "newchain"],
      tokens: [
        {
          chainId: 1,
          chain: "eth",
          contract: "native",
          symbol: "ETH",
          decimals: 18,
          rawAmount: "1000000000000000000000",
          amount: 1000,
          price: 2500,
        },
        {
          chainId: 1,
          chain: "eth",
          contract: USDC.toLowerCase(),
          symbol: "USDC",
          decimals: 6,
          rawAmount: "12500000",
          amount: 12.5,
          price: 1,
        },
        {
          chainId: 56,
          chain: "bsc",
          contract: "native",
          symbol: "BNB",
          decimals: 18,
          rawAmount: "500000000000000000",
          amount: 0.5,
          price: 600,
        },
        {
          chainId: null,
          chain: "newchain",
          contract: "native",
          symbol: "NEW",
          decimals: 18,
          rawAmount: "1",
          amount: 1e-18,
          price: 0,
        },
      ],
    },
  ]);
});

test("an empty used_chains list is ok with zero tokens", async () => {
  const dead = DEAD.toLowerCase();
  const page = createPage({ [dead]: { chains: [] } });

  await page.api.start([DEAD], FAST);
  const { records } = await finish(page.api);

  expect(records).toEqual([{ address: dead, status: "ok", attempts: 1, chains: [], tokens: [] }]);
});

test("retries a rate-limited used_chains after the cooldown, via / when the profile is still shown", async () => {
  const doc = DOC.toLowerCase();
  const page = createPage({ [doc]: { chains: [], usedChainsStatuses: [429, 200] } });

  await page.api.start([DOC], FAST);
  const { status, records } = await finish(page.api);

  expect(page.routes).toEqual([`/profile/${doc}`, "/", `/profile/${doc}`]);
  expect(status).toMatchObject({ ok: 1, failed: 0, rateLimited: 1 });
  expect(status.elapsedMs).toBeGreaterThanOrEqual(FAST.cooldownMs);
  expect(records).toEqual([{ address: doc, status: "ok", attempts: 2, chains: [], tokens: [] }]);
});

test("fails an address after maxAttempts while the rest of the queue continues", async () => {
  const [doc, dead] = [DOC.toLowerCase(), DEAD.toLowerCase()];
  const page = createPage({
    [dead]: { chains: [], usedChainsStatuses: [429, 429] },
    [doc]: { chains: [] },
  });

  await page.api.start([DEAD, DOC], { ...FAST, maxAttempts: 2 });
  const { status, records } = await finish(page.api);

  expect(page.routes).toEqual([`/profile/${dead}`, `/profile/${doc}`, `/profile/${dead}`]);
  expect(status).toMatchObject({ total: 2, done: 2, ok: 1, failed: 1, pending: 0, rateLimited: 2 });
  expect(records).toEqual([
    { address: dead, status: "failed", attempts: 2, error: "used_chains HTTP 429", chains: [], tokens: [] },
    { address: doc, status: "ok", attempts: 1, chains: [], tokens: [] },
  ]);
});

test("routes through / before collecting the profile already displayed", async () => {
  const doc = DOC.toLowerCase();
  const page = createPage({ [doc]: { chains: [] } }, `/profile/${DOC}`);

  await page.api.start([DOC], FAST);
  const { records } = await finish(page.api);

  expect(page.routes).toEqual(["/", `/profile/${doc}`]);
  expect(records).toEqual([{ address: doc, status: "ok", attempts: 1, chains: [], tokens: [] }]);
});

test("rejects invalid addresses and a second start while a run is active", async () => {
  const page = createPage({ [DOC.toLowerCase()]: { chains: [] } });

  await expect(page.api.start(["0x1234"], FAST)).rejects.toThrow("invalid addresses: 0x1234");
  await expect(page.api.start([DOC, `${DOC}00`], FAST)).rejects.toThrow("invalid addresses");
  expect(page.api.status()).toMatchObject({ running: false, total: 0 });

  const first = page.api.start([DOC], FAST);
  await expect(page.api.start([DEAD], FAST)).rejects.toThrow("a run is already active");
  expect(await first).toEqual({ queued: 1 });
  expect((await finish(page.api)).status).toMatchObject({ ok: 1 });
});
