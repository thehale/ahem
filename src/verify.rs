// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::chunk::{Chunk, chunks};
use crate::lang::{Lang, MARKUP};
use crate::rules::Rule;
use crate::say::say;
use ast_grep_core::tree_sitter::LanguageExt;

enum Expectation {
	Finding(Option<usize>),
	Silence,
}

impl Expectation {
	fn met_by(&self, found: Option<usize>) -> bool {
		match self {
			Expectation::Finding(None) => found.is_some(),
			Expectation::Finding(line) => found == *line,
			Expectation::Silence => found.is_none(),
		}
	}

	fn wanted(&self) -> String {
		match self {
			Expectation::Finding(None) => "expected a finding".to_string(),
			Expectation::Finding(Some(line)) => format!("expected a finding on line {line}"),
			Expectation::Silence => "expected no finding".to_string(),
		}
	}
}

pub fn verify(rules: &[Rule]) -> usize {
	let mut failures = 0;
	let mut cases = 0;
	for rule in rules {
		for (lang, snippets) in &rule.tests {
			for flagged in &snippets.invalid {
				cases += 1;
				let expectation = Expectation::Finding(flagged.line());
				failures += reported(rule, *lang, flagged.source(), expectation);
			}
			for source in &snippets.valid {
				cases += 1;
				failures += reported(rule, *lang, source, Expectation::Silence);
			}
		}
	}
	say(&format!("\n{cases} cases, {failures} failed"));
	failures
}

fn reported(rule: &Rule, lang: Lang, source: &str, expectation: Expectation) -> usize {
	let chunks = chunks(lang.ast_grep(source), Some(MARKUP));
	if let Some(chunk) = chunks.iter().find(|chunk| broken(chunk)) {
		say(&format!(
			"FAIL {}.{lang}: snippet is not {} in {source:?}",
			rule.id,
			chunk.lang()
		));
		return 1;
	}
	let found = chunks
		.iter()
		.flat_map(|chunk| chunk.found(rule))
		.map(|node| node.start_pos().line() + 1)
		.min();
	if expectation.met_by(found) {
		0
	} else {
		say(&format!(
			"FAIL {}.{lang}: {} in {source:?}",
			rule.id,
			expectation.wanted()
		));
		1
	}
}

fn broken(chunk: &Chunk) -> bool {
	chunk
		.tree
		.root()
		.dfs()
		.any(|node| node.is_error() || node.is_missing())
}
