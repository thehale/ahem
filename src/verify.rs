// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::Lang;
use crate::rules::Rule;
use ast_grep_core::tree_sitter::LanguageExt;

enum Expectation {
	Finding,
	Silence,
}

impl Expectation {
	fn met_by(&self, matched: bool) -> bool {
		match self {
			Expectation::Finding => matched,
			Expectation::Silence => !matched,
		}
	}

	fn wanted(&self) -> &'static str {
		match self {
			Expectation::Finding => "expected a finding",
			Expectation::Silence => "expected no finding",
		}
	}
}

pub fn verify(rules: &[Rule]) -> usize {
	let mut failures = 0;
	let mut cases = 0;
	for rule in rules {
		for (lang, snippets) in &rule.tests {
			for source in &snippets.invalid {
				cases += 1;
				failures += reported(rule, *lang, source, Expectation::Finding);
			}
			for source in &snippets.valid {
				cases += 1;
				failures += reported(rule, *lang, source, Expectation::Silence);
			}
		}
	}
	println!("\n{cases} cases, {failures} failed");
	failures
}

fn reported(rule: &Rule, lang: Lang, source: &str, expectation: Expectation) -> usize {
	let matcher = &rule.matchers[&lang];
	let root = lang.ast_grep(source);
	if root.root().dfs().any(|node| node.is_error() || node.is_missing()) {
		println!("FAIL {}.{lang}: snippet is not {lang} in {source:?}", rule.id);
		return 1;
	}
	let matched = root.root().find(&matcher.matcher).is_some();
	if expectation.met_by(matched) {
		0
	} else {
		println!("FAIL {}.{lang}: {} in {source:?}", rule.id, expectation.wanted());
		1
	}
}
