// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use ast_grep_core::Language;
use ast_grep_core::matcher::{Pattern, PatternBuilder, PatternError};
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use ast_grep_language::SupportLang;
use serde::Deserialize;
use std::borrow::Cow;
use std::fmt;
use std::path::Path;
use std::str::FromStr;
use tree_sitter_language::LanguageFn;

unsafe extern "C" {
	fn tree_sitter_gotmpl() -> *const ();
}

const GOTMPL: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_gotmpl) };

const UNSHIPPED: &[SupportLang] = &[SupportLang::Haskell];

const NAMED_NODE: bool = true;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
	Builtin(SupportLang),
	GoTmpl,
}

impl Lang {
	pub fn all() -> Vec<Lang> {
		let mut langs: Vec<Lang> = SupportLang::all_langs()
			.iter()
			.filter(|lang| !UNSHIPPED.contains(lang))
			.copied()
			.map(Lang::Builtin)
			.collect();
		langs.push(Lang::GoTmpl);
		langs
	}

	pub fn of(path: &Path, first_line: Option<&str>) -> Option<Lang> {
		Lang::from_path(path).or_else(|| first_line.and_then(interpreted))
	}
}

impl Ord for Lang {
	fn cmp(&self, other: &Self) -> std::cmp::Ordering {
		self.to_string().cmp(&other.to_string())
	}
}

impl PartialOrd for Lang {
	fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
		Some(self.cmp(other))
	}
}

impl fmt::Display for Lang {
	fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
		match self {
			Lang::Builtin(lang) => write!(f, "{lang:?}"),
			Lang::GoTmpl => write!(f, "gotmpl"),
		}
	}
}

impl<'de> Deserialize<'de> for Lang {
	fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
		let name = String::deserialize(deserializer)?;
		Lang::from_str(&name).map_err(serde::de::Error::custom)
	}
}

impl FromStr for Lang {
	type Err = String;

	fn from_str(name: &str) -> Result<Self, Self::Err> {
		if name == "gotmpl" {
			return Ok(Lang::GoTmpl);
		}
		let lang = SupportLang::from_str(name).map_err(|error| error.to_string())?;
		if UNSHIPPED.contains(&lang) {
			return Err(format!(
				"{name} is not shipped, see docs/single-file-executable.md"
			));
		}
		Ok(Lang::Builtin(lang))
	}
}

impl Language for Lang {
	fn kind_to_id(&self, kind: &str) -> u16 {
		match self {
			Lang::Builtin(lang) => lang.kind_to_id(kind),
			Lang::GoTmpl => self.get_ts_language().id_for_node_kind(kind, NAMED_NODE),
		}
	}

	fn field_to_id(&self, field: &str) -> Option<u16> {
		match self {
			Lang::Builtin(lang) => lang.field_to_id(field),
			Lang::GoTmpl => self.get_ts_language().field_id_for_name(field).map(|id| id.get()),
		}
	}

	fn meta_var_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.meta_var_char(),
			Lang::GoTmpl => '$',
		}
	}

	fn expando_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.expando_char(),
			Lang::GoTmpl => '_',
		}
	}

	fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
		match self {
			Lang::Builtin(lang) => lang.pre_process_pattern(query),
			Lang::GoTmpl => Cow::Borrowed(query),
		}
	}

	fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
		if path.as_ref().extension().is_some_and(|ext| ext == "gotmpl") {
			return Some(Lang::GoTmpl);
		}
		SupportLang::from_path(path)
			.filter(|lang| !UNSHIPPED.contains(lang))
			.map(Lang::Builtin)
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

fn interpreted(first_line: &str) -> Option<Lang> {
	let shebang = first_line.strip_prefix("#!")?;
	let interpreter = shebang.rsplit('/').next()?.split_whitespace().last()?;
	match interpreter {
		"bash" | "sh" | "zsh" => Some(Lang::Builtin(SupportLang::Bash)),
		"python" | "python3" => Some(Lang::Builtin(SupportLang::Python)),
		"ruby" => Some(Lang::Builtin(SupportLang::Ruby)),
		"node" => Some(Lang::Builtin(SupportLang::JavaScript)),
		_ => None,
	}
}
