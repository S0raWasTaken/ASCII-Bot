#!/usr/bin/env bash

XOR_KEY=66 # 0-255

fatal() {
    echo -e "\033[31m[fatal]\033[0m" "$@" >&2
    echo "" >&2
    echo "  Usage: ./xor_token.sh <DISCORD_TOKEN>" >&2
    echo "" >&2
    echo "Takes an argument and transforms it into a XOR'ed token file." >&2
    exit 1
}

[ -z "$1" ] && {
    fatal "Missing TOKEN argument"
}

printf "%s" "$1" | while IFS= read -r -n1 char; do
    printf '%b' "\\x$(printf '%02x' $(($(printf '%d' "'$char") ^ XOR_KEY)))"
done > ".token.xor"
