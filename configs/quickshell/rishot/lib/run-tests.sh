#!/bin/sh
# Runs every lib unit test. No framework, no deps beyond node.
set -eu
cd "$(dirname "$0")"
status=0
for t in *.test.mjs; do
	printf '\n# %s\n' "$t"
	node "$t" || status=1
done
exit "$status"
