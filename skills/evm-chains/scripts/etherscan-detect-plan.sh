#!/bin/bash
# etherscan-detect-plan.sh — Detect Etherscan API plan tier from $ETHERSCAN_API_KEY.
#
# Outputs key=value lines on stdout:
#   plan=<free|lite|standard|advanced|professional|pro_plus|enterprise|unknown>
#   credit_limit=<int>
#   credits_used=<int>
#   credits_available=<int>
#   limit_interval=<string>
#   interval_expiry=<HH:MM:SS>
#   pro_endpoints=<true|false>
#   paid_chains=<true|false>
#
# Cache the result for the session — getapilimit itself consumes 1 credit, and
# the paid-chain probe (when needed) consumes another.

set -eu

if [ -z "${ETHERSCAN_API_KEY:-}" ]; then
  echo "Error: ETHERSCAN_API_KEY is not set" >&2
  exit 1
fi

base="https://api.etherscan.io/v2/api"
response=$(curl -fsS "$base?chainid=1&module=getapilimit&action=getapilimit&apikey=$ETHERSCAN_API_KEY")

extract_num() {
  printf '%s' "$1" | grep -o "\"$2\":[0-9]*" | head -1 | grep -o '[0-9]*'
}
extract_str() {
  printf '%s' "$1" | grep -o "\"$2\":\"[^\"]*\"" | head -1 | sed 's/.*:"\([^"]*\)"/\1/'
}

status=$(extract_str "$response" "status")
if [ "$status" != "1" ]; then
  msg=$(extract_str "$response" "message")
  res=$(extract_str "$response" "result")
  echo "Error: getapilimit failed — message=$msg result=$res" >&2
  exit 1
fi

credit_limit=$(extract_num "$response" "creditLimit")
credits_used=$(extract_num "$response" "creditsUsed")
credits_avail=$(extract_num "$response" "creditsAvailable")
interval=$(extract_str "$response" "limitInterval")
expiry=$(extract_str "$response" "intervalExpiryTimespan")

# Map creditLimit → plan. Free and Lite share the 100k daily credit; the
# difference is paid-only-chain access. Standard+ implies PRO endpoints and
# paid-chain access; no probe needed.
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

# Lite ($49/mo) unlocks the paid Etherscan target chains (Base, OP,
# Avalanche, BNB) while Free does not. Probe a Base balance to disambiguate: status=1 → Lite,
# status=0 → Free. PRO endpoints stay false on Lite (Standard plan and up).
if [ "$plan" = "free_or_lite" ]; then
  probe=$(curl -fsS "$base?chainid=8453&module=account&action=balance&address=0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe&tag=latest&apikey=$ETHERSCAN_API_KEY" 2>/dev/null || printf '{"status":"0"}')
  probe_status=$(extract_str "$probe" "status")
  if [ "$probe_status" = "1" ]; then
    plan="lite"
    paid_chains="true"
  else
    plan="free"
    paid_chains="false"
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
