// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::check::{Scope, findings};
use crate::rules::Rule;
use crate::say::say;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::{Read, Write};
use std::path::PathBuf;

const ADMONITION: &str = "\
ahem read the file the tool just touched. Each line below names a principle the
code breaks. Hold to every one of them. The bar for rejecting one is
stratospheric: a direct operator instruction to the contrary, nearly universal
precedent throughout the codebase, and nothing less. Rejecting one means saying
out loud what makes this code the exception.";

const WRITTEN: &[&str] = &["content", "new_string", "new_source", "text"];
const ENVELOPE: &str = "*** Begin Patch";
const JOURNAL: &str = "AHEM_DEBUG_SHOW_HOOK_PAYLOAD";
const ALWAYS: bool = true;
const TOUCHED: &[&str] = &["file_path", "notebook_path", "path"];

struct Agent {
	speaks: fn(&Value) -> bool,
	envelope: fn(&str) -> Value,
}

const AGENTS: &[Agent] = &[
	Agent {
		speaks: codex,
		envelope: posted,
	},
	Agent {
		speaks: claude,
		envelope: posted,
	},
];

fn codex(payload: &Value) -> bool {
	payload.get("turn_id").is_some()
}

fn claude(payload: &Value) -> bool {
	payload.get("hook_event_name").is_some()
}

fn posted(context: &str) -> Value {
	json!({
		"hookSpecificOutput": {
			"hookEventName": "PostToolUse",
			"additionalContext": context,
		}
	})
}

pub fn hook(rules: &[Rule]) -> usize {
	let Some(payload) = heard() else {
		return 0;
	};
	let Some(agent) = AGENTS.iter().find(|agent| (agent.speaks)(&payload)) else {
		return 0;
	};
	let found = read(rules, &payload);
	if !found.is_empty() {
		say(&(agent.envelope)(&spoken(&found)).to_string());
	}
	0
}

fn heard() -> Option<Value> {
	let mut payload = String::new();
	std::io::stdin().read_to_string(&mut payload).ok()?;
	kept(&payload);
	serde_json::from_str(&payload).ok()
}

fn kept(payload: &str) {
	let Ok(path) = std::env::var(JOURNAL) else {
		return;
	};
	let opened = std::fs::OpenOptions::new()
		.create(ALWAYS)
		.append(ALWAYS)
		.open(path);
	if let Ok(mut journal) = opened {
		let _ = writeln!(journal, "{}", payload.trim_end());
	}
}

struct Edit {
	path: PathBuf,
	authored: Vec<String>,
}

fn read(rules: &[Rule], payload: &Value) -> Vec<String> {
	let arguments = payload.get("tool_input").unwrap_or(&Value::Null);
	edits(arguments)
		.iter()
		.flat_map(|edit| about(rules, edit))
		.collect()
}

fn about(rules: &[Rule], edit: &Edit) -> Vec<String> {
	let Ok(source) = std::fs::read_to_string(&edit.path) else {
		return vec![];
	};
	findings(rules, &edit.path, &source, &scoped(&source, &edit.authored))
}

fn edits(arguments: &Value) -> Vec<Edit> {
	match gathered(arguments, TOUCHED).first() {
		Some(path) => vec![Edit {
			path: PathBuf::from(path),
			authored: gathered(arguments, WRITTEN),
		}],
		None => patches(arguments)
			.iter()
			.flat_map(|patch| patched(patch))
			.collect(),
	}
}

fn patches(arguments: &Value) -> Vec<String> {
	wording(arguments)
		.into_iter()
		.filter(|text| text.contains(ENVELOPE))
		.collect()
}

fn wording(arguments: &Value) -> Vec<String> {
	match arguments {
		Value::String(text) => vec![text.clone()],
		Value::Object(fields) => fields.values().flat_map(wording).collect(),
		Value::Array(listed) => listed.iter().flat_map(wording).collect(),
		_ => vec![],
	}
}

fn patched(patch: &str) -> Vec<Edit> {
	let mut edits: Vec<Edit> = vec![];
	let mut running = false;
	for line in patch.lines() {
		match opening(line) {
			Some(path) => edits.push(Edit {
				path,
				authored: vec![],
			}),
			None => running = absorbed(&mut edits, line, running),
		}
	}
	edits
}

fn opening(line: &str) -> Option<PathBuf> {
	let named = line
		.strip_prefix("*** Add File: ")
		.or_else(|| line.strip_prefix("*** Update File: "))?;
	Some(PathBuf::from(named.trim()))
}

fn absorbed(edits: &mut [Edit], line: &str, running: bool) -> bool {
	let Some(edit) = edits.last_mut() else {
		return false;
	};
	if let Some(moved) = line.strip_prefix("*** Move to: ") {
		edit.path = PathBuf::from(moved.trim());
		return false;
	}
	let Some(text) = line.strip_prefix('+') else {
		return false;
	};
	match (running, edit.authored.last_mut()) {
		(true, Some(run)) => run.push_str(&format!("\n{text}")),
		_ => edit.authored.push(text.to_string()),
	}
	true
}

fn gathered(arguments: &Value, keys: &[&str]) -> Vec<String> {
	let mut found = vec![];
	match arguments {
		Value::Object(fields) => {
			for (key, held) in fields {
				match (keys.contains(&key.as_str()), held.as_str()) {
					(true, Some(text)) => found.push(text.to_string()),
					_ => found.extend(gathered(held, keys)),
				}
			}
		}
		Value::Array(listed) => {
			for entry in listed {
				found.extend(gathered(entry, keys));
			}
		}
		_ => (),
	}
	found
}

fn scoped(source: &str, authored: &[String]) -> Scope {
	let mut lines = BTreeSet::new();
	for text in authored {
		lines.extend(spanned(source, text));
	}
	match lines.is_empty() {
		true => Scope::Whole,
		false => Scope::Lines(lines),
	}
}

fn spanned(source: &str, authored: &str) -> Vec<usize> {
	let Some(at) = source.find(authored) else {
		return vec![];
	};
	let first = source[..at].matches('\n').count() + 1;
	let last = first + authored.trim_end_matches('\n').matches('\n').count();
	(first..=last).collect()
}

fn spoken(found: &[String]) -> String {
	format!("{ADMONITION}\n\n<findings>\n{}\n</findings>", found.join("\n"))
}

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn knows_codex_by_its_turn() {
		assert!(codex(&json!({"turn_id": "t1"})));
		assert!(!codex(&json!({"hook_event_name": "PostToolUse"})));
	}

	#[test]
	fn knows_claude_by_its_event() {
		assert!(claude(&json!({"hook_event_name": "PostToolUse"})));
		assert!(!claude(&json!({"tool_name": "Edit"})));
	}

	#[test]
	fn finds_the_file_a_call_touched() {
		let arguments = json!({"file_path": "src/a.ts", "old_string": "x"});

		assert_eq!(gathered(&arguments, TOUCHED), vec!["src/a.ts".to_string()]);
	}

	#[test]
	fn finds_what_a_call_wrote_however_deeply_it_sits() {
		let arguments = json!({"edits": [{"new_string": "one"}, {"new_string": "two"}]});

		assert_eq!(
			gathered(&arguments, WRITTEN),
			vec!["one".to_string(), "two".to_string()]
		);
	}

	#[test]
	fn reads_the_files_an_apply_patch_names() {
		let patch = "*** Begin Patch\n*** Add File: a.py\n+one\n*** Update File: b.py\n@@\n-gone\n+two\n*** End Patch";

		let edits = patched(patch);

		assert_eq!(edits.len(), 2);
		assert_eq!(edits[0].path, PathBuf::from("a.py"));
		assert_eq!(edits[0].authored, vec!["one".to_string()]);
		assert_eq!(edits[1].path, PathBuf::from("b.py"));
		assert_eq!(edits[1].authored, vec!["two".to_string()]);
	}

	#[test]
	fn reads_the_patch_codex_sends_under_command() {
		let patch = "*** Begin Patch\n*** Add File: a.py\n+data = 1\n*** End Patch";
		let arguments = json!({"command": patch});

		let edits = edits(&arguments);

		assert_eq!(edits.len(), 1);
		assert_eq!(edits[0].path, PathBuf::from("a.py"));
		assert_eq!(edits[0].authored, vec!["data = 1".to_string()]);
	}

	#[test]
	fn passes_over_a_command_that_is_not_a_patch() {
		assert!(edits(&json!({"command": "ls -la"})).is_empty());
	}

	#[test]
	fn keeps_a_run_of_added_lines_together() {
		let patch = "*** Begin Patch\n*** Add File: a.py\n+one\n+two\n@@\n+four\n*** End Patch";

		let edits = patched(patch);

		assert_eq!(
			edits[0].authored,
			vec!["one\ntwo".to_string(), "four".to_string()]
		);
	}

	#[test]
	fn follows_a_file_an_apply_patch_moves() {
		let patch = "*** Begin Patch\n*** Update File: old.py\n*** Move to: new.py\n@@\n+one\n*** End Patch";

		assert_eq!(patched(patch)[0].path, PathBuf::from("new.py"));
	}

	#[test]
	fn passes_over_a_file_an_apply_patch_deletes() {
		let patch = "*** Begin Patch\n*** Delete File: gone.py\n*** End Patch";

		assert!(patched(patch).is_empty());
	}

	#[test]
	fn scopes_to_the_lines_the_written_text_occupies() {
		let source = "one\ntwo\nthree\nfour\n";

		assert_eq!(spanned(source, "three"), vec![3]);
		assert_eq!(spanned(source, "two\nthree"), vec![2, 3]);
		assert_eq!(spanned(source, "absent"), Vec::<usize>::new());
	}

	#[test]
	fn reads_a_whole_file_when_it_cannot_place_what_was_written() {
		assert!(matches!(
			scoped("one\ntwo\n", &["absent".to_string()]),
			Scope::Whole
		));
		assert!(matches!(scoped("one\ntwo\n", &[]), Scope::Whole));
		assert!(matches!(
			scoped("one\ntwo\n", &["two".to_string()]),
			Scope::Lines(_)
		));
	}
}
