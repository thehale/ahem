#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

bats_require_minimum_version 1.5.0

setup() {
	FILES="$BATS_TEST_DIRNAME/../files"

	unset "${!GIT_@}"

	cd "${BATS_TEST_TMPDIR:?}"
	git init --quiet .
}

track() {
	git add --intent-to-add "$@"
}

@test "fails when given neither an extension nor a shebang" {
	run --separate-stderr "$FILES"

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"--extension"* ]]
}

@test "fails on an unknown option" {
	run --separate-stderr "$FILES" --nonsense

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"Usage:"* ]]
}

@test "fails when an option is missing its value" {
	run --separate-stderr "$FILES" --extension

	[ "$status" -ne 0 ]
}

@test "selects a file by its extension, and skips other extensions" {
	printf 'kept\n' >kept.txt
	printf 'skipped\n' >skipped.test
	track kept.txt skipped.test

	run "$FILES" --extension txt

	[ "$output" = 'kept.txt' ]
}

@test "selects a file by its shebang, and skips other shebangs" {
	printf '#!/usr/bin/env kept\n' >kept
	printf '#!/usr/bin/env skipped\n' >skipped
	track kept skipped

	run "$FILES" --shebang kept

	[ "$output" = 'kept' ]
}

@test "outputs nothing in an empty repo" {
	run "$FILES" --extension txt --shebang kept

	[ "$output" = '' ]
}

@test "unions several extensions" {
	printf 'a\n' >a.txt
	printf 'b\n' >b.test
	track a.txt b.test

	run "$FILES" --extension txt --extension test

	[ "$output" = "$(printf 'a.txt\nb.test')" ]
}

@test "unions extensions and shebangs" {
	printf 'a\n' >a.txt
	printf '#!/usr/bin/env kept\n' >tool
	track a.txt tool

	run "$FILES" --extension txt --shebang kept

	[ "$output" = "$(printf 'a.txt\ntool')" ]
}

@test "lists a file once when both its extension and shebang match" {
	printf '#!/usr/bin/env kept\n' >both.txt
	track both.txt

	run "$FILES" --extension txt --shebang kept

	[ "$output" = 'both.txt' ]
}

@test "matches at any depth, and sorts its output" {
	mkdir -p a/b
	printf 'deep\n' >a/b/deep.txt
	printf 'top\n' >top.txt
	track a/b/deep.txt top.txt

	run "$FILES" --extension txt

	[ "$output" = "$(printf 'a/b/deep.txt\ntop.txt')" ]
}

@test "includes an untracked file, but omits a gitignored one" {
	printf 'kept\n' >kept.txt
	printf 'ignored\n' >ignored.txt
	printf 'ignored.txt\n' >.gitignore

	run "$FILES" --extension txt

	[ "$output" = 'kept.txt' ]
}

@test "omits a file deleted from the worktree" {
	printf 'gone\n' >gone.txt
	git add gone.txt
	git -c user.email=t@t -c user.name=t commit --quiet -m init
	rm gone.txt

	run "$FILES" --extension txt

	[ "$output" = '' ]
}

@test "omits vendored files at any depth, keeping lookalike paths" {
	mkdir -p vendor deep/vendor vendored
	printf 'a\n' >vendor/a.txt
	printf 'b\n' >deep/vendor/b.txt
	printf 'c\n' >vendored/c.txt
	track vendor/a.txt deep/vendor/b.txt vendored/c.txt

	run "$FILES" --extension txt

	[ "$output" = 'vendored/c.txt' ]
}
