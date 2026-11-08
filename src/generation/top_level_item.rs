use dprint_core::formatting::*;
use lisette_syntax::ast::{Attribute, Expression};

use crate::generation::Context;

impl<'a> Context<'a> {
	pub(crate) fn package(&mut self, top_level_items: &'a [Expression]) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		let mut wrote_any = false;

		if let Some(shebang) = self.comments.take_shebang() {
			items.push_str_runtime_width_computed(shebang);
			wrote_any = true;
		}

		if let Some(file_comments) = self.comments.take_file_comments() {
			if wrote_any {
				items.push_signal(Signal::NewLine);
				items.push_signal(Signal::NewLine);
			}
			items.extend(file_comments);
			wrote_any = true;
		}

		let mut previous_end = None;
		for item in top_level_items {
			let span = item.get_span();
			let leading_edge = leading_edge(item);
			let doc_comments = self.comments.take_doc_comments_before(leading_edge);
			let split = self
				.comments
				.take_split_after_previous(previous_end, leading_edge);

			if wrote_any {
				if let Some(trailing) = split.trailing {
					items.push_space();
					items.extend(trailing);
				}
				items.push_signal(Signal::NewLine);
				items.push_signal(Signal::NewLine);
			}

			if let Some(doc_comments) = doc_comments {
				items.extend(doc_comments);
				items.push_signal(Signal::NewLine);
			}
			if let Some(leading) = split.leading {
				items.extend(leading);
			}

			items.extend(self.attributes(item));
			if is_public(item) {
				items.push_str("pub ");
			}
			items.extend(self.verbatim(span.byte_offset, span.end()));
			self.comments.mark_handled_before(span.end());

			previous_end = Some(span.end());
			wrote_any = true;
		}

		if let Some(trailing) = self.comments.take_trailing_comments() {
			if wrote_any {
				items.push_signal(Signal::NewLine);
				items.push_signal(Signal::NewLine);
			}
			items.extend(trailing);
			wrote_any = true;
		}

		if wrote_any {
			items.push_signal(Signal::NewLine);
		}

		items
	}

	fn attributes(&mut self, item: &'a Expression) -> PrintItems<'a> {
		let mut items = PrintItems::new(self.arena);

		for attribute in item_attributes(item) {
			if let Some(leading) = self
				.comments
				.take_comments_before(attribute.span.byte_offset)
			{
				items.extend(leading);
			}
			items.extend(self.verbatim(attribute.span.byte_offset, attribute.span.end()));
			items.push_signal(Signal::NewLine);
		}

		items
	}
}

/// Attributes come before the item's span, so they are what the comments attach to.
fn leading_edge(item: &Expression) -> u32 {
	item_attributes(item).first().map_or_else(
		|| item.get_span().byte_offset,
		|attribute| attribute.span.byte_offset,
	)
}

fn item_attributes(item: &Expression) -> &[Attribute] {
	match item {
		Expression::Function { attributes, .. }
		| Expression::Struct { attributes, .. }
		| Expression::Enum { attributes, .. }
		| Expression::Interface { attributes, .. }
		| Expression::VariableDeclaration { attributes, .. }
		| Expression::TypeAlias { attributes, .. } => attributes,
		_ => &[],
	}
}

fn is_public(item: &Expression) -> bool {
	match item {
		Expression::Function { visibility, .. }
		| Expression::Struct { visibility, .. }
		| Expression::Enum { visibility, .. }
		| Expression::Interface { visibility, .. }
		| Expression::VariableDeclaration { visibility, .. }
		| Expression::TypeAlias { visibility, .. }
		| Expression::Const { visibility, .. } => visibility.is_public(),
		_ => false,
	}
}
