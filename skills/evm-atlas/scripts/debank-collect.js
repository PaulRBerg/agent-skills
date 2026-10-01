() => {
  // Paste verbatim as the Chrome DevTools MCP `evaluate_script` function on a debank.com page.
  // It records the app's own signed responses; it never calls balance endpoints itself.
  if (window.__debankCollect) return { installed: true, reused: true };

  const ADDRESS = /^0x[0-9a-f]{40}$/i;
  const USED_CHAINS = "/user/used_chains";
  const BALANCE_LIST = "/token/balance_list";
  const networkFetch = window.fetch;
  const waiters = new Set();
  let events = [];
  let chainIds = null;
  let run = null;

  const sleep = (ms) => new Promise((resolve) => setTimeout(resolve, ms));

  function emit(event) {
    events.push(event);
    if (run && !run.finishedAt && (event.status === 429 || event.json?.error_code === 429)) run.rateLimited += 1;
    for (const waiter of [...waiters]) if (waiter.match(event)) waiter.done(event);
  }

  function trackedUrl(input) {
    try {
      const url = new URL(typeof input === "string" ? input : (input.url ?? String(input)), location.href);
      const tracked = url.hostname === "api.debank.com" && [USED_CHAINS, BALANCE_LIST].includes(url.pathname);
      return tracked ? url : null;
    } catch {
      return null;
    }
  }

  window.fetch = function (input, init) {
    const url = trackedUrl(input);
    if (!url) return networkFetch.call(window, input, init);
    const params = url.searchParams;
    const event = {
      path: url.pathname,
      address: (params.get("id") ?? params.get("user_addr") ?? "").toLowerCase(),
      chain: params.get("chain"),
      at: Date.now(),
    };
    return networkFetch.call(window, input, init).then(
      (res) => {
        res
          .clone()
          .json()
          .catch(() => null)
          .then((json) => emit({ ...event, status: res.status, json }));
        return res;
      },
      (error) => {
        emit({ ...event, status: 0, json: null, error: String(error) });
        throw error;
      },
    );
  };

  // Resolves with the first matching event (already recorded or future), or null at the deadline.
  function waitFor(match, deadline) {
    const seen = events.find(match);
    if (seen) return Promise.resolve(seen);
    return new Promise((resolve) => {
      const waiter = {
        match,
        done(event) {
          clearTimeout(timer);
          waiters.delete(waiter);
          resolve(event);
        },
      };
      const timer = setTimeout(() => waiter.done(null), Math.max(0, deadline - Date.now()));
      waiters.add(waiter);
    });
  }

  function problem(event, label) {
    if (event.status === 0) return `${label} network error: ${event.error}`;
    if (event.status !== 200) return `${label} HTTP ${event.status}`;
    if (!event.json) return `${label} returned invalid JSON`;
    if (event.json.error_code) return `${label} error_code ${event.json.error_code}`;
    return null;
  }

  function route(path) {
    history.pushState({}, "", path);
    dispatchEvent(new PopStateEvent("popstate", { state: {} }));
  }

  function toToken(item) {
    const hex = String(item.raw_amount_hex_str);
    return {
      chainId: chainIds.get(item.chain) ?? null,
      chain: item.chain,
      contract: ADDRESS.test(item.id) ? item.id.toLowerCase() : "native",
      symbol: item.symbol,
      decimals: item.decimals,
      rawAmount: BigInt(hex.startsWith("0x") ? hex : `0x${hex}`).toString(),
      amount: item.amount,
      price: item.price,
    };
  }

  async function collect(address, timeoutMs) {
    events = [];
    // The app ignores a route to the profile it already shows.
    if (location.pathname.toLowerCase() === `/profile/${address}`) route("/");
    const since = Date.now();
    const deadline = since + timeoutMs;
    const ours = (path) => (event) => event.path === path && event.address === address && event.at >= since;
    route(`/profile/${address}`);

    const used = await waitFor(ours(USED_CHAINS), deadline);
    if (!used) throw new Error("timeout waiting for used_chains");
    const usedProblem = problem(used, "used_chains");
    if (usedProblem) throw new Error(usedProblem);
    const chains = used.json.data?.chains;
    if (!Array.isArray(chains)) throw new Error("used_chains response has no data.chains array");

    const pending = new Set(chains);
    const tokens = [];
    while (pending.size > 0) {
      const event = await waitFor((e) => ours(BALANCE_LIST)(e) && pending.has(e.chain), deadline);
      if (!event) throw new Error(`timeout waiting for balance_list: ${[...pending].join(", ")}`);
      const listProblem = problem(event, `balance_list ${event.chain}`);
      if (listProblem) throw new Error(listProblem);
      if (!Array.isArray(event.json.data)) throw new Error(`balance_list ${event.chain} has no data array`);
      pending.delete(event.chain);
      tokens.push(...event.json.data.map(toToken));
    }
    return { chains, tokens };
  }

  function failedRecord(address, attempts, error) {
    return { address, status: "failed", attempts, error, chains: [], observedAt: new Date().toISOString(), tokens: [] };
  }

  async function drain(current, { timeoutMs, maxAttempts, cooldownMs, haltAfter }) {
    const queue = [...current.addresses];
    // Consecutive failed attempts that saw a 429; a sustained streak means DeBank's WAF is blocking this client.
    let limitedStreak = 0;
    while (queue.length > 0) {
      const address = queue.shift();
      const attempts = (current.attempts.get(address) ?? 0) + 1;
      current.attempts.set(address, attempts);
      const limitedBefore = current.rateLimited;
      try {
        const { chains, tokens } = await collect(address, timeoutMs);
        const observedAt = new Date().toISOString();
        current.records.set(address, { address, status: "ok", attempts, chains, observedAt, tokens });
        limitedStreak = 0;
      } catch (error) {
        limitedStreak = current.rateLimited > limitedBefore ? limitedStreak + 1 : 0;
        if (attempts < maxAttempts) queue.push(address);
        else current.records.set(address, failedRecord(address, attempts, error.message));
        if (limitedStreak >= haltAfter) {
          current.blocked = true;
          const reason = `rate-limit block: run halted after ${limitedStreak} consecutive rate-limited attempts`;
          for (const queued of queue)
            current.records.set(queued, failedRecord(queued, current.attempts.get(queued) ?? 0, reason));
          break;
        }
        if (queue.length > 0) await sleep(cooldownMs);
      }
    }
    current.finishedAt = Date.now();
  }

  async function loadChainIds() {
    const res = await networkFetch.call(window, "https://api.debank.com/chain/list");
    const json = await res.json().catch(() => null);
    if (res.status !== 200 || !Array.isArray(json?.data?.chains))
      throw new Error(`chain/list failed: HTTP ${res.status}`);
    const ids = new Map();
    for (const chain of json.data.chains) {
      const id = Number(chain.network_id);
      if (Number.isSafeInteger(id)) ids.set(chain.id, id);
    }
    return ids;
  }

  async function start(addresses, { timeoutMs = 30000, maxAttempts = 3, cooldownMs = 20000, haltAfter = 3 } = {}) {
    if (run && !run.finishedAt) throw new Error("a run is already active; poll status() until running is false");
    if (!Array.isArray(addresses)) throw new Error("addresses must be an array");
    const invalid = addresses.filter((address) => typeof address !== "string" || !ADDRESS.test(address));
    if (invalid.length > 0) throw new Error(`invalid addresses: ${invalid.map(String).join(", ")}`);
    const unique = [...new Set(addresses.map((address) => address.toLowerCase()))];
    const current = {
      addresses: unique,
      attempts: new Map(),
      records: new Map(),
      rateLimited: 0,
      blocked: false,
      startedAt: Date.now(),
      finishedAt: null,
    };
    run = current;
    try {
      chainIds ??= await loadChainIds();
    } catch (error) {
      run = null;
      throw error;
    }
    drain(current, { timeoutMs, maxAttempts, cooldownMs, haltAfter });
    return { queued: unique.length };
  }

  function status() {
    const records = run ? [...run.records.values()] : [];
    const ok = records.filter((record) => record.status === "ok").length;
    const total = run ? run.addresses.length : 0;
    return {
      running: Boolean(run && !run.finishedAt),
      total,
      done: records.length,
      ok,
      failed: records.length - ok,
      pending: total - records.length,
      rateLimited: run ? run.rateLimited : 0,
      blocked: run ? run.blocked : false,
      startedAt: run ? new Date(run.startedAt).toISOString() : null,
      elapsedMs: run ? (run.finishedAt ?? Date.now()) - run.startedAt : 0,
    };
  }

  const results = () => (run ? run.addresses.map((address) => run.records.get(address)).filter(Boolean) : []);

  window.__debankCollect = { start, status, results };
  return { installed: true, reused: false };
}
