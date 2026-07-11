#!/usr/bin/env -S uv run --script
"""Build a CoinGecko historical-data URL around one ISO date."""

from __future__ import annotations

import argparse
import re
from datetime import date, timedelta
from urllib.parse import quote


COIN_ID_RE = re.compile(r"^[a-z0-9]+(?:-[a-z0-9]+)*$")


def build_url(coin_id: str, raw_date: str) -> str:
    if not COIN_ID_RE.fullmatch(coin_id):
        raise ValueError("coin-id must be a lowercase CoinGecko slug")
    target = date.fromisoformat(raw_date)
    start = target - timedelta(days=1)
    end = target + timedelta(days=1)
    return (
        f"https://coingecko.com/en/coins/{quote(coin_id, safe='')}/historical_data"
        f"?start={start.isoformat()}&end={end.isoformat()}"
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("coin_id")
    parser.add_argument("date")
    args = parser.parse_args()
    try:
        print(build_url(args.coin_id, args.date))
    except ValueError as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
