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
		"languages" => {
			for lang in Lang::all() {
				println!("{lang}");
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
		println!("{}  {:?}", rule.id, rule.severity);
		println!("  {:2} languages  {}", reached.len(), reached.join(" "));
	}
}

fn named(langs: &[&Lang]) -> Vec<String> {
	langs.iter().map(|lang| lang.to_string()).collect()
}

fn version() -> i32 {
	println!("{} {}", env!("CARGO_PKG_NAME"), env!("CARGO_PKG_VERSION"));
	0
}

fn help() -> i32 {
	version();
	println!("{}", env!("CARGO_PKG_DESCRIPTION"));
	println!();
	println!("{}", manual());
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
  rules          list the rules and the languages each one reaches
  languages      list the grammars linked into this binary
  test           run every rule against its own snippets
  --version      print the version
  --help         print this message",
		env!("CARGO_PKG_NAME")
	)
}
