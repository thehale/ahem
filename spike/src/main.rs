// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use ast_grep_config::{GlobalRules, RuleConfig, from_yaml_string};
use ast_grep_core::Language;
use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use ast_grep_language::SupportLang;
use serde::Deserialize;
use std::path::Path;
use std::str::FromStr;
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
	fn tree_sitter_gotmpl() -> *const ();
}

const GOTMPL: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_gotmpl) };

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Lang {
	Builtin(SupportLang),
	GoTmpl,
}

impl Lang {
	fn all() -> Vec<Lang> {
		let mut langs: Vec<Lang> = SupportLang::all_langs().iter().copied().map(Lang::Builtin).collect();
		langs.push(Lang::GoTmpl);
		langs
	}

	fn name(&self) -> String {
		match self {
			Lang::Builtin(lang) => format!("{lang:?}"),
			Lang::GoTmpl => "gotmpl".to_string(),
		}
	}
}

impl<'de> Deserialize<'de> for Lang {
	fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		let name = String::deserialize(d)?;
		if name == "gotmpl" {
			return Ok(Lang::GoTmpl);
		}
		SupportLang::from_str(&name)
			.map(Lang::Builtin)
			.map_err(serde::de::Error::custom)
	}
}

impl Language for Lang {
	fn kind_to_id(&self, kind: &str) -> u16 {
		match self {
			Lang::Builtin(lang) => lang.kind_to_id(kind),
			Lang::GoTmpl => self.get_ts_language().id_for_node_kind(kind, true),
		}
	}

	fn field_to_id(&self, field: &str) -> Option<u16> {
		match self {
			Lang::Builtin(lang) => lang.field_to_id(field),
			Lang::GoTmpl => self.get_ts_language().field_id_for_name(field).map(|f| f.get()),
		}
	}

	fn expando_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.expando_char(),
			Lang::GoTmpl => '_',
		}
	}

	fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
		if path.as_ref().extension().is_some_and(|ext| ext == "gotmpl") {
			return Some(Lang::GoTmpl);
		}
		SupportLang::from_path(path).map(Lang::Builtin)
	}

	fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
		builder.build(|src| StrDoc::try_new(src, *self))
	}
}

impl LanguageExt for Lang {
	fn get_ts_language(&self) -> TSLanguage {
		match self {
			Lang::Builtin(lang) => lang.get_ts_language(),
			Lang::GoTmpl => GOTMPL.into(),
		}
	}
}

fn main() {
	matching();
	coverage();
	waiver();
}

fn matching() {
	println!("== rule semantics reused, gotmpl grammar linked in\n");
	let rules = from_yaml_string::<Lang>(COMPILED, &GlobalRules::default()).expect("rules parse");
	for rule in &rules {
		report(rule, snippets(rule.language));
	}
}

fn report(rule: &RuleConfig<Lang>, cases: &[(&str, bool)]) {
	for (source, expected) in cases {
		let root = rule.language.ast_grep(source);
		let matched = root.root().find(&rule.matcher).is_some();
		let outcome = if matched == *expected { "ok  " } else { "FAIL" };
		println!("{outcome} {:16} {matched:<5} {source:?}", rule.id);
		assert_eq!(matched, *expected, "{}", rule.id);
	}
}

fn snippets(lang: Lang) -> &'static [(&'static str, bool)] {
	match lang {
		Lang::GoTmpl => &[("{{- /* a thing */ -}}", true), ("<p>{{ .Title }}</p>", false)],
		Lang::Builtin(SupportLang::Bash) => &[
			("# a thing", true),
			("#!/usr/bin/env bash", false),
			("# SPDX-License-Identifier: MPL-2.0", false),
		],
		_ => &[("// a thing", true), ("fn main() {}", false)],
	}
}

fn coverage() {
	println!("\n== compiling `kind: comment` to every language\n");
	let langs = Lang::all();
	let mut gaps = vec![];
	for lang in &langs {
		let name = lang.name();
		let yaml =
			format!("id: probe\nlanguage: {name}\nseverity: warning\nmessage: m\nrule:\n  any: [{{kind: comment}}]\n");
		match from_yaml_string::<Lang>(&yaml, &GlobalRules::default()) {
			Ok(_) => println!("ok   {name}"),
			Err(error) => {
				println!("GAP  {name}: {error}");
				gaps.push(name);
			}
		}
	}
	println!("\n{} of {} languages need an override: {}", gaps.len(), langs.len(), gaps.join(", "));
}

fn waiver() {
	println!("\n== candidate no-op markers, compiled against Json\n");
	for (label, body) in [
		("any: []", "rule:\n  any: []\n"),
		("all: []", "rule:\n  all: []\n"),
		("not: {any: []}", "rule:\n  not: {any: []}\n"),
		("kind: comment", "rule:\n  any: [{kind: comment}]\n"),
	] {
		let yaml = format!("id: probe\nlanguage: Json\nseverity: warning\nmessage: m\n{body}");
		match from_yaml_string::<Lang>(&yaml, &GlobalRules::default()) {
			Ok(rules) => {
				let rule = &rules[0];
				let hits = ["{\"a\": 1}", "[]", "// c"]
					.iter()
					.filter(|s| rule.language.ast_grep(s).root().find(&rule.matcher).is_some())
					.count();
				println!("ok   {label:<16} compiles, matches {hits} of 3");
			}
			Err(error) => println!("GAP  {label:<16} {error}"),
		}
	}
}

const COMPILED: &str = r#"
id: comment-earns-nothing.gotmpl
language: gotmpl
severity: warning
message: Default to no comments.
rule:
  all:
    - any: [{kind: comment}]
    - not: {any: [{regex: Copyright}, {regex: SPDX-License-Identifier}]}
---
id: comment-earns-nothing.Bash
language: Bash
severity: warning
message: Default to no comments.
rule:
  all:
    - any: [{kind: comment}]
    - not: {any: [{regex: Copyright}, {regex: SPDX-License-Identifier}]}
    - not: {regex: "^#!"}
---
id: comment-earns-nothing.Rust
language: Rust
severity: warning
message: Default to no comments.
rule:
  all:
    - any: [{kind: line_comment}, {kind: block_comment}]
    - not: {any: [{regex: Copyright}, {regex: SPDX-License-Identifier}]}
"#;
