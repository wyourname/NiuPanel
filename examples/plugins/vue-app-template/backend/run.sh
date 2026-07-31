#!/usr/bin/env bash
set -euo pipefail

request="$(cat)"
request_id="$(printf '%s' "$request" | sed -n 's/.*"request_id":"\([^"]*\)".*/\1/p')"

printf '{"request_id":"%s","ok":true,"output":{"message":"vue-app-template backend is reachable"}}\n' "$request_id"
