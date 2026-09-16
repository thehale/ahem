// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::Lang;
use ast_grep_config::{
	GlobalRules, RuleConfig, SerializableRule, SerializableRuleConfig, SerializableRuleCore, Severity,
	from_str,
};
use include_dir::{Dir, include_dir};
use serde::Deserialize;
use std::collections::BTreeMap;

static RULES: Dir = include_dir!("$CARGO_MANIFEST_DIR/rules");
static TESTS: Dir = include_dir!("$CARGO_MANIFEST_DIR/rule-tests");

#[derive(Deserialize)]
struct Authored {
	id: String,
	severity: Severity,
	message: String,
	rule: SerializableRule,
	except: Option<SerializableRule>,
	#[serde(default)]
	languages: BTreeMap<Lang, serde_yaml::Value>,
}

#[derive(Deserialize, Default)]
pub struct Snippets {
	#[serde(default)]
	pub valid: Vec<String>,
	#[serde(default)]
	pub invalid: Vec<String>,
}

pub struct Rule {
	pub id: String,
	pub severity: Severity,
	pub matchers: BTreeMap<Lang, RuleConfig<Lang>>,
	pub tests: BTreeMap<Lang, Snippets>,
}

pub fn compile() -> Result<Vec<Rule>, String> {
	let mut rules = vec![];
	for file in RULES.files() {
		let name = file.path().file_name().ok_or("rule file has no name")?;
		let text = file.contents_utf8().ok_or("rule file is not utf-8")?;
		let authored: Authored =
			from_str(text).map_err(|error| format!("{}: {error}", file.path().display()))?;
		rules.push(assembled(authored, tests(name)?)?);
	}
	rules.sort_by(|a, b| a.id.cmp(&b.id));
	Ok(rules)
}

fn tests(name: &std::ffi::OsStr) -> Result<BTreeMap<Lang, Snippets>, String> {
	let file = TESTS
		.get_file(name)
		.ok_or_else(|| format!("rule-tests/{} is missing", name.to_string_lossy()))?;
	let text = file.contents_utf8().ok_or("test file is not utf-8")?;
	from_str(text).map_err(|error| format!("{}: {error}", file.path().display()))
}

fn assembled(authored: Authored, tests: BTreeMap<Lang, Snippets>) -> Result<Rule, String> {
	let mut matchers = BTreeMap::new();
	for (lang, snippets) in &tests {
		evidenced(&authored.id, *lang, snippets)?;
		let matched =
			matcher(&authored, *lang).map_err(|error| format!("{}.{lang}: {error}", authored.id))?;
		matchers.insert(*lang, matched);
	}
	for lang in authored.languages.keys() {
		tailored(&authored.id, *lang, &tests)?;
	}
	Ok(Rule {
		id: authored.id.clone(),
		severity: authored.severity.clone(),
		matchers,
		tests,
	})
}

fn evidenced(id: &str, lang: Lang, snippets: &Snippets) -> Result<(), String> {
	match snippets.invalid.is_empty() || snippets.valid.is_empty() {
		true => Err(format!(
			"{id}.{lang}: a language needs a snippet the rule flags and one it leaves alone"
		)),
		false => Ok(()),
	}
}

fn tailored(id: &str, lang: Lang, tests: &BTreeMap<Lang, Snippets>) -> Result<(), String> {
	match tests.contains_key(&lang) {
		true => Ok(()),
		false => Err(format!("{id}: {lang} is tailored but has no snippets")),
	}
}

fn matcher(authored: &Authored, lang: Lang) -> Result<RuleConfig<Lang>, String> {
	let config = SerializableRuleConfig {
		core: SerializableRuleCore {
			rule: body(authored, lang)?,
			constraints: None,
			utils: None,
			transform: None,
		},
		fix: None,
		rewriters: None,
		id: authored.id.clone(),
		language: lang,
		message: authored.message.clone(),
		note: None,
		severity: authored.severity.clone(),
		labels: None,
		files: None,
		ignores: None,
		url: None,
		metadata: None,
	};
	RuleConfig::try_from(config, &GlobalRules::default()).map_err(|error| error.to_string())
}

fn body(authored: &Authored, lang: Lang) -> Result<SerializableRule, String> {
	let (replacement, exception) = tailoring(authored, lang)?;
	let base = replacement.unwrap_or_else(|| authored.rule.clone());
	let exceptions: Vec<SerializableRule> = [authored.except.clone(), exception]
		.into_iter()
		.flatten()
		.map(negated)
		.collect();
	if exceptions.is_empty() {
		return Ok(base);
	}
	Ok(composed(base, exceptions))
}

fn tailoring(
	authored: &Authored,
	lang: Lang,
) -> Result<(Option<SerializableRule>, Option<SerializableRule>), String> {
	let Some(tailored) = authored.languages.get(&lang) else {
		return Ok((None, None));
	};
	let serde_yaml::Value::Mapping(keyed) = tailored else {
		return Err(format!("{}: languages.{lang} is not a mapping", authored.id));
	};
	let mut mapping = keyed.clone();
	let exception = mapping
		.remove(serde_yaml::Value::from("except"))
		.map(reparsed)
		.transpose()?;
	let replacement = match mapping.is_empty() {
		true => None,
		false => Some(reparsed(serde_yaml::Value::Mapping(mapping))?),
	};
	Ok((replacement, exception))
}

fn reparsed(body: serde_yaml::Value) -> Result<SerializableRule, String> {
	let text = serde_yaml::to_string(&body).map_err(|error| error.to_string())?;
	from_str(&text).map_err(|error| error.to_string())
}

fn negated(rule: SerializableRule) -> SerializableRule {
	SerializableRule {
		not: Some(Box::new(rule)).into(),
		..Default::default()
	}
}

fn composed(base: SerializableRule, exceptions: Vec<SerializableRule>) -> SerializableRule {
	let mut all = vec![base];
	all.extend(exceptions);
	SerializableRule {
		all: Some(all).into(),
		..Default::default()
	}
}
