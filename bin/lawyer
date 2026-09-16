#!/usr/bin/env bash
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

set -euo pipefail

LICENSOR="Joseph Hale"
LICENSE="MPL-2.0"
YEAR="$(date +%Y)"
CONFIG='
schema_version = 1
project {
  header_ignore = ["**/vendor/**"]
}
'

if [[ "${1:-}" == "--fix" ]]; then
	EXTRA_ARGS=""
else
	EXTRA_ARGS="--plan"
fi

copywrite license "$EXTRA_ARGS" \
	-c "(c) $LICENSOR," \
	--year "$YEAR" \
	--spdx "$LICENSE"

copywrite headers "$EXTRA_ARGS" \
	--config <(echo "$CONFIG") \
	-c "(c) $LICENSOR," \
	--spdx "$LICENSE"
