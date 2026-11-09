use dprint_core::formatting::*;
use lisette_syntax::ast::{Literal, Span};

use crate::generation::Context;

impl<'a> Context<'a> {
	/// The span is needed for the literals that are not generated yet, and for a string
	/// holding a newline or a tab, which the printer does not accept as one string.
	pub(crate) fn literal(&mut self, literal: &'a Literal, span: Span) -> PrintItems<'a> {
		let arena = self.arena;

		match literal {
			Literal::Integer { value, text } => match text {
				Some(text) => arena.bump().alloc_str(text).into_print_items(arena),
				None => arena
					.bump()
					.alloc_str(&value.to_string())
					.into_print_items(arena),
			},
			Literal::Float { value, text } => match text {
				Some(text) => arena.bump().alloc_str(text).into_print_items(arena),
				None => {
					let text = value.to_string();
					let text = if text.contains(['.', 'e', 'E']) {
						text
					} else {
						format!("{text}.0")
					};

					arena.bump().alloc_str(&text).into_print_items(arena)
				}
			},
			Literal::Imaginary(value) => {
				let text = if *value == value.trunc() && value.abs() < 1e15 {
					format!("{}i", *value as i64)
				} else {
					format!("{value}i")
				};

				arena.bump().alloc_str(&text).into_print_items(arena)
			}
			Literal::Boolean(value) => if *value { "true" } else { "false" }.into_print_items(arena),
			Literal::Char(value) => arena
				.bump()
				.alloc_str(&format!("'{value}'"))
				.into_print_items(arena),
			Literal::String { value, raw } => {
				let text = if *raw {
					format!("r\"{value}\"")
				} else {
					format!("\"{value}\"")
				};
				let text = arena.bump().alloc_str(&text);

				ir_helpers::gen_from_raw_string(arena, text)
			}
			Literal::Slice(_) | Literal::FormatString(_) => {
				self.comments.mark_handled_before(span.end());
				self.verbatim(span.byte_offset, span.end())
			}
		}
	}
}
