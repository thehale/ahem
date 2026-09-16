#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

bats_require_minimum_version 1.5.0

setup() {
	source "$BATS_TEST_DIRNAME/../filelike"

	unset "${!GIT_@}"

	cd "${BATS_TEST_TMPDIR:?}"
	git init --quiet .
}

track() {
	git add --intent-to-add "$@"
}

@test "fails when given no language" {
	run --separate-stderr filelike

	[ "$status" -eq 1 ]
	[[ "$stderr" == *"language"* ]]
}

@test "takes a language as its extension" {
	printf 'kept\n' >kept.txt
	printf 'skipped\n' >skipped.test
	track kept.txt skipped.test

	run filelike txt

	[ "$output" = 'kept.txt' ]
}

@test "bash also selects .sh files and bash shebangs" {
	printf 'echo hi\n' >a.bash
	printf 'echo hi\n' >b.sh
	printf '#!/usr/bin/env bash\n' >tool
	printf '#!/usr/bin/env python3\n' >skipped
	track a.bash b.sh tool skipped

	run filelike bash

	[ "$output" = "$(printf 'a.bash\nb.sh\ntool')" ]
}
