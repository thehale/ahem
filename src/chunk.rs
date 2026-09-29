// Copyright (c) Joseph Hale, 2026
// SPDX-License-Identifier: MPL-2.0

use crate::inject;
use crate::lang::{Lang, MARKUP};
use crate::remark::is_remark;
use crate::rules::Rule;
use ast_grep_core::tree_sitter::{LanguageExt, StrDoc};
use ast_grep_core::{AstGrep, Node, NodeMatch};
use ast_grep_language::SupportLang;
use std::ops::Range;
use tree_sitter::{Parser, Range as Span};

pub type Tree = AstGrep<StrDoc<Lang>>;

const UNFENCED: bool = false;

pub struct Chunk {
	pub tree: Tree,
	ceded: Vec<Range<usize>>,
	fenced: bool,
}

impl Chunk {
	pub fn lang(&self) -> Lang {
		*self.tree.lang()
	}

	pub fn found<'a>(&'a self, rule: &'a Rule) -> Vec<NodeMatch<'a, StrDoc<Lang>>> {
		match rule.matchers.get(&self.lang()) {
			Some(matcher) => self
				.tree
				.root()
				.find_all(&matcher.matcher)
				.filter(|node| self.owns(node))
				.filter(|node| !(self.fenced && is_remark(node)))
				.collect(),
			None => vec![],
		}
	}

	fn owns(&self, node: &Node<'_, StrDoc<Lang>>) -> bool {
		let range = node.range();
		!self
			.ceded
			.iter()
			.any(|ceded| ceded.start <= range.start && range.end <= ceded.end)
	}
}

pub fn chunks(tree: Tree, markup: Option<Lang>) -> Vec<Chunk> {
	nested(tree, markup, UNFENCED)
}

fn nested(tree: Tree, markup: Option<Lang>, fenced: bool) -> Vec<Chunk> {
	let inner = injected(&tree, markup);
	let ceded = inner
		.iter()
		.flat_map(seen)
		.map(|span| span.start_byte..span.end_byte)
		.collect();
	let fenced_below = fenced || *tree.lang() == Lang::Builtin(SupportLang::Markdown);
	let below = inner
		.into_iter()
		.flat_map(|tree| nested(tree, Some(MARKUP), fenced_below));
	let mut all = vec![Chunk { tree, ceded, fenced }];
	all.extend(below);
	all
}

fn injected(tree: &Tree, markup: Option<Lang>) -> Vec<Tree> {
	let visible = seen(tree);
	inject::regions(*tree.lang(), tree.root(), markup)
		.into_iter()
		.filter_map(|(name, spans)| parsed(tree, Lang::named(&name)?, &clipped(&spans, &visible)))
		.collect()
}

fn seen(tree: &Tree) -> Vec<Span> {
	tree.root().get_doc().tree.included_ranges()
}

fn clipped(spans: &[Span], visible: &[Span]) -> Vec<Span> {
	spans
		.iter()
		.flat_map(|span| visible.iter().filter_map(|edge| overlap(span, edge)))
		.collect()
}

fn overlap(a: &Span, b: &Span) -> Option<Span> {
	let start = if a.start_byte >= b.start_byte { a } else { b };
	let end = if a.end_byte <= b.end_byte { a } else { b };
	(start.start_byte < end.end_byte).then_some(Span {
		start_byte: start.start_byte,
		start_point: start.start_point,
		end_byte: end.end_byte,
		end_point: end.end_point,
	})
}

fn parsed(tree: &Tree, lang: Lang, spans: &[Span]) -> Option<Tree> {
	match spans.is_empty() {
		true => None,
		false => {
			let mut parser = Parser::new();
			parser.set_included_ranges(spans).ok()?;
			parser.set_language(&lang.get_ts_language()).ok()?;
			let src = tree.get_text().to_string();
			let parsed = parser.parse(&src, None)?;
			Some(AstGrep::doc(StrDoc {
				src,
				lang,
				tree: parsed,
			}))
		}
	}
}
