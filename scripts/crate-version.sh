#!/bin/sh
# Print the `[package]` version from Cargo.toml.
#
# Only the `[package]` table is read, so a dependency that spells out its
# version cannot be mistaken for the crate's own.
set -eu

version=$(awk '
  /^\[package\]/ { in_package = 1; next }
  /^\[/ { in_package = 0 }
  in_package && /^version[[:space:]]*=/ {
    gsub(/^version[[:space:]]*=[[:space:]]*"|".*$/, "")
    print
    exit
  }
' Cargo.toml)

if [ -z "$version" ]; then
  printf 'no [package] version in Cargo.toml\n' >&2
  exit 1
fi

printf '%s\n' "$version"
