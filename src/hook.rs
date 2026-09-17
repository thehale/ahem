// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::check::{Scope, findings};
use crate::rules::Rule;
use crate::say::say;
use serde_json::{Value, json};
use std::collections::BTreeSet;
use std::io::Read;
use std::path::PathBuf;

const ADMONITION: &str = "\
ahem read the file the tool just touched. Each line below names a principle the
code breaks. Hold to every one of them. The bar for rejecting one is
stratospheric: a direct operator instruction to the contrary, nearly universal
precedent throughout the codebase, and nothing less. Rejecting one means saying
out loud what makes this code the exception.";

const WRITTEN: &[&str] = &["content", "new_string", "new_source", "text"];
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
	serde_json::from_str(&payload).ok()
}

fn read(rules: &[Rule], payload: &Value) -> Vec<String> {
	let arguments = payload.get("tool_input").unwrap_or(&Value::Null);
	let Some(path) = gathered(arguments, TOUCHED).first().map(PathBuf::from) else {
		return vec![];
	};
	let Ok(source) = std::fs::read_to_string(&path) else {
		return vec![];
	};
	let scope = scoped(&source, &gathered(arguments, WRITTEN));
	findings(rules, &path, &source, &scope)
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
