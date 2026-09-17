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
	fn tree_sitter_toml() -> *const ();
}

const GOTMPL: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_gotmpl) };
const TOML: LanguageFn = unsafe { LanguageFn::from_raw(tree_sitter_toml) };

const UNSHIPPED: &[SupportLang] = &[SupportLang::Haskell];

const NAMED_NODE: bool = true;

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Vendored {
	GoTmpl,
	Toml,
}

impl Vendored {
	const ALL: [Vendored; 2] = [Vendored::GoTmpl, Vendored::Toml];

	fn name(&self) -> &'static str {
		match self {
			Vendored::GoTmpl => "gotmpl",
			Vendored::Toml => "Toml",
		}
	}

	fn extension(&self) -> &'static str {
		match self {
			Vendored::GoTmpl => "gotmpl",
			Vendored::Toml => "toml",
		}
	}

	fn parser(&self) -> LanguageFn {
		match self {
			Vendored::GoTmpl => GOTMPL,
			Vendored::Toml => TOML,
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
	Builtin(SupportLang),
	Vendored(Vendored),
}

impl Lang {
	pub fn all() -> Vec<Lang> {
		let mut langs: Vec<Lang> = SupportLang::all_langs()
			.iter()
			.filter(|lang| !UNSHIPPED.contains(lang))
			.copied()
			.map(Lang::Builtin)
			.collect();
		langs.extend(Vendored::ALL.iter().copied().map(Lang::Vendored));
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
			Lang::Vendored(vendored) => write!(f, "{}", vendored.name()),
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
		if let Some(vendored) = Vendored::ALL.iter().find(|vendored| vendored.name() == name) {
			return Ok(Lang::Vendored(*vendored));
		}
		let lang = SupportLang::from_str(name).map_err(|error| error.to_string())?;
		if UNSHIPPED.contains(&lang) {
			return Err(format!("{name} is not shipped, see UNSHIPPED in src/lang.rs"));
		}
		let canonical = Lang::Builtin(lang);
		if canonical.to_string() != name {
			return Err(format!("{name} is spelled {canonical} everywhere else"));
		}
		Ok(canonical)
	}
}

impl Language for Lang {
	fn kind_to_id(&self, kind: &str) -> u16 {
		match self {
			Lang::Builtin(lang) => lang.kind_to_id(kind),
			Lang::Vendored(_) => self.get_ts_language().id_for_node_kind(kind, NAMED_NODE),
		}
	}

	fn field_to_id(&self, field: &str) -> Option<u16> {
		match self {
			Lang::Builtin(lang) => lang.field_to_id(field),
			Lang::Vendored(_) => self.get_ts_language().field_id_for_name(field).map(|id| id.get()),
		}
	}

	fn meta_var_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.meta_var_char(),
			Lang::Vendored(_) => '$',
		}
	}

	fn expando_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.expando_char(),
			Lang::Vendored(_) => '_',
		}
	}

	fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
		match self {
			Lang::Builtin(lang) => lang.pre_process_pattern(query),
			Lang::Vendored(_) => Cow::Borrowed(query),
		}
	}

	fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
		let suffix = path.as_ref().extension();
		let vendored = Vendored::ALL
			.iter()
			.find(|vendored| suffix.is_some_and(|ext| ext == vendored.extension()));
		if let Some(vendored) = vendored {
			return Some(Lang::Vendored(*vendored));
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
			Lang::Vendored(vendored) => vendored.parser().into(),
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
