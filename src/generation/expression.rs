use dprint_core::formatting::bumpalo::collections::Vec as BumpVec;
use dprint_core::formatting::*;
use lisette_syntax::ast::{
	BinaryOperator, Expression, FormatStringPart, IfLetAlternative, LetMode, Literal, MatchArm,
	SelectArm, Span, StructFieldAssignment, StructSpread, UnaryOperator,
};

use crate::generation::context::closing_brace_from;
use crate::generation::{Context, ListOptions};

impl<'a> Context<'a> {
	pub(crate) fn expression(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		let span = expression.get_span();
		let leading = self.comments.take_comments_before(span.byte_offset);
		let items = self.expression_inner(expression);

		self.with_leading(leading, items)
	}

	fn expression_inner(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		let arena = self.arena;

		match expression {
			Expression::Literal { literal, span, .. } => self.literal(literal, *span),
			Expression::Identifier { value, .. } => arena.bump().alloc_str(value).into_print_items(arena),
			Expression::Unit { .. } => "()".into_print_items(arena),

			Expression::Paren { expression, .. } => surround(self.expression(expression), "(", ")"),

			Expression::Block { items, span, .. } => self.block(items, *span),

			Expression::Let {
				binding,
				value,
				mode,
				..
			} => {
				let mut items = PrintItems::new(arena);
				match mode {
					LetMode::Assert | LetMode::InvalidAssertElse { .. } => items.push_str("let assert "),
					LetMode::Plain | LetMode::Else { .. } => items.push_str("let "),
				}
				items.extend(self.binding(binding));
				items.push_str(" = ");
				items.extend(self.expression(value));
				if let Some(else_block) = mode.else_block() {
					items.push_str(" else ");
					items.extend(self.as_block(else_block));
				}
				items
			}

			Expression::Return { expression, .. } => {
				let mut items = PrintItems::new(arena);
				items.push_str("return");
				if !matches!(**expression, Expression::Unit { .. }) {
					items.push_str(" ");
					items.extend(self.expression(expression));
				}
				items
			}

			Expression::Binary { .. } => self.binary(expression),

			Expression::Unary {
				operator,
				expression,
				..
			} => match operator {
				UnaryOperator::Deref => {
					let mut items = self.expression(expression);
					items.push_str(".*");
					items
				}
				_ => {
					let symbol = match operator {
						UnaryOperator::Negative => "-",
						UnaryOperator::Not => "!",
						UnaryOperator::BitwiseNot => "^",
						UnaryOperator::Deref => unreachable!(),
					};
					let mut items = PrintItems::new(arena);
					items.push_str(symbol);
					items.extend(self.expression(expression));
					items
				}
			},

			Expression::Reference { expression, .. } => {
				let mut items = PrintItems::new(arena);
				items.push_str("&");
				items.extend(self.expression(expression));
				items
			}

			Expression::Propagate { expression, .. } => {
				let mut items = self.expression(expression);
				items.push_str("?");
				items
			}

			Expression::Call {
				expression,
				args,
				spread,
				type_arguments,
				..
			} => {
				let mut items = self.expression(expression);

				if !type_arguments.is_empty() {
					let mut types = BumpVec::new_in(arena.bump());
					for annotation in type_arguments.annotations() {
						types.push(annotation);
					}
					let types = types.into_bump_slice();

					items.extend(self.list(
						types,
						ListOptions::angle(),
						|annotation| annotation.get_span().byte_offset,
						|context, annotation| context.annotation(annotation).into_rc_path(),
					));
				}

				items.extend(self.arguments(args, spread.as_deref()));
				items
			}

			Expression::DotAccess {
				expression, member, ..
			} => {
				let mut items = self.expression(expression);
				items.push_str(".");
				items.push_str(member);
				items
			}

			Expression::IndexedAccess {
				expression, index, ..
			} => {
				let mut items = self.expression(expression);
				items.push_str("[");
				items.extend(self.expression(index));
				items.push_str("]");
				items
			}

			Expression::Tuple { elements, .. } => self.list(
				elements,
				ListOptions::parens(),
				|element| element.get_span().byte_offset,
				|context, element| context.expression(element).into_rc_path(),
			),

			Expression::Assignment {
				target,
				value,
				compound_operator,
				..
			} => {
				let mut items = self.expression(target);
				items.push_str(" ");
				items.push_str(match compound_operator {
					Some(operator) => compound_assignment(operator),
					None => "=",
				});
				items.push_str(" ");
				items.extend(self.expression(value));
				items
			}

			Expression::If {
				condition,
				consequence,
				alternative,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("if ");
				items.extend(self.expression(condition));
				items.push_str(" ");
				items.extend(self.as_block(consequence));

				if let Some(alternative) = alternative
					&& !matches!(**alternative, Expression::Unit { .. })
				{
					items.push_str(" else ");
					items.extend(self.alternative(alternative));
				}

				items
			}

			Expression::IfLet {
				pattern,
				scrutinee,
				consequence,
				alternative,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("if let ");
				items.extend(self.pattern(pattern));
				items.push_str(" = ");
				items.extend(self.expression(scrutinee));
				items.push_str(" ");
				items.extend(self.as_block(consequence));

				if let IfLetAlternative::Present { expression, .. } = alternative {
					items.push_str(" else ");
					items.extend(self.alternative(expression));
				}

				items
			}

			Expression::Match {
				subject,
				arms,
				span,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("match ");
				items.extend(self.expression(subject));
				items.push_str(" {");
				items.push_signal(Signal::NewLine);
				items.extend(ir_helpers::with_indent(self.match_arms(arms, *span)));
				items.push_signal(Signal::NewLine);
				items.push_str("}");
				items
			}

			Expression::Select { arms, span, .. } => self.select(arms, *span),

			Expression::ImplBlock {
				annotation,
				generics,
				methods,
				span,
				..
			} => self.impl_block(annotation, generics, methods, *span),

			Expression::RawGo { text } => {
				let mut items = PrintItems::new(arena);
				items.push_str("@rawgo(\"");
				items.extend(ir_helpers::gen_from_raw_string(
					arena,
					arena.bump().alloc_str(text),
				));
				items.push_str("\")");
				items
			}

			Expression::StructCall {
				name,
				field_assignments,
				spread,
				span,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str(name);

				let mut values = BumpVec::new_in(arena.bump());
				for field in field_assignments {
					values.push(Field::Assignment(field));
				}
				match spread {
					StructSpread::None => {}
					StructSpread::From(expression) => values.push(Field::Spread(expression)),
					StructSpread::Autofill { .. } => values.push(Field::Autofill),
				}

				if values.is_empty() {
					items.push_str(" {}");
					return items;
				}
				let values = values.into_bump_slice();

				let mut options = ListOptions::braces();
				options.force_use_new_lines =
					self.has_line_break_between(span.byte_offset, values[0].start());

				items.push_str(" ");
				items.extend(self.list(
					values,
					options,
					|field| field.start(),
					|context, field| field.items(context).into_rc_path(),
				));
				items
			}

			Expression::Lambda {
				params,
				return_annotation,
				body,
				..
			} => {
				let mut items = PrintItems::new(arena);
				if params.is_empty() {
					items.push_str("||");
				} else {
					items.extend(self.list(
						params,
						ListOptions::pipes(),
						|param| param.pattern.get_span().byte_offset,
						|context, param| context.binding(param).into_rc_path(),
					));
				}

				if !return_annotation.is_unknown() {
					items.push_str(" -> ");
					items.extend(self.annotation(return_annotation));
				}

				items.push_str(" ");
				items.extend(self.expression(body));
				items
			}

			Expression::Loop { body, .. } => {
				let mut items = PrintItems::new(arena);
				items.push_str("loop ");
				items.extend(self.as_block(body));
				items
			}

			Expression::While {
				condition, body, ..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("while ");
				items.extend(self.expression(condition));
				items.push_str(" ");
				items.extend(self.as_block(body));
				items
			}

			Expression::WhileLet {
				pattern,
				scrutinee,
				body,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("while let ");
				items.extend(self.pattern(pattern));
				items.push_str(" = ");
				items.extend(self.expression(scrutinee));
				items.push_str(" ");
				items.extend(self.as_block(body));
				items
			}

			Expression::For {
				binding,
				iterable,
				body,
				..
			} => {
				let mut items = PrintItems::new(arena);
				items.push_str("for ");
				items.extend(self.binding(binding));
				items.push_str(" in ");
				items.extend(self.expression(iterable));
				items.push_str(" ");
				items.extend(self.as_block(body));
				items
			}

			Expression::Break { value, .. } => {
				let mut items = PrintItems::new(arena);
				items.push_str("break");
				if let Some(value) = value {
					items.push_str(" ");
					items.extend(self.expression(value));
				}
				items
			}

			Expression::Continue { .. } => "continue".into_print_items(arena),

			Expression::Range {
				start,
				end,
				inclusive,
				..
			} => {
				let mut items = PrintItems::new(arena);
				if let Some(start) = start {
					items.extend(self.expression(start));
				}
				items.push_str(if *inclusive { "..=" } else { ".." });
				if let Some(end) = end {
					items.extend(self.expression(end));
				}
				items
			}

			Expression::Cast {
				expression,
				target_type,
				..
			} => {
				let mut items = self.expression(expression);
				items.push_str(" as ");
				items.extend(self.annotation(target_type));
				items
			}

			Expression::Task { expression, .. } => prefixed(self, "task ", expression),
			Expression::Defer { expression, .. } => prefixed(self, "defer ", expression),
			Expression::Assert { expression, .. } => prefixed(self, "assert ", expression),

			Expression::TryBlock { items, span, .. } => prefixed_block(self, "try ", items, *span),
			Expression::RecoverBlock { items, span, .. } => {
				prefixed_block(self, "recover ", items, *span)
			}

			other => {
				let span = other.get_span();
				self.comments.mark_handled_before(span.end());
				self.verbatim(span.byte_offset, span.end())
			}
		}
	}

	/// One statement per line, keeping the blank lines the author left.
	pub(crate) fn block(&mut self, items: &'a [Expression], span: Span) -> PrintItems<'a> {
		let arena = self.arena;
		let end = span.end();
		let mut body = PrintItems::new(arena);
		let mut previous_end = None;

		for item in items {
			let item_span = item.get_span();
			let doc_comments = self
				.comments
				.take_doc_comments_before(item_span.byte_offset);
			let split = self
				.comments
				.take_split_after_previous(previous_end, item_span.byte_offset);

			if let Some(end) = previous_end {
				if let Some(trailing) = split.trailing {
					body.push_space();
					body.extend(trailing);
				}
				body.push_signal(Signal::NewLine);
				if split.has_blank_before_leading
					|| (split.leading.is_none() && self.has_blank_line_between(end, item_span.byte_offset))
				{
					body.push_signal(Signal::NewLine);
				}
			}

			if let Some(doc_comments) = doc_comments {
				body.extend(doc_comments);
				body.push_signal(Signal::NewLine);
			}
			if let Some(leading) = split.leading {
				body.extend(leading);
			}

			body.extend(self.expression(item));
			previous_end = Some(item_span.end());
		}

		let split = self.comments.take_split_after_previous(previous_end, end);
		if let Some(trailing) = split.trailing {
			body.push_space();
			body.extend(trailing);
		}
		if let Some(leading) = split.leading {
			if !body.is_empty() {
				body.push_signal(Signal::NewLine);
			}
			body.extend(leading);
		}

		let mut result = PrintItems::new(arena);
		result.push_str("{");
		if body.is_empty() {
			result.push_str("}");
			return result;
		}
		result.push_signal(Signal::NewLine);
		result.extend(ir_helpers::with_indent(body));
		result.push_signal(Signal::NewLine);
		result.push_str("}");
		result
	}

	pub(crate) fn as_block(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		match expression {
			Expression::Block { items, span, .. } => {
				let leading = self.comments.take_comments_before(span.byte_offset);
				let block = self.block(items, *span);

				self.with_leading(leading, block)
			}
			_ => surround(self.expression(expression), "{ ", " }"),
		}
	}

	fn alternative(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		if expression.is_conditional() {
			self.expression(expression)
		} else {
			self.as_block(expression)
		}
	}

	fn arguments(
		&mut self,
		args: &'a [Expression],
		spread: Option<&'a Expression>,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let mut values = BumpVec::new_in(arena.bump());
		for arg in args {
			values.push(arg);
		}
		if let Some(spread) = spread {
			values.push(spread);
		}
		let values = values.into_bump_slice();

		let last = values.len().saturating_sub(1);
		self.list_with_hug(
			values,
			ListOptions::parens(),
			|argument| argument.get_span().byte_offset,
			|context, argument| context.expression(argument).into_rc_path(),
			|index, argument| index == last && is_huggable(argument),
		)
	}

	/// Breaks before every operator once the chain no longer fits, indenting the
	/// continuation one level.
	fn binary(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		let arena = self.arena;
		let indent_width = self.config.indent_width;

		let mut operands = Vec::new();
		let mut operators = Vec::new();
		collect_binary(expression, &mut operands, &mut operators);

		let last = operands.len() - 1;
		let result = ir_helpers::gen_separated_values(
			|_| {
				let mut generated = Vec::with_capacity(operands.len());

				for (index, operand) in operands.iter().enumerate() {
					let mut items = PrintItems::new(arena);
					if index > 0 {
						items.push_str(operators[index - 1]);
						items.push_str(" ");
					}
					items.extend(self.expression(operand));

					generated.push(ir_helpers::GeneratedValue {
						items,
						lines_span: None,
						allow_inline_multi_line: index == last,
						allow_inline_single_line: false,
						is_known_multi_line: false,
					});
				}

				generated
			},
			ir_helpers::GenSeparatedValuesOptions {
				prefer_hanging: false,
				force_use_new_lines: false,
				max_blank_lines: 0,
				indent_width,
				single_line_options: ir_helpers::SingleLineOptions::separated_same_line(
					Signal::SpaceOrNewLine.into_print_items(arena),
				),
				multi_line_options: ir_helpers::MultiLineOptions {
					newline_at_start: false,
					newline_at_end: false,
					with_indent: true,
					with_hanging_indent: ir_helpers::BoolOrCondition::Bool(false),
					maintain_line_breaks: false,
				},
				force_possible_newline_at_start: false,
			},
		);

		result.items
	}

	fn match_arms(&mut self, arms: &'a [MatchArm], span: Span) -> PrintItems<'a> {
		self.join_braced_arms(
			arms,
			span.end(),
			|arm| arm.pattern.get_span().byte_offset,
			|arm| arm.expression.get_span().end(),
			match_arm,
		)
	}

	fn select(&mut self, arms: &'a [SelectArm], span: Span) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str("select");

		if arms.is_empty() {
			items.push_str(" {}");
			return items;
		}

		let source = self.source;
		let end = span.end();
		let body = self.join_braced_arms(
			arms,
			end,
			select_arm_start,
			|arm| select_arm_end(arm, source, end),
			|context, arm| {
				let mut items = context.select_arm(arm, end);
				items.push_str(",");
				items
			},
		);

		items.push_str(" {");
		items.push_signal(Signal::NewLine);
		items.extend(ir_helpers::with_indent(body));
		items.push_signal(Signal::NewLine);
		items.push_str("}");
		items
	}

	fn select_arm(&mut self, arm: &'a SelectArm, end: u32) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);

		match arm {
			SelectArm::Receive {
				binding,
				receive_expression,
				body,
			} => {
				items.push_str("let ");
				items.extend(self.pattern(binding));
				items.push_str(" = ");
				items.extend(self.expression(receive_expression));
				items.push_str(" => ");
				items.extend(self.expression(body));
			}
			SelectArm::Send {
				send_expression,
				body,
			} => {
				items.extend(self.expression(send_expression));
				items.push_str(" => ");
				items.extend(self.expression(body));
			}
			SelectArm::MatchReceive {
				receive_expression,
				arms,
			} => {
				items.push_str("match ");
				items.extend(self.expression(receive_expression));

				if arms.is_empty() {
					items.push_str(" {}");
					return items;
				}

				let inner_end = arms
					.last()
					.map(|arm| closing_brace_from(self.source, arm.expression.get_span().end(), end))
					.unwrap_or(end);

				items.push_str(" {");
				items.push_signal(Signal::NewLine);
				items.extend(ir_helpers::with_indent(self.join_braced_arms(
					arms,
					inner_end,
					|arm| arm.pattern.get_span().byte_offset,
					|arm| arm.expression.get_span().end(),
					match_arm,
				)));
				items.push_signal(Signal::NewLine);
				items.push_str("}");
			}
			SelectArm::WildCard { body } => {
				items.push_str("_ => ");
				items.extend(self.expression(body));
			}
		}

		items
	}

	/// Joins the arms of a braced construct, one per line, keeping the comments and blank lines
	/// the author left between them.
	fn join_braced_arms<T>(
		&mut self,
		arms: &'a [T],
		body_end: u32,
		start: impl Fn(&T) -> u32,
		end: impl Fn(&T) -> u32,
		mut format: impl FnMut(&mut Context<'a>, &'a T) -> PrintItems<'a>,
	) -> PrintItems<'a> {
		let mut items = PrintItems::new(self.arena);
		let mut previous_end = None;

		for arm in arms {
			let start = start(arm);
			let split = self.comments.take_split_after_previous(previous_end, start);

			if let Some(previous_end) = previous_end {
				if let Some(trailing) = split.trailing {
					items.push_space();
					items.extend(trailing);
				}
				items.push_signal(Signal::NewLine);
				if split.has_blank_before_leading
					|| (split.leading.is_none() && self.has_blank_line_between(previous_end, start))
				{
					items.push_signal(Signal::NewLine);
				}
			}
			if let Some(leading) = split.leading {
				items.extend(leading);
			}

			items.extend(format(self, arm));
			previous_end = Some(end(arm));
		}

		let split = self
			.comments
			.take_split_after_previous(previous_end, body_end);
		if let Some(trailing) = split.trailing {
			items.push_space();
			items.extend(trailing);
		}
		if let Some(leading) = split.leading {
			if !items.is_empty() {
				items.push_signal(Signal::NewLine);
			}
			items.extend(leading);
		}

		items
	}
}

fn match_arm<'a>(context: &mut Context<'a>, arm: &'a MatchArm) -> PrintItems<'a> {
	let mut items = context.pattern(&arm.pattern);
	if let Some(guard) = &arm.guard {
		items.push_str(" if ");
		items.extend(context.expression(guard));
	}
	items.push_str(" => ");
	items.extend(context.expression(&arm.expression));
	items.push_str(",");
	items
}

fn select_arm_start(arm: &SelectArm) -> u32 {
	match arm {
		SelectArm::Receive { binding, .. } => binding.get_span().byte_offset,
		SelectArm::Send {
			send_expression, ..
		} => send_expression.get_span().byte_offset,
		SelectArm::MatchReceive {
			receive_expression, ..
		} => receive_expression.get_span().byte_offset,
		SelectArm::WildCard { body } => body.get_span().byte_offset,
	}
}

fn select_arm_end(arm: &SelectArm, source: &str, before: u32) -> u32 {
	match arm {
		SelectArm::Receive { body, .. }
		| SelectArm::Send { body, .. }
		| SelectArm::WildCard { body } => body.get_span().end(),
		SelectArm::MatchReceive { arms, .. } => {
			let last = arms.last().map_or(0, |arm| arm.expression.get_span().end());

			closing_brace_from(source, last, before)
		}
	}
}

impl<'a> Context<'a> {
	/// The span is needed for the kinds that are not generated yet.
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
			Literal::Slice(elements) => {
				let mut options = ListOptions::brackets();
				if let Some(first) = elements.first() {
					options.force_use_new_lines =
						self.has_line_break_between(span.byte_offset, first.get_span().byte_offset);
				}

				self.list(
					elements,
					options,
					|element| element.get_span().byte_offset,
					|context, element| context.expression(element).into_rc_path(),
				)
			}
			Literal::FormatString(parts) => {
				let mut items = PrintItems::new(arena);
				items.push_str("f\"");

				for part in parts {
					match part {
						FormatStringPart::Text(text) => {
							let text = arena.bump().alloc_str(text);
							items.extend(ir_helpers::gen_from_raw_string(arena, text));
						}
						FormatStringPart::Expression(expression) => {
							items.push_str("{");
							items.extend(self.format_string_interpolation(expression));
							items.push_str("}");
						}
					}
				}

				items.push_str("\"");
				self.comments.mark_handled_before(span.end());
				items
			}
		}
	}

	/// An interpolation is kept on one line, unless it is written over several lines, where
	/// flattening it could swallow a line break that is part of a string.
	fn format_string_interpolation(&mut self, expression: &'a Expression) -> PrintItems<'a> {
		let span = expression.get_span();
		if self.has_line_break_between(span.byte_offset, span.end()) {
			self.comments.mark_handled_before(span.end());
			return self.verbatim(span.byte_offset, span.end());
		}

		ir_helpers::with_no_new_lines(self.expression(expression))
	}
}

/// A field of a struct literal, where the spread forms are not named fields.
enum Field<'a> {
	Assignment(&'a StructFieldAssignment),
	Spread(&'a Expression),
	Autofill,
}

impl<'a> Field<'a> {
	fn start(&self) -> u32 {
		match self {
			Self::Assignment(field) => field.name_span.byte_offset,
			Self::Spread(expression) => expression.get_span().byte_offset,
			Self::Autofill => u32::MAX,
		}
	}

	fn items(&self, context: &mut Context<'a>) -> PrintItems<'a> {
		match self {
			Self::Assignment(field) => {
				let mut items = PrintItems::new(context.arena);
				items.push_str(&field.name);
				items.push_str(": ");
				items.extend(context.expression(&field.value));
				items
			}
			Self::Spread(expression) => {
				let mut items = PrintItems::new(context.arena);
				items.push_str("..");
				items.extend(context.expression(expression));
				items
			}
			Self::Autofill => "..".into_print_items(context.arena),
		}
	}
}

/// The operands and operators of a binary chain, in source order.
fn collect_binary<'a>(
	expression: &'a Expression,
	operands: &mut Vec<&'a Expression>,
	operators: &mut Vec<&'static str>,
) {
	match expression {
		Expression::Binary {
			operator,
			left,
			right,
			..
		} => {
			collect_binary(left, operands, operators);
			operators.push(binary_symbol(operator));
			collect_binary(right, operands, operators);
		}
		_ => operands.push(expression),
	}
}

/// A value whose own line breaks do not force the list around it onto several lines,
/// which keeps `call(a, |x| {` on one line.
fn is_huggable(expression: &Expression) -> bool {
	matches!(
		expression,
		Expression::Lambda { .. }
			| Expression::Block { .. }
			| Expression::Match { .. }
			| Expression::Tuple { .. }
			| Expression::StructCall { .. }
			| Expression::Literal {
				literal: Literal::Slice(_),
				..
			}
	)
}

fn prefixed<'a>(
	context: &mut Context<'a>,
	prefix: &'static str,
	expression: &'a Expression,
) -> PrintItems<'a> {
	let mut items = PrintItems::new(context.arena);
	items.push_str(prefix);
	items.extend(context.expression(expression));
	items
}

fn prefixed_block<'a>(
	context: &mut Context<'a>,
	prefix: &'static str,
	items: &'a [Expression],
	span: Span,
) -> PrintItems<'a> {
	let block = context.block(items, span);
	let mut result = PrintItems::new(context.arena);
	result.push_str(prefix);
	result.extend(block);
	result
}

fn surround<'a>(items: PrintItems<'a>, open: &'static str, close: &'static str) -> PrintItems<'a> {
	let mut result = PrintItems::new(items.arena());
	result.push_str(open);
	result.extend(items);
	result.push_str(close);
	result
}

fn binary_symbol(operator: &BinaryOperator) -> &'static str {
	use BinaryOperator::*;

	match operator {
		Addition => "+",
		Subtraction => "-",
		Multiplication => "*",
		Division => "/",
		Remainder => "%",
		BitwiseAnd => "&",
		BitwiseOr => "|",
		BitwiseXor => "^",
		BitwiseAndNot => "&^",
		ShiftLeft => "<<",
		ShiftRight => ">>",
		LessThan => "<",
		LessThanOrEqual => "<=",
		GreaterThan => ">",
		GreaterThanOrEqual => ">=",
		Equal => "==",
		NotEqual => "!=",
		And => "&&",
		Or => "||",
		Pipeline => "|>",
	}
}

fn compound_assignment(operator: &BinaryOperator) -> &'static str {
	use BinaryOperator::*;

	match operator {
		Addition => "+=",
		Subtraction => "-=",
		Multiplication => "*=",
		Division => "/=",
		Remainder => "%=",
		BitwiseAnd => "&=",
		BitwiseOr => "|=",
		BitwiseXor => "^=",
		BitwiseAndNot => "&^=",
		ShiftLeft => "<<=",
		ShiftRight => ">>=",
		_ => "=",
	}
}
