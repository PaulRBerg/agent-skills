#!/bin/bash
# etherscan-detect-plan.sh — Detect the API plan through the official Etherscan CLI.
#
# Outputs key=value lines on stdout:
#   plan=<free|lite|standard|advanced|professional|pro_plus|enterprise|unknown>
#   credit_limit=<int>
#   credits_used=<int>
#   credits_available=<int>
#   limit_interval=<string>
#   interval_expiry=<HH:MM:SS>
#   pro_endpoints=<true|false|unknown>
#   paid_chains=<true|false|unknown>
#
# Cache the result for the session. apilimit consumes 1 credit.
# The paid-chain probe, when needed, consumes another credit.

set -eu

if ! command -v etherscan >/dev/null 2>&1; then
  echo "Error: etherscan CLI is not installed" >&2
  exit 1
fi

if ! response=$(etherscan apilimit --chain 1 --output json 2>/dev/null); then
  echo "Error: etherscan apilimit failed. Check CLI credentials, quota, and network access" >&2
  exit 1
fi

if ! fields=$(printf '%s' "$response" | jq -ser '
  def credit: type == "number" and . >= 0 and floor == .;
  def interval: type == "string" and length > 0 and (test("[\u0000-\u001f\u007f]") | not);
  select(length == 1) | .[0] | select(type == "object") |
  select([.creditLimit, .creditsUsed, .creditsAvailable] | all(credit)) |
  select([.limitInterval, .intervalExpiryTimespan] | all(interval)) |
  [.creditLimit, .creditsUsed, .creditsAvailable, .limitInterval, .intervalExpiryTimespan] | @tsv
' 2>/dev/null); then
  echo "Error: etherscan apilimit returned invalid credit data" >&2
  exit 1
fi
IFS=$'\t' read -r credit_limit credits_used credits_avail interval expiry <<EOF
$fields
EOF

# Free and Lite share the 100k daily credit limit.
# Paid-chain access distinguishes them. Standard+ needs no probe.
case "$credit_limit" in
  100000)  plan="free_or_lite";  pro_endpoints="false"; paid_chains="probe" ;;
  200000)  plan="standard";      pro_endpoints="true";  paid_chains="true"  ;;
  500000)  plan="advanced";      pro_endpoints="true";  paid_chains="true"  ;;
  1000000) plan="professional";  pro_endpoints="true";  paid_chains="true"  ;;
  1500000) plan="pro_plus";      pro_endpoints="true";  paid_chains="true"  ;;
  *)
    if [ "${credit_limit:-0}" -gt 1500000 ]; then
      plan="enterprise"; pro_endpoints="true"; paid_chains="true"
    else
      plan="unknown"; pro_endpoints="unknown"; paid_chains="unknown"
    fi
    ;;
esac

# Probe Base access. Success proves Lite. An explicit Free-tier denial proves Free.
# Other failures leave access unknown. Both possible plans lack PRO endpoints.
if [ "$plan" = "free_or_lite" ]; then
  plan="unknown"
  paid_chains="unknown"
  probe_error=$(mktemp)
  trap 'rm -f "$probe_error"' EXIT
  if probe=$(etherscan account balance --chain 8453 --output json \
    --address 0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe --tag latest 2>"$probe_error"); then
    if printf '%s' "$probe" | jq -se 'length == 1 and (.[0] | type == "string" and test("^[0-9]+$"))' >/dev/null 2>&1; then
      plan="lite"
      paid_chains="true"
    fi
  elif grep -qF 'Free API access is not supported for this chain.' "$probe_error"; then
    plan="free"
    paid_chains="false"
  fi
  if [ "$plan" = "unknown" ]; then
    echo "Warning: paid-chain probe inconclusive; plan and paid-chain access remain unknown" >&2
  fi
fi

cat <<EOF
plan=$plan
credit_limit=$credit_limit
credits_used=$credits_used
credits_available=$credits_avail
limit_interval=$interval
interval_expiry=$expiry
pro_endpoints=$pro_endpoints
paid_chains=$paid_chains
EOF
