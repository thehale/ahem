// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

mod check;
mod diff;
mod hook;
mod lang;
mod rules;
mod say;
mod verify;

use lang::Lang;
use rules::Rule;
use say::say;

const COMMANDS: &[&str] = &["check", "rules", "languages", "test"];

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let (command, rest) = split(&args);
	match asked(&args) {
		true => std::process::exit(help()),
		false => std::process::exit(dispatch(command, rest)),
	}
}

fn asked(args: &[String]) -> bool {
	args.iter().any(|arg| arg == "--help" || arg == "-h")
}

fn split(args: &[String]) -> (&str, &[String]) {
	match args.split_first() {
		Some((command, rest)) if command == "check" => checking(command, rest),
		Some((command, rest)) if spoken(command) => (command.as_str(), rest),
		Some(_) => ("check", args),
		None => ("check", &[]),
	}
}

fn spoken(command: &str) -> bool {
	COMMANDS.contains(&command) || command.starts_with('-')
}

fn checking<'a>(command: &'a str, rest: &'a [String]) -> (&'a str, &'a [String]) {
	match rest.split_first() {
		Some((flag, tail)) if flag.starts_with("--") => (flag.as_str(), tail),
		_ => (command, rest),
	}
}

fn dispatch(command: &str, rest: &[String]) -> i32 {
	match command {
		"check" => run(|rules| check::check(rules, rest)),
		"--diff" => run(|rules| diff::diff(rules, rest)),
		"--hook" => run(hook::hook),
		named if named.starts_with("--diff=") => run(|rules| diff::patched(rules, named, rest)),
		"test" => run(verify::verify),
		"rules" => run(|rules| {
			listed(rules);
			0
		}),
		"languages" => {
			for lang in Lang::all() {
				say(&lang.to_string());
			}
			0
		}
		"--version" => version(),
		"--help" | "-h" | "help" => help(),
		_ => usage(),
	}
}

fn run(action: impl Fn(&[Rule]) -> usize) -> i32 {
	let rules = match rules::compile() {
		Ok(rules) => rules,
		Err(error) => {
			eprintln!("ahem: {error}");
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
		let reached = named(&rule.matchers.keys().collect::<Vec<&Lang>>());
		say(&format!("{}  {:?}", rule.id, rule.severity));
		say(&format!("  {:2} languages  {}", reached.len(), reached.join(" ")));
	}
}

fn named(langs: &[&Lang]) -> Vec<String> {
	langs.iter().map(|lang| lang.to_string()).collect()
}

fn version() -> i32 {
	say(&format!(
		"{} {}",
		env!("CARGO_PKG_NAME"),
		env!("CARGO_PKG_VERSION")
	));
	0
}

fn help() -> i32 {
	version();
	say(env!("CARGO_PKG_DESCRIPTION"));
	say("");
	say(&manual());
	0
}

fn usage() -> i32 {
	eprintln!("{}", manual());
	1
}

fn manual() -> String {
	format!(
		"usage: {} [COMMAND] [PATH...]

  check PATH...  report what the rules find under each path, or under .
  check --diff [PATH...]
                 report on what this repository has not committed, or on each
                 PATH given, narrowed to the lines the diff covers
  check --diff - read the unified diff on stdin instead
  check --diff=FILE
                 read the unified diff in FILE instead
  check --hook   answer a coding agent's tool-use payload on stdin
  rules          list the rules and the languages each one reaches
  languages      list the grammars linked into this binary
  test           run every rule against its own snippets
  --version      print the version
  --help         print this message",
		env!("CARGO_PKG_NAME")
	)
}
