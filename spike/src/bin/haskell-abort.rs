// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use ast_grep_config::{GlobalRules, from_yaml_string};
use ast_grep_core::Language;
use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use ast_grep_language::SupportLang;
use serde::Deserialize;
use std::str::FromStr;

#[derive(Clone, Copy)]
struct Lang(SupportLang);

impl<'de> Deserialize<'de> for Lang {
	fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
		let name = String::deserialize(d)?;
		SupportLang::from_str(&name).map(Lang).map_err(serde::de::Error::custom)
	}
}

impl Language for Lang {
	fn kind_to_id(&self, kind: &str) -> u16 {
		self.0.kind_to_id(kind)
	}
	fn field_to_id(&self, field: &str) -> Option<u16> {
		self.0.field_to_id(field)
	}
	fn build_pattern(&self, builder: &PatternBuilder) -> Result<Pattern, PatternError> {
		builder.build(|src| StrDoc::try_new(src, *self))
	}
}

impl LanguageExt for Lang {
	fn get_ts_language(&self) -> TSLanguage {
		self.0.get_ts_language()
	}
}

fn main() {
	let blocks: usize = std::env::var("PERTURB").ok().and_then(|s| s.parse().ok()).unwrap_or(20);
	let mut held: Vec<Vec<u8>> = Vec::new();
	for i in 0..blocks {
		held.push(vec![7u8; (i * 37) % 4096 + 16]);
	}
	std::mem::forget(held);

	let skip = std::env::var("SKIP").unwrap_or_default();
	for lang in SupportLang::all_langs() {
		let name = format!("{lang:?}");
		if skip.split(',').any(|s| s == name) {
			continue;
		}
		let yaml =
			format!("id: p\nlanguage: {name}\nseverity: warning\nmessage: m\nrule:\n  any: [{{kind: comment}}]\n");
		let Ok(rules) = from_yaml_string::<Lang>(&yaml, &GlobalRules::default()) else {
			continue;
		};
		let rule = &rules[0];
		eprintln!("{name}");
		for source in ["// a thing", "# a thing", "/* a thing */", "-- a thing", "{{- /* a */ -}}"] {
			let root = rule.language.ast_grep(source);
			let _ = root.root().find(&rule.matcher).is_some();
		}
	}
	eprintln!("SURVIVED");
}
