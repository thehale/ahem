// Copyright (c) 2026 Joseph Hale
// SPDX-License-Identifier: MPL-2.0

use crate::inject;
use crate::lang::Lang;
use ast_grep_core::Node;
use ast_grep_core::tree_sitter::StrDoc;

pub fn is_remark(node: &Node<'_, StrDoc<Lang>>) -> bool {
	let doc = !inject::hosts(*node.lang()).is_empty();
	doc || node.kind().contains("comment")
}
