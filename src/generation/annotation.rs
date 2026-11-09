use dprint_core::formatting::*;
use lisette_syntax::ast::{Annotation, Generic};

use crate::generation::{Context, ListOptions};

impl<'a> Context<'a> {
	pub(crate) fn annotation(&mut self, annotation: &'a Annotation) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);

		match annotation {
			Annotation::Constructor {
				name,
				params,
				writable,
				..
			} => {
				if *writable {
					items.push_str("mut ");
				}

				if params.is_empty() {
					if name == "Unit" {
						items.push_str("()");
					} else {
						items.push_str(name);
					}
				} else {
					items.push_str(name);
					items.extend(self.list(
						params,
						ListOptions::angle(),
						annotation_start,
						|context, param| context.annotation(param).into_rc_path(),
					));
				}
			}
			Annotation::Function {
				params,
				return_type,
				..
			} => {
				items.push_str("fn");
				items.extend(self.list(
					params,
					ListOptions::parens(),
					annotation_start,
					|context, param| context.annotation(param).into_rc_path(),
				));

				if !return_type.is_unknown() {
					items.push_str(" -> ");
					items.extend(self.annotation(return_type));
				}
			}
			Annotation::Tuple { elements, .. } => {
				items.extend(self.list(
					elements,
					ListOptions::parens(),
					annotation_start,
					|context, element| context.annotation(element).into_rc_path(),
				));
			}
			Annotation::Unknown => items.push_str("_"),
			Annotation::Opaque { .. } => {}
			Annotation::Constant { value, text, .. } => match text {
				Some(text) => items.push_str(text),
				None => items.push_string(value.to_string()),
			},
		}

		items
	}

	pub(crate) fn generics(&mut self, generics: &'a [Generic]) -> PrintItems<'a> {
		if generics.is_empty() {
			return PrintItems::new(self.arena);
		}

		self.list(
			generics,
			ListOptions::angle(),
			|generic| generic.span.byte_offset,
			|context, generic| context.generic(generic).into_rc_path(),
		)
	}

	fn generic(&mut self, generic: &'a Generic) -> PrintItems<'a> {
		let mut items = PrintItems::new(self.arena);
		items.push_str(&generic.name);

		let bounds = generic.bounds().collect::<Vec<_>>();
		for (index, bound) in bounds.iter().enumerate() {
			if index == 0 {
				items.push_str(": ");
			} else {
				items.push_str(" + ");
			}
			items.extend(self.annotation(bound));
		}

		items
	}
}

fn annotation_start(annotation: &Annotation) -> u32 {
	annotation.get_span().byte_offset
}
