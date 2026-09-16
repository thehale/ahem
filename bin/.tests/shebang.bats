#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

bats_require_minimum_version 1.5.0

WANTED='#!/usr/bin/env bash'

setup() {
	REPO="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
	cd "${BATS_TEST_TMPDIR:?}"

	printf '%s\necho right\n' "$WANTED" >right.sh
	printf '#!/bin/sh\necho wrong\n' >wrong.sh
	printf 'echo bare\n' >bare.sh
}

@test "passes a file whose shebang matches" {
	run "$REPO/bin/shebang" "$WANTED" <<<"right.sh"

	[ "$status" -eq 0 ]
}

@test "fails and names the offender on stderr" {
	run --separate-stderr "$REPO/bin/shebang" "$WANTED" <<<"wrong.sh"

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"wrong.sh"* ]]
}

@test "reports every offender, not just the first" {
	printf '#!/bin/zsh\n' >alsowrong.sh

	run --separate-stderr "$REPO/bin/shebang" "$WANTED" <<<"$(printf 'wrong.sh\nalsowrong.sh\n')"

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"wrong.sh"* ]]
	[[ "$stderr" == *"alsowrong.sh"* ]]
}

@test "ignores a file with no shebang at all" {
	run "$REPO/bin/shebang" "$WANTED" <<<"bare.sh"

	[ "$status" -eq 0 ]
}

@test "accepts empty stdin" {
	run "$REPO/bin/shebang" "$WANTED" </dev/null

	[ "$status" -eq 0 ]
}

@test "--fix rewrites a wrong shebang in place" {
	run "$REPO/bin/shebang" --fix "$WANTED" <<<"wrong.sh"

	[ "$status" -eq 0 ]
	[ "$(head -1 wrong.sh)" = "$WANTED" ]
}

@test "--fix leaves the rest of the file alone" {
	"$REPO/bin/shebang" --fix "$WANTED" <<<"wrong.sh"

	[ "$(sed -n '2 p' wrong.sh)" = 'echo wrong' ]
	[ "$(wc -l <wrong.sh)" -eq 2 ]
}

@test "--fix does not add a shebang to a file without one" {
	"$REPO/bin/shebang" --fix "$WANTED" <<<"bare.sh"

	[ "$(head -1 bare.sh)" = 'echo bare' ]
}

@test "exits with usage when given no shebang" {
	run --separate-stderr "$REPO/bin/shebang" </dev/null

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"Usage:"* ]]
}
