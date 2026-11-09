use dprint_core::formatting::bumpalo::collections::Vec as BumpVec;
use dprint_core::formatting::{conditions, ir_helpers, *};

use crate::generation::Context;

pub(crate) struct ListOptions {
	pub open: &'static str,
	pub close: &'static str,
	pub spaces_inside: bool,
	pub trailing_comma: bool,
	pub force_use_new_lines: bool,
}

impl ListOptions {
	pub(crate) fn parens() -> Self {
		Self {
			open: "(",
			close: ")",
			spaces_inside: false,
			trailing_comma: true,
			force_use_new_lines: false,
		}
	}

	pub(crate) fn brackets() -> Self {
		Self {
			open: "[",
			close: "]",
			spaces_inside: false,
			trailing_comma: true,
			force_use_new_lines: false,
		}
	}

	/// Braces put a space inside when the list is on one line: `{ a, b }`.
	pub(crate) fn braces() -> Self {
		Self {
			open: "{",
			close: "}",
			spaces_inside: true,
			trailing_comma: true,
			force_use_new_lines: false,
		}
	}

	/// Angle brackets never take a trailing comma.
	pub(crate) fn angle() -> Self {
		Self {
			open: "<",
			close: ">",
			spaces_inside: false,
			trailing_comma: false,
			force_use_new_lines: false,
		}
	}
}

impl<'a> Context<'a> {
	/// Stays on one line until it does not fit, then one value per line with a trailing
	/// comma. The values come out in the order they are given.
	///
	/// `start_of` is where each value begins in the source, so the comments before it are
	/// taken and put before it rather than being dropped.
	pub(crate) fn list<T>(
		&mut self,
		values: &'a [T],
		options: ListOptions,
		start_of: impl Fn(&T) -> u32,
		mut format: impl FnMut(&mut Self, &'a T) -> Option<PrintItemPath<'a>>,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let indent_width = self.config.indent_width;
		let mut items = PrintItems::new(arena);
		items.push_str(options.open);

		if values.is_empty() {
			items.push_str(options.close);
			return items;
		}

		let last = values.len() - 1;
		let result = ir_helpers::gen_separated_values(
			|is_multi_line_ref| {
				let is_multi_line = is_multi_line_ref.create_resolver(arena);
				let mut generated = BumpVec::new_in(arena.bump());

				for (index, value) in values.iter().enumerate() {
					let mut items = match self.comments.take_comments_before(start_of(value)) {
						Some(mut leading) => {
							leading.push_signal(Signal::NewLine);
							leading
						}
						None => PrintItems::new(arena),
					};

					if let Some(path) = format(self, value) {
						items.extend(path.into_print_items(arena));
					}

					if index == last {
						if options.trailing_comma {
							items.push_condition(conditions::if_true(
								"trailingCommaIfMultiLine",
								is_multi_line,
								",".into_print_items(arena),
							));
						}
					} else {
						items.push_str(",");
					}

					generated.push(ir_helpers::GeneratedValue::from_items(items));
				}

				generated
			},
			ir_helpers::GenSeparatedValuesOptions {
				prefer_hanging: false,
				force_use_new_lines: options.force_use_new_lines,
				max_blank_lines: 0,
				indent_width,
				single_line_options: if options.spaces_inside {
					ir_helpers::SingleLineOptions::surrounded_line(arena)
				} else {
					ir_helpers::SingleLineOptions::separated_same_line(
						Signal::SpaceOrNewLine.into_print_items(arena),
					)
				},
				multi_line_options: ir_helpers::MultiLineOptions::surround_newlines_indented(),
				force_possible_newline_at_start: false,
			},
		);

		items.extend(result.items);
		items.push_str(options.close);
		items
	}
}
