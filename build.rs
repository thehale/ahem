// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

fn main() {
	compile("gotmpl", &["parser.c"]);
	compile("toml", &["parser.c", "scanner.c"]);
}

fn compile(grammar: &str, sources: &[&str]) {
	let src = format!("vendor/{grammar}/src");
	let mut build = cc::Build::new();
	build.include(&src).warnings(false);
	for source in sources {
		println!("cargo:rerun-if-changed={src}/{source}");
		build.file(format!("{src}/{source}"));
	}
	build.compile(&format!("tree_sitter_{grammar}"));
}
