#!/usr/bin/env bash
set -euo pipefail

registry_host="//registry.npmjs.org/:_authToken="
npmrc_path="${NPMRC_PATH:-$HOME/.npmrc}"
token="${NPM_TOKEN:-}"

if [[ -z "${token}" ]]; then
  read -r -s -p "npm access token: " token
  printf '\n'
fi

if [[ -z "${token}" ]]; then
  echo "No npm token provided." >&2
  exit 1
fi

tmp_file="$(mktemp)"
trap 'rm -f "$tmp_file"' EXIT

if [[ -f "${npmrc_path}" ]]; then
  grep -v '^//registry\.npmjs\.org/:_authToken=' "${npmrc_path}" | \
    grep -v '^always-auth=' > "${tmp_file}" || true
fi

{
  cat "${tmp_file}"
  echo "${registry_host}${token}"
  echo "always-auth=true"
} > "${npmrc_path}"

chmod 0600 "${npmrc_path}"

echo "Wrote npm auth token to ${npmrc_path}"
echo "Next step: npm whoami"
