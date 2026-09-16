// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::Lang;
use crate::rules::Rule;
use ast_grep_config::Severity;
use ast_grep_core::tree_sitter::LanguageExt;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

const SKIP_HIDDEN: bool = false;

pub fn check(rules: &[Rule], paths: &[String]) -> usize {
	walked(paths)
		.iter()
		.map(|found| match found {
			Ok(path) => inspect(rules, path),
			Err(error) => unreachable(error),
		})
		.sum()
}

fn unreachable(error: &str) -> usize {
	eprintln!("deconfuse: {error}");
	1
}

fn walked(paths: &[String]) -> Vec<Result<PathBuf, String>> {
	let mut roots = paths.iter();
	let first = roots.next().cloned().unwrap_or_else(|| ".".to_string());
	let mut builder = WalkBuilder::new(first);
	for root in roots {
		builder.add(root);
	}
	builder
		.hidden(SKIP_HIDDEN)
		.build()
		.filter(|found| match found {
			Ok(entry) => entry.file_type().is_some_and(|kind| kind.is_file()),
			Err(_) => true,
		})
		.map(|found| match found {
			Ok(entry) => Ok(entry.into_path()),
			Err(error) => Err(error.to_string()),
		})
		.collect()
}

fn inspect(rules: &[Rule], path: &Path) -> usize {
	let Ok(source) = std::fs::read_to_string(path) else {
		return 0;
	};
	let Some(lang) = Lang::of(path, source.lines().next()) else {
		return 0;
	};
	let root = lang.ast_grep(&source);
	let mut found = 0;
	for rule in applicable(rules, lang) {
		let matcher = &rule.matchers[&lang];
		for node in root.root().find_all(&matcher.matcher) {
			let line = node.start_pos().line() + 1;
			println!("{}:{line}: {} [{}]", path.display(), matcher.message, rule.id);
			found += 1;
		}
	}
	found
}

fn applicable(rules: &[Rule], lang: Lang) -> impl Iterator<Item = &Rule> {
	rules
		.iter()
		.filter(move |rule| rule.matchers.contains_key(&lang))
		.filter(|rule| !matches!(rule.severity, Severity::Off))
}
