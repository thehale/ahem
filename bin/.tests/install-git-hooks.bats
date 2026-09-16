#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

setup() {
	REPO="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"

	unset "${!GIT_@}"

	cd "${BATS_TEST_TMPDIR:?}"
}

@test "points git at the hooks in bin/.hooks" {
	git init --quiet .

	run "$REPO/bin/install-git-hooks"

	[ "$status" -eq 0 ]
	[ "$(git config core.hooksPath)" = 'bin/.hooks' ]
}

@test "leaves a directory that is not a repository alone" {
	mkdir plain
	cd plain

	run "$REPO/bin/install-git-hooks"

	[ "$status" -eq 0 ]
	[[ "$output" == *"Not a git repository"* ]]
}
