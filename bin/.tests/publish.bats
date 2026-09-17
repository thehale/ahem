#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

setup() {
	REPO="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
	RELEASE="$REPO/bin/${BATS_TEST_FILENAME##*/}"
	RELEASE="${RELEASE%.bats}"

	unset "${!GIT_@}"

	cd "${BATS_TEST_TMPDIR:?}"
	clone
}

clone() {
	git init --quiet --initial-branch main .
	git remote add origin "$(git -C "$REPO" remote get-url origin)"
	cp "$REPO/Cargo.toml" Cargo.toml
	git add Cargo.toml
	git -c user.email=a@b -c user.name=a commit --quiet --message "release"
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

@test "refuses to publish when the manifest points away from origin" {
	git tag "v$(sed --quiet --regexp-extended 's%^version = "(.*)"%\1%p' Cargo.toml | head --lines 1)"
	git remote set-url origin https://github.com/someone/else

	run "$RELEASE"

	[ "$status" -eq 1 ]
	[[ "$output" == *"is not origin"* ]]
}
