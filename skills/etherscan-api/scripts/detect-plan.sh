#!/bin/bash
# detect-plan.sh — Detect Etherscan API plan tier from $ETHERSCAN_API_KEY.
#
# Outputs key=value lines on stdout:
#   plan=<free|lite|standard|advanced|professional|pro_plus|enterprise|unknown>
#   credit_limit=<int>
#   credits_used=<int>
#   credits_available=<int>
#   limit_interval=<string>
#   interval_expiry=<HH:MM:SS>
#   pro_endpoints=<true|false>
#
# Cache the result for the session — getapilimit itself consumes 1 credit, and
# the optional PRO probe consumes another.

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

# Map creditLimit → plan.
case "$credit_limit" in
  100000)  plan="free_or_lite";  pro_endpoints="false" ;;
  200000)  plan="standard";      pro_endpoints="true"  ;;
  500000)  plan="advanced";      pro_endpoints="true"  ;;
  1000000) plan="professional";  pro_endpoints="true"  ;;
  1500000) plan="pro_plus";      pro_endpoints="true"  ;;
  *)
    if [ "${credit_limit:-0}" -gt 1500000 ]; then
      plan="enterprise"; pro_endpoints="true"
    else
      plan="unknown"; pro_endpoints="unknown"
    fi
    ;;
esac

# Disambiguate free vs Lite by probing a PRO endpoint (Lite has no PRO; Standard+ does).
if [ "$plan" = "free_or_lite" ]; then
  probe=$(curl -fsS "$base?chainid=1&module=account&action=addresstokenbalance&address=0xde0B295669a9FD93d5F28D9Ec85E40f4cb697BAe&page=1&offset=1&apikey=$ETHERSCAN_API_KEY" 2>/dev/null || printf '{"status":"0"}')
  probe_status=$(extract_str "$probe" "status")
  if [ "$probe_status" = "1" ]; then
    # Unexpected: 100k limit AND PRO access. Report it transparently.
    plan="lite_with_pro"
    pro_endpoints="true"
  else
    plan="free"
    pro_endpoints="false"
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
EOF
