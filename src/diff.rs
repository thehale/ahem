// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::check::{Scope, check, reported, root};
use crate::rules::Rule;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

const REMOVED: &str = "/dev/null";

struct Touched {
	path: PathBuf,
	lines: BTreeSet<usize>,
	added: Vec<(usize, String)>,
}

pub fn diff(rules: &[Rule], source: &str, paths: &[String]) -> usize {
	match given(source) {
		Ok(patch) => examined(rules, &touched(&patch), paths),
		Err(error) => unread(source, error),
	}
}

fn examined(rules: &[Rule], files: &[Touched], paths: &[String]) -> usize {
	match paths.is_empty() {
		true => files.iter().map(|file| inspect(rules, file)).sum(),
		false => paths.iter().map(|path| asked(rules, files, path)).sum(),
	}
}

fn asked(rules: &[Rule], files: &[Touched], path: &str) -> usize {
	match covering(files, path) {
		Some(file) => inspect(rules, file),
		None => check(rules, std::slice::from_ref(&path.to_string())),
	}
}

fn covering<'a>(files: &'a [Touched], path: &str) -> Option<&'a Touched> {
	let wanted = std::fs::canonicalize(path).ok()?;
	files
		.iter()
		.find(|file| resolved(&file.path) == Some(wanted.clone()))
}

fn resolved(named: &Path) -> Option<PathBuf> {
	std::fs::canonicalize(located(named)?).ok()
}

fn given(source: &str) -> Result<String, std::io::Error> {
	let mut patch = String::new();
	match source {
		"-" => std::io::stdin().read_to_string(&mut patch).map(|_| patch),
		named => std::fs::read_to_string(named),
	}
}

fn touched(patch: &str) -> Vec<Touched> {
	let mut files: Vec<Touched> = vec![];
	let mut line = 0;
	for text in patch.lines() {
		if let Some(path) = destination(text) {
			files.push(Touched::new(path));
		} else if let Some(start) = hunk(text) {
			line = start;
		} else if let Some(file) = files.last_mut() {
			line = file.absorb(text, line);
		}
	}
	files.retain(|file| file.path != Path::new(REMOVED));
	files
}

fn destination(text: &str) -> Option<PathBuf> {
	let named = text.strip_prefix("+++ ")?.split('\t').next()?;
	Some(PathBuf::from(named))
}

fn hunk(text: &str) -> Option<usize> {
	let after = text.strip_prefix("@@ ")?.split(" @@").next()?;
	let arriving = after.split_whitespace().find(|side| side.starts_with('+'))?;
	arriving.trim_start_matches('+').split(',').next()?.parse().ok()
}

impl Touched {
	fn new(path: PathBuf) -> Self {
		Touched {
			path,
			lines: BTreeSet::new(),
			added: vec![],
		}
	}

	fn absorb(&mut self, text: &str, line: usize) -> usize {
		match text.chars().next() {
			Some('+') => self.add(text, line),
			Some(' ') => line + 1,
			_ => line,
		}
	}

	fn add(&mut self, text: &str, line: usize) -> usize {
		self.lines.insert(line);
		self.added.push((line, text[1..].to_string()));
		line + 1
	}
}

fn inspect(rules: &[Rule], file: &Touched) -> usize {
	let Some(path) = located(&file.path) else {
		return unfound(&file.path);
	};
	let source = match std::fs::read_to_string(&path) {
		Ok(source) => source,
		Err(error) => return unopened(&path, error),
	};
	match stale(file, &source) {
		true => outdated(&path),
		false => reported(rules, &path, &source, &Scope::Lines(file.lines.clone())),
	}
}

fn located(named: &Path) -> Option<PathBuf> {
	candidates(named).into_iter().find(|path| path.is_file())
}

fn candidates(named: &Path) -> Vec<PathBuf> {
	let bare = unprefixed(named);
	let mut tried = vec![named.to_path_buf(), bare.clone()];
	if let Some(root) = root() {
		tried.push(root.join(named));
		tried.push(root.join(bare));
	}
	tried
}

fn unprefixed(named: &Path) -> PathBuf {
	let mut parts = named.components();
	match parts.next().is_some() {
		true => parts.as_path().to_path_buf(),
		false => named.to_path_buf(),
	}
}

fn unfound(named: &Path) -> usize {
	eprintln!("ahem: {} is in the diff but not on disk", named.display());
	1
}

fn stale(file: &Touched, source: &str) -> bool {
	let lines: Vec<&str> = source.lines().collect();
	file.added
		.iter()
		.any(|(number, text)| lines.get(number - 1) != Some(&text.as_str()))
}

fn outdated(path: &Path) -> usize {
	eprintln!("ahem: {} has moved on from the diff it was given", path.display());
	1
}

fn unopened(path: &Path, error: std::io::Error) -> usize {
	eprintln!("ahem: {}: {error}", path.display());
	1
}

fn unread(source: &str, error: std::io::Error) -> usize {
	eprintln!("ahem: the diff at {source} could not be read: {error}");
	1
}

#[cfg(test)]
mod tests {
	use super::*;

	fn parsed(patch: &str) -> Vec<Touched> {
		touched(patch)
	}

	#[test]
	fn collects_the_lines_a_hunk_adds() {
		let files = parsed("+++ b/a.ts\n@@ -1,0 +1,2 @@\n+one\n+two\n");

		assert_eq!(files.len(), 1);
		assert_eq!(files[0].path, PathBuf::from("b/a.ts"));
		assert_eq!(files[0].lines, BTreeSet::from([1, 2]));
	}

	#[test]
	fn counts_context_lines_and_skips_removed_ones() {
		let files = parsed("+++ b/a.ts\n@@ -4,3 +4,3 @@\n keep\n-gone\n+fresh\n");

		assert_eq!(files[0].lines, BTreeSet::from([5]));
	}

	#[test]
	fn reads_a_hunk_header_that_names_no_count() {
		let files = parsed("+++ b/a.ts\n@@ -7 +9 @@\n+only\n");

		assert_eq!(files[0].lines, BTreeSet::from([9]));
	}

	#[test]
	fn keeps_each_file_in_a_diff_apart() {
		let files = parsed("+++ b/a.ts\n@@ -1 +1 @@\n+first\n+++ b/b.ts\n@@ -9 +9 @@\n+second\n");

		assert_eq!(files.len(), 2);
		assert_eq!(files[0].lines, BTreeSet::from([1]));
		assert_eq!(files[1].lines, BTreeSet::from([9]));
	}

	#[test]
	fn keeps_whatever_prefix_the_diff_used() {
		let files = parsed("+++ w/a.ts\n@@ -1 +1 @@\n+one\n");

		assert_eq!(files[0].path, PathBuf::from("w/a.ts"));
		assert_eq!(unprefixed(&files[0].path), PathBuf::from("a.ts"));
	}

	#[test]
	fn leaves_a_path_that_carries_no_prefix_whole() {
		assert_eq!(candidates(Path::new("a.ts"))[0], PathBuf::from("a.ts"));
	}

	#[test]
	fn drops_a_deleted_file_without_losing_the_next_one() {
		let files = parsed("+++ /dev/null\n@@ -1,2 +0,0 @@\n-one\n-two\n+++ b/b.ts\n@@ -3 +3 @@\n+kept\n");

		assert_eq!(files.len(), 1);
		assert_eq!(files[0].path, PathBuf::from("b/b.ts"));
		assert_eq!(files[0].lines, BTreeSet::from([3]));
	}

	#[test]
	fn remembers_what_each_added_line_said() {
		let files = parsed("+++ b/a.ts\n@@ -1 +1 @@\n+let count = 1;\n");

		assert_eq!(files[0].added, vec![(1, "let count = 1;".to_string())]);
	}
}
