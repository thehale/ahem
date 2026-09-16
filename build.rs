// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

fn main() {
	println!("cargo:rerun-if-changed=vendor/gotmpl/src/parser.c");
	cc::Build::new()
		.include("vendor/gotmpl/src")
		.file("vendor/gotmpl/src/parser.c")
		.warnings(false)
		.compile("tree_sitter_gotmpl");
}
