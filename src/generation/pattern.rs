use dprint_core::formatting::bumpalo::collections::Vec as BumpVec;
use dprint_core::formatting::*;
use lisette_syntax::ast::{Pattern, RestPattern, StructFieldPattern};

use crate::generation::{Context, ListOptions};

/// The rest pattern is one more value in the list rather than a separate shape, and a
/// struct field carries the name it may be written without.
#[derive(Clone, Copy)]
enum Value<'a> {
	Pattern(&'a Pattern),
	Field(&'a StructFieldPattern),
	Rest,
	RestBind(&'a str),
}

impl<'a> Value<'a> {
	fn of_rest(rest: &'a RestPattern) -> Option<Self> {
		match rest {
			RestPattern::Absent => None,
			RestPattern::Discard(_) => Some(Self::Rest),
			RestPattern::Bind { name, .. } => Some(Self::RestBind(name)),
		}
	}

	fn start(&self) -> u32 {
		match self {
			Self::Pattern(pattern) => pattern.get_span().byte_offset,
			Self::Field(field) => field.value.get_span().byte_offset,
			Self::Rest | Self::RestBind(_) => u32::MAX,
		}
	}

	fn items(&self, context: &mut Context<'a>) -> PrintItems<'a> {
		match self {
			Self::Pattern(pattern) => context.pattern(pattern),
			Self::Field(field) => {
				let value = context.pattern(&field.value);
				if let Pattern::Identifier { identifier, .. } = &field.value
					&& identifier == &field.name
				{
					return value;
				}

				let mut items = PrintItems::new(context.arena);
				items.push_str(&field.name);
				items.push_str(": ");
				items.extend(value);
				items
			}
			Self::Rest => "..".into_print_items(context.arena),
			Self::RestBind(name) => {
				let mut items = PrintItems::new(context.arena);
				items.push_str("..");
				items.push_str(name);
				items
			}
		}
	}
}

impl<'a> Context<'a> {
	pub(crate) fn pattern(&mut self, pattern: &'a Pattern) -> PrintItems<'a> {
		let leading = self
			.comments
			.take_comments_before(pattern.get_span().byte_offset);
		let items = self.pattern_inner(pattern);

		match leading {
			Some(mut leading) => {
				leading.push_signal(Signal::NewLine);
				leading.extend(items);
				leading
			}
			None => items,
		}
	}

	fn pattern_inner(&mut self, pattern: &'a Pattern) -> PrintItems<'a> {
		let arena = self.arena;

		match pattern {
			Pattern::Literal { literal, span, .. } => self.literal(literal, *span),
			Pattern::Unit { .. } => "()".into_print_items(arena),
			Pattern::WildCard { .. } => "_".into_print_items(arena),
			Pattern::Identifier { identifier, .. } => {
				arena.bump().alloc_str(identifier).into_print_items(arena)
			}

			Pattern::EnumVariant {
				identifier,
				fields,
				rest,
				..
			} => {
				if fields.is_empty() && !rest {
					return arena.bump().alloc_str(identifier).into_print_items(arena);
				}

				let mut values = BumpVec::new_in(arena.bump());
				for field in fields {
					values.push(Value::Pattern(field));
				}
				if *rest {
					values.push(Value::Rest);
				}

				let mut items = PrintItems::new(arena);
				items.push_str(identifier);
				items.extend(self.value_list(values.into_bump_slice(), ListOptions::parens()));
				items
			}

			Pattern::Struct {
				identifier,
				fields,
				rest,
				..
			} => {
				if fields.is_empty() && !rest {
					let mut items = PrintItems::new(arena);
					items.push_str(identifier);
					items.push_str(" {}");
					return items;
				}

				let mut values = BumpVec::new_in(arena.bump());
				for field in fields {
					values.push(Value::Field(field));
				}
				if *rest {
					values.push(Value::Rest);
				}

				let mut items = PrintItems::new(arena);
				items.push_str(identifier);
				items.push_str(" ");
				items.extend(self.value_list(values.into_bump_slice(), ListOptions::braces()));
				items
			}

			Pattern::Tuple { elements, .. } => {
				let mut values = BumpVec::new_in(arena.bump());
				for element in elements {
					values.push(Value::Pattern(element));
				}
				self.value_list(values.into_bump_slice(), ListOptions::parens())
			}

			Pattern::Slice { prefix, rest, .. } => {
				let mut values = BumpVec::new_in(arena.bump());
				for pattern in prefix {
					values.push(Value::Pattern(pattern));
				}
				if let Some(rest) = Value::of_rest(rest) {
					values.push(rest);
				}
				self.value_list(values.into_bump_slice(), ListOptions::brackets())
			}

			Pattern::Or { patterns, .. } => {
				let mut items = PrintItems::new(arena);
				for (index, pattern) in patterns.iter().enumerate() {
					if index > 0 {
						items.push_str(" |");
						items.push_signal(Signal::SpaceOrNewLine);
					}
					items.extend(self.pattern(pattern));
				}
				items
			}

			Pattern::AsBinding { pattern, name, .. } => {
				let mut items = self.pattern(pattern);
				items.push_str(" as ");
				items.push_str(name);
				items
			}
		}
	}

	fn value_list(&mut self, values: &'a [Value<'a>], options: ListOptions) -> PrintItems<'a> {
		self.list(
			values,
			options,
			|value| value.start(),
			|context, value| value.items(context).into_rc_path(),
		)
	}
}
