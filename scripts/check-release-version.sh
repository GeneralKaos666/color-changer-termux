#!/bin/sh
# Fail unless the pushed tag exactly matches the [package] version in Cargo.toml.
#
# Only the `[package]` table is read, and the comparison is on the whole string,
# so `v1.1.1` never satisfies a crate version of `1.1.10`.
set -eu

tag=${1:?usage: check-release-version.sh <tag>}

version=$(sh "$(dirname "$0")/crate-version.sh")

if [ "v$version" != "$tag" ]; then
  printf 'tag %s does not match crate version v%s\n' "$tag" "$version" >&2
  exit 1
fi
