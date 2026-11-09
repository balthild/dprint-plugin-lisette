use dprint_core::formatting::*;
use lisette_syntax::ast::{
	Attribute, Binding, ConstInitializer, EnumFieldDefinition, EnumVariant, Expression, FunctionBody,
	Generic, ParentInterface, StructFieldDefinition, StructFields, VariantFields,
};

use crate::generation::{Context, ListOptions};

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
			items.extend(self.definition(item));
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

	fn definition(&mut self, item: &'a Expression) -> PrintItems<'a> {
		match item {
			Expression::Function {
				name,
				generics,
				params,
				return_annotation,
				body,
				..
			} => self.function(name, generics, params, return_annotation, body),

			Expression::Struct {
				name,
				generics,
				fields,
				span,
				..
			} => self.struct_definition(name, generics, fields, span.byte_offset),

			Expression::Enum {
				name,
				generics,
				variants,
				span,
				..
			} => self.enum_definition(name, generics, variants, span.byte_offset),

			Expression::TypeAlias {
				name,
				generics,
				annotation,
				..
			} => {
				let arena = self.arena;
				let mut items = PrintItems::new(arena);
				items.push_str("type ");
				items.push_str(name);
				items.extend(self.generics(generics));
				if !annotation.is_opaque() {
					items.push_str(" = ");
					items.extend(self.annotation(annotation));
				}
				items
			}

			Expression::Interface {
				name,
				generics,
				parents,
				method_signatures,
				span,
				..
			} => self.interface(name, generics, parents, method_signatures, span.byte_offset),

			Expression::Const {
				identifier,
				annotation,
				expression,
				..
			} => {
				let arena = self.arena;
				let mut items = PrintItems::new(arena);
				items.push_str("const ");
				items.push_str(identifier);
				if let Some(annotation) = annotation {
					items.push_str(": ");
					items.extend(self.annotation(annotation));
				}
				if let ConstInitializer::Value(value) = expression {
					items.push_str(" = ");
					items.extend(self.expression(value));
				}
				items
			}

			Expression::VariableDeclaration {
				name, annotation, ..
			} => {
				let arena = self.arena;
				let mut items = PrintItems::new(arena);
				items.push_str("var ");
				items.push_str(name);
				items.push_str(": ");
				items.extend(self.annotation(annotation));
				items
			}

			Expression::PackageImport { name, alias, .. } => {
				let arena = self.arena;
				let mut items = PrintItems::new(arena);
				items.push_str("import ");
				match alias {
					Some(lisette_syntax::ast::ImportAlias::Named(alias, _)) => {
						items.push_str(alias);
						items.push_str(" ");
					}
					Some(lisette_syntax::ast::ImportAlias::Blank(_)) => items.push_str("_ "),
					None => {}
				}
				items.push_str("\"");
				items.push_str(name);
				items.push_str("\"");
				items
			}

			other => {
				let span = other.get_span();
				self.verbatim(span.byte_offset, span.end())
			}
		}
	}

	fn function(
		&mut self,
		name: &'a str,
		generics: &'a [Generic],
		params: &'a [Binding],
		return_annotation: &'a lisette_syntax::ast::Annotation,
		body: &'a FunctionBody,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str("fn ");
		items.push_str(name);
		items.extend(self.generics(generics));
		items.extend(self.parameters(params));

		if !return_annotation.is_unknown() {
			items.push_str(" -> ");
			items.extend(self.annotation(return_annotation));
		}

		if let Some(body) = body.definition() {
			items.push_str(" ");
			items.extend(self.expression(body));
		}

		items
	}

	fn parameters(&mut self, params: &'a [Binding]) -> PrintItems<'a> {
		self.list(
			params,
			ListOptions::parens(),
			|param| param.pattern.get_span().byte_offset,
			|context, param| context.binding(param).into_rc_path(),
		)
	}

	pub(crate) fn binding(&mut self, binding: &'a Binding) -> PrintItems<'a> {
		let mut items = PrintItems::new(self.arena);

		if binding.is_mutable() {
			items.push_str("mut ");
		}
		items.extend(self.pattern(&binding.pattern));
		if let Some(annotation) = &binding.annotation {
			items.push_str(": ");
			items.extend(self.annotation(annotation));
		}

		items
	}

	fn struct_definition(
		&mut self,
		name: &'a str,
		generics: &'a [Generic],
		fields: &'a StructFields,
		start: u32,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str("struct ");
		items.push_str(name);
		items.extend(self.generics(generics));

		match fields {
			StructFields::Record(fields) => {
				if fields.is_empty() {
					items.push_str(" {}");
					return items;
				}

				let mut options = ListOptions::braces();
				options.force_use_new_lines =
					self.has_line_break_between(start, fields[0].name_span.byte_offset);
				items.push_str(" ");
				items.extend(self.list(
					fields,
					options,
					|field| field.name_span.byte_offset,
					|context, field| context.record_field(field).into_rc_path(),
				));
			}
			StructFields::Tuple(fields) => {
				items.extend(self.list(
					fields,
					ListOptions::parens(),
					|field| field.name_span.byte_offset,
					|context, field| context.annotation(&field.annotation).into_rc_path(),
				));
			}
		}

		items
	}

	fn record_field(&mut self, field: &'a StructFieldDefinition) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);

		if field.is_embedded() {
			items.push_str("embed ");
			items.extend(self.annotation(&field.annotation));
			return items;
		}

		if field.visibility.is_public() {
			items.push_str("pub ");
		}
		items.push_str(&field.name);
		items.push_str(": ");
		items.extend(self.annotation(&field.annotation));
		items
	}

	fn enum_definition(
		&mut self,
		name: &'a str,
		generics: &'a [Generic],
		variants: &'a [EnumVariant],
		start: u32,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str("enum ");
		items.push_str(name);
		items.extend(self.generics(generics));

		if variants.is_empty() {
			items.push_str(" {}");
			return items;
		}

		let mut options = ListOptions::braces();
		options.force_use_new_lines =
			self.has_line_break_between(start, variants[0].name_span.byte_offset);
		items.push_str(" ");
		items.extend(self.list(
			variants,
			options,
			|variant| variant.name_span.byte_offset,
			|context, variant| context.enum_variant(variant).into_rc_path(),
		));
		items
	}

	fn enum_variant(&mut self, variant: &'a EnumVariant) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str(&variant.name);

		match &variant.fields {
			VariantFields::Unit => {}
			VariantFields::Tuple(fields) => {
				items.extend(self.list(
					fields,
					ListOptions::parens(),
					|field| field.name_span.byte_offset,
					|context, field| context.annotation(&field.annotation).into_rc_path(),
				));
			}
			VariantFields::Struct(fields) => {
				items.push_str(" ");
				items.extend(self.list(
					fields,
					ListOptions::braces(),
					|field| field.name_span.byte_offset,
					|context, field| context.enum_field(field).into_rc_path(),
				));
			}
		}

		items
	}

	fn enum_field(&mut self, field: &'a EnumFieldDefinition) -> PrintItems<'a> {
		let mut items = PrintItems::new(self.arena);
		items.push_str(&field.name);
		items.push_str(": ");
		items.extend(self.annotation(&field.annotation));
		items
	}

	fn interface(
		&mut self,
		name: &'a str,
		generics: &'a [Generic],
		parents: &'a [ParentInterface],
		methods: &'a [Expression],
		start: u32,
	) -> PrintItems<'a> {
		let arena = self.arena;
		let mut items = PrintItems::new(arena);
		items.push_str("interface ");
		items.push_str(name);
		items.extend(self.generics(generics));

		if parents.is_empty() && methods.is_empty() {
			items.push_str(" {}");
			return items;
		}

		// Members are separated by lines rather than commas, and always one per line.
		let mut body = PrintItems::new(arena);
		let mut wrote_any = false;
		for parent in parents {
			if wrote_any {
				body.push_signal(Signal::NewLine);
			}
			if let Some(leading) = self.comments.take_comments_before(parent.span.byte_offset) {
				body.extend(leading);
				body.push_signal(Signal::NewLine);
			}
			body.push_str("embed ");
			body.extend(self.annotation(&parent.annotation));
			wrote_any = true;
		}
		for method in methods {
			if wrote_any {
				body.push_signal(Signal::NewLine);
			}
			if let Some(leading) = self
				.comments
				.take_comments_before(method.get_span().byte_offset)
			{
				body.extend(leading);
				body.push_signal(Signal::NewLine);
			}
			body.extend(self.method_signature(method));
			wrote_any = true;
		}
		let _ = start;

		items.push_str(" {");
		items.push_signal(Signal::NewLine);
		items.extend(ir_helpers::with_indent(body));
		items.push_signal(Signal::NewLine);
		items.push_str("}");
		items
	}

	fn method_signature(&mut self, method: &'a Expression) -> PrintItems<'a> {
		match method {
			Expression::Function {
				name,
				generics,
				params,
				return_annotation,
				..
			} => {
				let arena = self.arena;
				let mut items = PrintItems::new(arena);
				items.push_str("fn ");
				items.push_str(name);
				items.extend(self.generics(generics));
				items.extend(self.parameters(params));
				if !return_annotation.is_unknown() {
					items.push_str(" -> ");
					items.extend(self.annotation(return_annotation));
				}
				items
			}
			other => {
				let span = other.get_span();
				self.verbatim(span.byte_offset, span.end())
			}
		}
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
