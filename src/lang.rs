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

const GO_KEYWORDS: &[&str] = &[
	"end", "if", "range", "with", "define", "block", "template", "partial",
];

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Added {
	GoTmpl,
	Toml,
}

impl Added {
	const ALL: [Added; 2] = [Added::GoTmpl, Added::Toml];

	fn name(&self) -> &'static str {
		match self {
			Added::GoTmpl => "gotmpl",
			Added::Toml => "Toml",
		}
	}

	fn extension(&self) -> &'static str {
		match self {
			Added::GoTmpl => "gotmpl",
			Added::Toml => "toml",
		}
	}

	fn parser(&self) -> LanguageFn {
		match self {
			Added::GoTmpl => GOTMPL,
			Added::Toml => TOML,
		}
	}
}

#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Lang {
	Builtin(SupportLang),
	Added(Added),
}

impl Lang {
	pub fn all() -> Vec<Lang> {
		let mut langs: Vec<Lang> = SupportLang::all_langs()
			.iter()
			.filter(|lang| !UNSHIPPED.contains(lang))
			.copied()
			.map(Lang::Builtin)
			.collect();
		langs.extend(Added::ALL.iter().copied().map(Lang::Added));
		langs
	}

	pub fn of(path: &Path, source: &str) -> Option<Lang> {
		let lang = Lang::from_path(path).or_else(|| source.lines().next().and_then(interpreted));
		lang.map(|lang| lang.templated(source))
	}

	fn templated(self, source: &str) -> Lang {
		match self {
			Lang::Builtin(SupportLang::Html) if is_go_template(source) => Lang::Added(Added::GoTmpl),
			lang => lang,
		}
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
			Lang::Added(added) => write!(f, "{}", added.name()),
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
		if let Some(added) = Added::ALL.iter().find(|added| added.name() == name) {
			return Ok(Lang::Added(*added));
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
			Lang::Added(_) => self.get_ts_language().id_for_node_kind(kind, NAMED_NODE),
		}
	}

	fn field_to_id(&self, field: &str) -> Option<u16> {
		match self {
			Lang::Builtin(lang) => lang.field_to_id(field),
			Lang::Added(_) => self.get_ts_language().field_id_for_name(field).map(|id| id.get()),
		}
	}

	fn meta_var_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.meta_var_char(),
			Lang::Added(_) => '$',
		}
	}

	fn expando_char(&self) -> char {
		match self {
			Lang::Builtin(lang) => lang.expando_char(),
			Lang::Added(_) => '_',
		}
	}

	fn pre_process_pattern<'q>(&self, query: &'q str) -> Cow<'q, str> {
		match self {
			Lang::Builtin(lang) => lang.pre_process_pattern(query),
			Lang::Added(_) => Cow::Borrowed(query),
		}
	}

	fn from_path<P: AsRef<Path>>(path: P) -> Option<Self> {
		let suffix = path.as_ref().extension();
		let added = Added::ALL
			.iter()
			.find(|added| suffix.is_some_and(|ext| ext == added.extension()));
		if let Some(added) = added {
			return Some(Lang::Added(*added));
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
			Lang::Added(added) => added.parser().into(),
		}
	}
}

fn is_go_template(source: &str) -> bool {
	source.split("{{").skip(1).map(action).any(is_go_action)
}

fn action(text: &str) -> &str {
	let inside = text.split_once("}}").map_or(text, |(inside, _)| inside);
	inside.trim_start_matches('-').trim_start()
}

fn is_go_action(action: &str) -> bool {
	let mut words = action.split(|c: char| c.is_whitespace() || c == '(');
	let opens_with_keyword = words
		.clone()
		.next()
		.is_some_and(|word| GO_KEYWORDS.contains(&word));
	action.starts_with("/*") || opens_with_keyword || words.any(|word| word.starts_with('.'))
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

#[cfg(test)]
mod tests {
	use super::*;

	#[test]
	fn reads_html_with_go_actions_as_a_go_template() {
		let lang = Lang::of(
			Path::new("menu.html"),
			"{{- range .Pages }}<a>{{ .Title }}</a>{{ end -}}",
		);
		assert_eq!(lang, Some(Lang::Added(Added::GoTmpl)));
	}

	#[test]
	fn reads_html_calling_a_go_function_as_a_go_template() {
		let lang = Lang::of(
			Path::new("comment.html"),
			r#"{{ print "<!-- " (.Get 0) " -->" | safeHTML }}"#,
		);
		assert_eq!(lang, Some(Lang::Added(Added::GoTmpl)));
	}

	#[test]
	fn reads_html_with_other_mustaches_as_html() {
		let lang = Lang::of(
			Path::new("index.html"),
			"<p>{{ $t('greeting') }} {{#if user}}{{ user }}{{/if}}</p>",
		);
		assert_eq!(lang, Some(Lang::Builtin(SupportLang::Html)));
	}
}
