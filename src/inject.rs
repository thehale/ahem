// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::{Added, Lang};
use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSRange};
use ast_grep_language::SupportLang;

struct DocComment {
	host: Lang,
	kind: &'static str,
	opener: &'static str,
	doc: Lang,
}

const JSDOC: Lang = Lang::Added(Added::JsDoc);

const DOC_COMMENTS: &[DocComment] = &[
	DocComment {
		host: Lang::Builtin(SupportLang::JavaScript),
		kind: "comment",
		opener: "/**",
		doc: JSDOC,
	},
	DocComment {
		host: Lang::Builtin(SupportLang::TypeScript),
		kind: "comment",
		opener: "/**",
		doc: JSDOC,
	},
	DocComment {
		host: Lang::Builtin(SupportLang::Tsx),
		kind: "comment",
		opener: "/**",
		doc: JSDOC,
	},
	DocComment {
		host: Lang::Added(Added::GoTmpl),
		kind: "comment",
		opener: "/*",
		doc: JSDOC,
	},
];

pub fn regions<L: LanguageExt>(host: Lang, root: Node<StrDoc<L>>) -> Vec<(String, Vec<TSRange>)> {
	let mut regions = documented(host, &root);
	regions.extend(match host {
		Lang::Builtin(SupportLang::Html) => SupportLang::Html.extract_injections(root),
		_ => vec![],
	});
	regions
}

pub fn hosts(doc: Lang) -> Vec<Lang> {
	DOC_COMMENTS
		.iter()
		.filter(|comment| comment.doc == doc)
		.map(|comment| comment.host)
		.collect()
}

fn documented<L: LanguageExt>(host: Lang, root: &Node<StrDoc<L>>) -> Vec<(String, Vec<TSRange>)> {
	DOC_COMMENTS
		.iter()
		.filter(|comment| comment.host == host)
		.flat_map(|comment| {
			root.find_all(KindMatcher::new(comment.kind, root.lang().clone()))
				.filter(|node| node.text().starts_with(comment.opener))
				.map(|node| (comment.doc.to_string(), vec![node.get_inner_node().range()]))
		})
		.collect()
}
