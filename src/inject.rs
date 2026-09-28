// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::Lang;
use ast_grep_core::Node;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSRange};
use ast_grep_language::SupportLang;

pub fn regions<L: LanguageExt>(host: Lang, root: Node<StrDoc<L>>) -> Vec<(String, Vec<TSRange>)> {
	match host {
		Lang::Builtin(SupportLang::Html) => SupportLang::Html.extract_injections(root),
		_ => vec![],
	}
}
