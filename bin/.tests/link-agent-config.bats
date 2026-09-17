#!/usr/bin/env bats
# Copyright (c) Joseph Hale, 2026
# SPDX-License-Identifier: MPL-2.0

setup() {
	REPO="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"

	cd "${BATS_TEST_TMPDIR:?}"

	echo "conventions" >AGENTS.md
	mkdir --parents .agents/skills/example
	echo "a skill" >.agents/skills/example/SKILL.md
}

@test "links the places Claude Code looks at the committed config" {
	run "$REPO/bin/link-agent-config"

	[ "$status" -eq 0 ]
	[ "$(readlink CLAUDE.md)" = 'AGENTS.md' ]
	[ "$(readlink .claude/skills)" = '../.agents/skills' ]
}

@test "serves the committed files through the links" {
	"$REPO/bin/link-agent-config"

	[ "$(cat CLAUDE.md)" = 'conventions' ]
	[ "$(cat .claude/skills/example/SKILL.md)" = 'a skill' ]
}

@test "says nothing the second time" {
	"$REPO/bin/link-agent-config"

	run "$REPO/bin/link-agent-config"

	[ "$status" -eq 0 ]
	[ "$output" = '' ]
}

@test "keeps a directory of its own beside the linked skills" {
	mkdir .claude
	echo '{}' >.claude/settings.local.json

	run "$REPO/bin/link-agent-config"

	[ "$status" -eq 0 ]
	[ "$(readlink .claude/skills)" = '../.agents/skills' ]
	[ -f .claude/settings.local.json ]
}

@test "leaves a real file where a link would go alone" {
	echo "mine" >CLAUDE.md

	run "$REPO/bin/link-agent-config"

	[ "$status" -eq 0 ]
	[ "$(cat CLAUDE.md)" = 'mine' ]
	[[ "$output" == *"CLAUDE.md"* ]]
}

@test "leaves a link pointing somewhere else alone" {
	ln --symbolic elsewhere.md CLAUDE.md

	run "$REPO/bin/link-agent-config"

	[ "$status" -eq 0 ]
	[ "$(readlink CLAUDE.md)" = 'elsewhere.md' ]
	[[ "$output" == *"CLAUDE.md"* ]]
}
