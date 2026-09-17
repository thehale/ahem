#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

setup() {
	REPO="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
	RELEASE="$REPO/bin/${BATS_TEST_FILENAME##*/}"
	RELEASE="${RELEASE%.bats}"

	unset "${!GIT_@}"
	unset CI

	export GIT_CONFIG_GLOBAL=/dev/null
	export GIT_CONFIG_SYSTEM=/dev/null

	ORIGIN="${BATS_TEST_TMPDIR:?}/origin.git"

	mkdir --parents "$BATS_TEST_TMPDIR/work"
	cd "$BATS_TEST_TMPDIR/work"
	release_history
}

release_history() {
	git init --quiet --initial-branch main .
	git remote add origin https://example.invalid/unpublished
	cp "$REPO/Cargo.toml" Cargo.toml
	git add Cargo.toml
	commit "first"
	commit "second"
}

commit() {
	git -c user.email=a@b -c user.name=a commit --quiet --allow-empty --message "$1"
}

version() {
	sed --quiet --regexp-extended 's%^version = "(.*)"%\1%p' Cargo.toml | head --lines 1
}

serve() {
	git clone --quiet --bare . "$ORIGIN"
	git remote set-url origin "$ORIGIN"
}

@test "refuses to publish from CI" {
	CI=1 run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"CI publishes nothing"* ]]
}

@test "refuses to publish from a branch that is not main" {
	git checkout --quiet -b release

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"published from main"* ]]
}

@test "refuses to publish a working tree with changes" {
	echo "stray" >stray.txt

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"working tree has changes"* ]]
}

@test "refuses to publish a commit that carries no release tag" {
	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"no vX.Y.Z tag"* ]]
}

@test "refuses to publish when the tag and the manifest disagree" {
	git tag v9.9.9

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"different releases"* ]]
}

@test "refuses to publish what origin has never heard of" {
	git tag "v$(version)"
	git init --quiet --bare "$ORIGIN"
	git remote set-url origin "$ORIGIN"

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"origin has no main"* ]]
}

@test "refuses to publish a commit that never left the workstation" {
	git tag "v$(version)"
	serve
	commit "third"
	git tag --force "v$(version)"

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"origin/main is a different commit"* ]]
}

@test "refuses to publish a tag that never left the workstation" {
	serve
	git tag "v$(version)"

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"origin has no v$(version)"* ]]
}

@test "refuses to publish when origin's tag names another commit" {
	git tag "v$(version)"
	serve
	git --git-dir "$ORIGIN" update-ref "refs/tags/v$(version)" "$(git rev-parse HEAD~1)"

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"names a different commit"* ]]
}

@test "refuses to publish when the manifest points away from origin" {
	git tag "v$(version)"
	serve

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"is not origin"* ]]
}
