// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::Lang;
use crate::rules::Rule;
use crate::say::say;
use ast_grep_config::Severity;
use ast_grep_core::tree_sitter::LanguageExt;
use ignore::WalkBuilder;
use std::collections::{BTreeSet, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

const SKIP_HIDDEN: bool = false;
const REPOSITORY: &str = ".git";

pub fn check(rules: &[Rule], paths: &[String]) -> usize {
	walked(paths)
		.iter()
		.map(|found| match found {
			Ok(path) => inspect(rules, path),
			Err(error) => unopened(error.clone()),
		})
		.sum()
}

fn unopened(error: String) -> usize {
	eprintln!("ahem: {error}");
	1
}

fn walked(paths: &[String]) -> Vec<Result<PathBuf, String>> {
	let mut roots = paths.iter();
	let first = roots.next().cloned().unwrap_or_else(|| ".".to_string());
	let mut builder = WalkBuilder::new(first);
	for root in roots {
		builder.add(root);
	}
	let reviewed = reviewed(paths);
	builder
		.hidden(SKIP_HIDDEN)
		.filter_entry(|entry| entry.file_name() != REPOSITORY)
		.build()
		.filter(|found| match found {
			Ok(entry) => entry.file_type().is_some_and(|kind| kind.is_file()),
			Err(_) => true,
		})
		.filter(|found| match found {
			Ok(entry) => reviewed.covers(entry.path()),
			Err(_) => true,
		})
		.map(|found| match found {
			Ok(entry) => Ok(entry.into_path()),
			Err(error) => Err(error.to_string()),
		})
		.collect()
}

struct Reviewed {
	repository: Option<Repository>,
	named: HashSet<PathBuf>,
}

struct Repository {
	root: PathBuf,
	tracked: HashSet<PathBuf>,
}

impl Reviewed {
	fn covers(&self, path: &Path) -> bool {
		let Some(repository) = &self.repository else {
			return true;
		};
		let Ok(full) = std::fs::canonicalize(path) else {
			return true;
		};
		if !full.starts_with(&repository.root) {
			return true;
		}
		repository.tracked.contains(&full) || self.named.contains(&full)
	}
}

fn reviewed(paths: &[String]) -> Reviewed {
	Reviewed {
		repository: repository(),
		named: paths.iter().filter_map(whole).collect(),
	}
}

pub fn root() -> Option<PathBuf> {
	let shown = spoken(&["rev-parse", "--show-toplevel"])?;
	std::fs::canonicalize(shown.trim()).ok()
}

fn repository() -> Option<Repository> {
	let root = root()?;
	let listed = spoken(&["-C", root.to_str()?, "ls-files", "-z"])?;
	let tracked = listed
		.split('\0')
		.filter(|name| !name.is_empty())
		.filter_map(|name| whole(root.join(name)))
		.collect();
	Some(Repository { root, tracked })
}

pub fn spoken(args: &[&str]) -> Option<String> {
	let said = Command::new("git").args(args).output().ok()?;
	match said.status.success() {
		true => Some(String::from_utf8_lossy(&said.stdout).into_owned()),
		false => None,
	}
}

fn whole<P: AsRef<Path>>(path: P) -> Option<PathBuf> {
	std::fs::canonicalize(path).ok().filter(|full| full.is_file())
}

fn inspect(rules: &[Rule], path: &Path) -> usize {
	let source = match std::fs::read_to_string(path) {
		Ok(source) => source,
		Err(error) => return skipped(path, error),
	};
	reported(rules, path, &source, &Scope::Whole)
}

pub enum Scope {
	Whole,
	Lines(BTreeSet<usize>),
}

impl Scope {
	fn covers(&self, first: usize, last: usize) -> bool {
		match self {
			Scope::Whole => true,
			Scope::Lines(lines) => lines.range(first..=last).next().is_some(),
		}
	}
}

pub fn reported(rules: &[Rule], path: &Path, source: &str, scope: &Scope) -> usize {
	let Some(lang) = Lang::of(path, source.lines().next()) else {
		return 0;
	};
	let root = lang.ast_grep(source);
	let mut found = 0;
	for rule in applicable(rules, lang) {
		let matcher = &rule.matchers[&lang];
		for node in root.root().find_all(&matcher.matcher) {
			let first = node.start_pos().line() + 1;
			if !scope.covers(first, node.end_pos().line() + 1) {
				continue;
			}
			say(&format!(
				"{}:{first}: {} [{}]",
				path.display(),
				matcher.message,
				rule.id
			));
			found += 1;
		}
	}
	found
}

fn skipped(path: &Path, error: std::io::Error) -> usize {
	match error.kind() {
		std::io::ErrorKind::InvalidData => 0,
		_ => unopened(format!("{}: {error}", path.display())),
	}
}

fn applicable(rules: &[Rule], lang: Lang) -> impl Iterator<Item = &Rule> {
	rules
		.iter()
		.filter(move |rule| rule.matchers.contains_key(&lang))
		.filter(|rule| !matches!(rule.severity, Severity::Off))
}
