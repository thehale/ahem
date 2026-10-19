#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

bats_require_minimum_version 1.5.0

setup() {
	PORTABLE="$BATS_TEST_DIRNAME/../portable"

	cd "${BATS_TEST_TMPDIR:?}"
}

script() {
	printf '#!/usr/bin/env bash\n%s\n' "$1" >script.sh
	echo script.sh
}

@test "accepts a script whose flags every userland reads" {
	run "$PORTABLE" <<<"$(script 'mkdir -p "$d"')"

	[ "$status" -eq 0 ]
}

@test "rejects a long option to a POSIX tool" {
	run "$PORTABLE" <<<"$(script 'mkdir --parents "$d"')"

	[ "$status" -eq 1 ]
	[[ "$output" == *"script.sh:2"* ]]
}

@test "rejects the long options that broke setup on a Mac" {
	local flag
	for flag in 'mkdir --parents d' 'ln --symbolic a b' 'rm --recursive --force d' \
		'mktemp --directory' 'sed --quiet p f' 'cut --fields 1' 'grep --only-matching x f'; do
		run "$PORTABLE" <<<"$(script "$flag")"
		[ "$status" -eq 1 ] || {
			echo "not rejected: $flag"
			return 1
		}
	done
}

@test "rejects GNU-only spellings that have no long form" {
	local flag
	for flag in "find . -printf '%f\\n'" 'sed -i s/a/b/ f' 'grep -P x f' \
		'date -d yesterday' 'stat -c %s f'; do
		run "$PORTABLE" <<<"$(script "$flag")"
		[ "$status" -eq 1 ] || {
			echo "not rejected: $flag"
			return 1
		}
	done
}

@test "leaves another program's long option alone" {
	local line
	for line in 'xargs shfmt --write' 'xargs shellcheck --external-sources' \
		'npm run --silent build -w web' './gradlew --quiet :shared:assemble'; do
		run "$PORTABLE" <<<"$(script "$line")"
		[ "$status" -eq 0 ] || {
			echo "wrongly rejected: $line"
			return 1
		}
	done
}

@test "names every offending line, not just the first" {
	run "$PORTABLE" <<<"$(printf '#!/usr/bin/env bash\nmkdir --parents a\nln --symbolic b c\n' >s.sh && echo s.sh)"

	[ "$status" -eq 1 ]
	[[ "$output" == *"s.sh:2"* ]]
	[[ "$output" == *"s.sh:3"* ]]
}

@test "accepts empty input" {
	run "$PORTABLE" </dev/null

	[ "$status" -eq 0 ]
}
