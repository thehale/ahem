// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::lang::{Added, Lang};
use ast_grep_core::Node;
use ast_grep_core::matcher::KindMatcher;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc, TSRange};
use ast_grep_language::SupportLang;
use std::iter::once;

struct DocComment {
	host: Lang,
	kind: &'static str,
	opener: &'static str,
	doc: Lang,
}

const JSDOC: Lang = Lang::Added(Added::JsDoc);
const JAVADOC: Lang = Lang::Added(Added::JavaDoc);

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
		host: Lang::Builtin(SupportLang::Java),
		kind: "block_comment",
		opener: "/**",
		doc: JAVADOC,
	},
	DocComment {
		host: Lang::Added(Added::GoTmpl),
		kind: "comment",
		opener: "/*",
		doc: JSDOC,
	},
];

pub fn regions<L: LanguageExt>(
	host: Lang,
	root: Node<StrDoc<L>>,
	markup: Option<Lang>,
) -> Vec<(String, Vec<TSRange>)> {
	let mut regions = documented(host, &root);
	regions.extend(match host {
		Lang::Builtin(SupportLang::Html) => SupportLang::Html.extract_injections(root),
		Lang::Added(Added::GoTmpl) => text(&root, markup),
		Lang::Builtin(SupportLang::Markdown) => fences(&root),
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

fn text<L: LanguageExt>(root: &Node<StrDoc<L>>, markup: Option<Lang>) -> Vec<(String, Vec<TSRange>)> {
	let spans: Vec<TSRange> = root
		.find_all(KindMatcher::new("text", root.lang().clone()))
		.map(|node| node.get_inner_node().range())
		.collect();
	markup.map(|lang| (lang.to_string(), spans)).into_iter().collect()
}

fn fences<L: LanguageExt>(root: &Node<StrDoc<L>>) -> Vec<(String, Vec<TSRange>)> {
	root.find_all(KindMatcher::new("fenced_code_block", root.lang().clone()))
		.filter_map(|fence| {
			let language = fence.find(KindMatcher::new("language", root.lang().clone()))?;
			let content = fence
				.children()
				.find(|child| child.kind() == "code_fence_content")?;
			Some((language.text().to_string(), unquoted(&content)))
		})
		.collect()
}

fn unquoted<L: LanguageExt>(content: &Node<StrDoc<L>>) -> Vec<TSRange> {
	let whole = content.get_inner_node().range();
	let quotes: Vec<TSRange> = content
		.children()
		.filter(|child| child.kind() == "block_continuation" && !child.range().is_empty())
		.map(|child| child.get_inner_node().range())
		.collect();
	let starts = once((whole.start_byte, whole.start_point))
		.chain(quotes.iter().map(|quote| (quote.end_byte, quote.end_point)));
	let ends = quotes
		.iter()
		.map(|quote| (quote.start_byte, quote.start_point))
		.chain(once((whole.end_byte, whole.end_point)));
	starts
		.zip(ends)
		.filter(|(start, end)| start.0 < end.0)
		.map(|((start_byte, start_point), (end_byte, end_point))| TSRange {
			start_byte,
			end_byte,
			start_point,
			end_point,
		})
		.collect()
}
