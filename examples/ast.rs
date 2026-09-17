// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0
//
// Usage: cargo run --example ast -- LANGUAGE PATH

#![allow(dead_code)]

#[path = "../src/lang.rs"]
mod lang;

use ast_grep_core::Node;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSLanguage};
use lang::Lang;
use std::str::FromStr;

fn main() {
	let args: Vec<String> = std::env::args().skip(1).collect();
	let [name, path] = args.as_slice() else {
		eprintln!("usage: cargo run --example ast -- LANGUAGE PATH");
		std::process::exit(1);
	};
	let lang = Lang::from_str(name).unwrap_or_else(|error| panic!("{error}"));
	let source = std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"));
	dump(lang.ast_grep(&source).root(), 0, "", &lang.get_ts_language());
}

fn dump(node: Node<'_, StrDoc<Lang>>, depth: usize, field: &str, ts: &TSLanguage) {
	let flat: String = node
		.text()
		.chars()
		.take(48)
		.collect::<String>()
		.replace('\n', "\\n");
	let named = match node.is_named() {
		true => "",
		false => "~",
	};
	println!(
		"{:indent$}{field}{named}{} {flat:?}",
		"",
		node.kind(),
		indent = depth * 2
	);
	for child in node.children() {
		let label = fielded(&node, &child, ts);
		dump(child, depth + 1, &label, ts);
	}
}

fn fielded(parent: &Node<'_, StrDoc<Lang>>, child: &Node<'_, StrDoc<Lang>>, ts: &TSLanguage) -> String {
	let named = (1..=ts.field_count() as u16)
		.filter(|id| {
			parent
				.child_by_field_id(*id)
				.is_some_and(|found| found.node_id() == child.node_id())
		})
		.find_map(|id| ts.field_name_for_id(id));
	match named {
		Some(name) => format!("{name}: "),
		None => String::new(),
	}
}
