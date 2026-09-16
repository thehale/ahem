// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

mod check;
mod lang;
mod rules;
mod verify;

use lang::Lang;
use rules::Rule;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let (command, rest) = split(&args);
	std::process::exit(dispatch(command, rest));
}

fn split(args: &[String]) -> (&str, &[String]) {
	match args.split_first() {
		Some((command, rest)) => (command.as_str(), rest),
		None => ("check", &[]),
	}
}

fn dispatch(command: &str, rest: &[String]) -> i32 {
	match command {
		"check" => run(|rules| check::check(rules, rest)),
		"test" => run(verify::verify),
		"rules" => run(|rules| {
			listed(rules);
			0
		}),
		_ => usage(),
	}
}

fn run(action: impl Fn(&[Rule]) -> usize) -> i32 {
	let rules = match rules::compile() {
		Ok(rules) => rules,
		Err(error) => {
			eprintln!("deconfuse: {error}");
			return 2;
		}
	};
	match action(&rules) {
		0 => 0,
		_ => 1,
	}
}

fn listed(rules: &[Rule]) {
	for rule in rules {
		let (proven, unproven) = split_by_evidence(rule);
		println!("{}  {:?}", rule.id, rule.severity);
		println!("  proven   {:2}  {}", proven.len(), proven.join(" "));
		println!("  unproven {:2}  {}", unproven.len(), unproven.join(" "));
	}
}

fn split_by_evidence(rule: &Rule) -> (Vec<String>, Vec<String>) {
	let (proven, unproven): (Vec<&Lang>, Vec<&Lang>) = rule
		.matchers
		.keys()
		.partition(|lang| rule.tests.contains_key(lang));
	(named(proven), named(unproven))
}

fn named(langs: Vec<&Lang>) -> Vec<String> {
	langs.iter().map(|lang| lang.to_string()).collect()
}

fn usage() -> i32 {
	eprintln!("usage: deconfuse [check PATH... | rules | test]");
	1
}
