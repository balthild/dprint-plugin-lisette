use dprint_core::formatting::{Arena, PrintItems, Signal, ir_helpers};
use lisette_syntax::lex::{Token, TokenKind};

#[derive(Clone, Copy)]
struct Comment<'a> {
	start: u32,
	text: &'a str,
}

#[derive(Clone, Copy)]
enum Event<'a> {
	Comment(Comment<'a>),
	BlankLine(u32),
}

impl Event<'_> {
	fn start(&self) -> u32 {
		match self {
			Self::Comment(comment) => comment.start,
			Self::BlankLine(start) => *start,
		}
	}
}

/// The comments before a position, split at the point where the previous item ends:
/// comments on the previous item's line trail it, the rest lead the next item.
pub(crate) struct Split<'a> {
	pub(crate) header: Option<PrintItems<'a>>,
	pub(crate) trailing: Option<PrintItems<'a>>,
	pub(crate) leading: Option<PrintItems<'a>>,
	pub(crate) has_blank_before_leading: bool,
}

pub(crate) struct Comments<'a> {
	arena: &'a Arena,
	source: &'a str,
	events: Vec<Event<'a>>,
	cursor: usize,
	doc_comments: Vec<Comment<'a>>,
	doc_cursor: usize,
	file_comments: Vec<Comment<'a>>,
	shebang: Option<&'a str>,
}

impl<'a> Comments<'a> {
	pub(crate) fn from_lexed(
		arena: &'a Arena,
		tokens: &[Token<'a>],
		blank_lines: Vec<u32>,
		source: &'a str,
	) -> Self {
		let mut events = blank_lines
			.into_iter()
			.map(Event::BlankLine)
			.collect::<Vec<_>>();
		let mut doc_comments = Vec::new();
		let mut file_comments = Vec::new();
		let mut shebang = None;

		for token in tokens {
			let kind = token.kind;
			if kind == TokenKind::Shebang {
				shebang = source.get(token.byte_offset as usize..token.end_offset() as usize);
				continue;
			}
			if !matches!(
				kind,
				TokenKind::Comment | TokenKind::DocComment | TokenKind::FileComment
			) {
				continue;
			}

			// The token's text is the full comment, including its slashes.
			let text = source
				.get(token.byte_offset as usize..token.end_offset() as usize)
				.expect("a comment token must be a valid source range");
			let comment = Comment {
				start: token.byte_offset,
				text,
			};

			match kind {
				TokenKind::Comment => events.push(Event::Comment(comment)),
				TokenKind::DocComment => doc_comments.push(comment),
				TokenKind::FileComment => file_comments.push(comment),
				_ => unreachable!(),
			}
		}

		events.sort_by_key(Event::start);

		Self {
			arena,
			source,
			events,
			cursor: 0,
			doc_comments,
			doc_cursor: 0,
			file_comments,
			shebang,
		}
	}

	fn newline_between(&self, start: u32, end: u32) -> bool {
		if start >= end {
			return false;
		}

		let start = (start as usize).min(self.source.len());
		let end = (end as usize).min(self.source.len()).max(start);

		self.source.as_bytes()[start..end].contains(&b'\n')
	}

	/// The events before `position`, advancing the cursor past them.
	fn take_events_before(&mut self, position: u32) -> Vec<Event<'a>> {
		let start = self.cursor;
		let count = self.events[start..].partition_point(|event| event.start() < position);
		self.cursor += count;

		self.events[start..self.cursor].to_vec()
	}

	/// Splits the comments before `before` into those trailing `previous_end` and those
	/// leading the next item. `None` means there is no previous item, so all of them lead.
	pub(crate) fn take_split_after_previous(
		&mut self,
		previous_end: Option<u32>,
		before: u32,
	) -> Split<'a> {
		let events = self.take_events_before(before);
		let split_at = match previous_end {
			None => 0,
			Some(previous_end) => events
				.iter()
				.position(|event| match event {
					Event::Comment(comment) => self.newline_between(previous_end, comment.start),
					Event::BlankLine(_) => true,
				})
				.unwrap_or(events.len()),
		};
		let (trailing, leading) = events.split_at(split_at);

		// With no previous item, comments detached from the first one by a blank line are a
		// file header rather than something that travels with it.
		let (header, leading) = if previous_end.is_none() {
			let at = leading
				.iter()
				.rposition(|event| matches!(event, Event::BlankLine(_)))
				.map_or(0, |index| index + 1);

			leading.split_at(at)
		} else {
			(&[][..], leading)
		};

		Split {
			header: self.to_items(header),
			trailing: self.to_items(trailing),
			leading: self.to_items(leading),
			has_blank_before_leading: matches!(leading.first(), Some(Event::BlankLine(_))),
		}
	}

	pub(crate) fn take_comments_before(&mut self, position: u32) -> Option<PrintItems<'a>> {
		let events = self.take_events_before(position);

		self.to_items(&events)
	}

	pub(crate) fn take_doc_comments_before(&mut self, position: u32) -> Option<PrintItems<'a>> {
		let end = self.doc_comments[self.doc_cursor..]
			.iter()
			.position(|comment| comment.start >= position)
			.map(|index| self.doc_cursor + index)
			.unwrap_or(self.doc_comments.len());
		let taken = self.doc_comments[self.doc_cursor..end].to_vec();
		self.doc_cursor = end;

		self.to_items_from_comments(&taken)
	}

	pub(crate) fn take_shebang(&mut self) -> Option<&'a str> {
		self.shebang.take()
	}

	pub(crate) fn take_file_comments(&mut self) -> Option<PrintItems<'a>> {
		if self.file_comments.is_empty() {
			return None;
		}

		let taken = std::mem::take(&mut self.file_comments);

		self.to_items_from_comments(&taken)
	}

	pub(crate) fn take_trailing_comments(&mut self) -> Option<PrintItems<'a>> {
		self.take_comments_before(u32::MAX)
	}

	/// Comments in text that was emitted as is must not be emitted a second time.
	pub(crate) fn mark_handled_before(&mut self, end: u32) {
		let count = self.events[self.cursor..].partition_point(|event| event.start() < end);
		self.cursor += count;
		let count = self.doc_comments[self.doc_cursor..].partition_point(|comment| comment.start < end);
		self.doc_cursor += count;
	}

	/// A blank line between comments is kept as one blank line.
	fn to_items(&self, events: &[Event<'a>]) -> Option<PrintItems<'a>> {
		let mut items = PrintItems::new(self.arena);
		let mut events = events
			.iter()
			.skip_while(|event| matches!(event, Event::BlankLine(_)))
			.peekable();
		let mut wrote_any = false;

		while let Some(event) = events.next() {
			let Event::Comment(comment) = event else {
				continue;
			};

			if wrote_any {
				items.push_signal(Signal::NewLine);
			}
			self.push_comment(&mut items, comment);

			let mut has_blank_line = false;
			while matches!(events.peek(), Some(Event::BlankLine(_))) {
				has_blank_line = true;
				events.next();
			}
			if has_blank_line && events.peek().is_some() {
				items.push_signal(Signal::NewLine);
			}
			wrote_any = true;
		}

		if wrote_any { Some(items) } else { None }
	}

	fn to_items_from_comments(&self, comments: &[Comment<'a>]) -> Option<PrintItems<'a>> {
		let mut items = PrintItems::new(self.arena);

		for (index, comment) in comments.iter().enumerate() {
			if index > 0 {
				items.push_signal(Signal::NewLine);
			}
			self.push_comment(&mut items, comment);
		}

		if comments.is_empty() {
			None
		} else {
			Some(items)
		}
	}

	fn push_comment(&self, items: &mut PrintItems<'a>, comment: &Comment<'a>) {
		if comment.text.contains('\t') {
			items.extend(ir_helpers::gen_from_string(self.arena, comment.text));
		} else {
			items.push_str_runtime_width_computed(comment.text);
		}
		items.push_signal(Signal::ExpectNewLine);
	}
}

#[cfg(test)]
mod tests {
	use dprint_core::formatting::{PrintOptions, print};
	use lisette_syntax::lex::Lexer;

	use super::*;

	const OPTIONS: PrintOptions = PrintOptions {
		max_width: 80,
		indent_width: 2,
		use_tabs: false,
		new_line_text: "\n",
	};

	fn new_comments<'a>(arena: &'a Arena, source: &'a str) -> Comments<'a> {
		let lexed = Lexer::new(source, 0).lex();
		Comments::from_lexed(arena, &lexed.tokens, lexed.blank_lines, source)
	}

	fn render(items: Option<PrintItems<'_>>) -> String {
		let Some(mut items) = items else {
			return String::new();
		};
		items.push_signal(Signal::NewLine);

		print(items, OPTIONS)
	}

	#[test]
	fn keeps_comments_in_source_order() {
		let source = "// one\n// two\nlet x = 1\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);

		assert_eq!(
			render(comments.take_comments_before(u32::MAX)),
			"// one\n// two\n"
		);
	}

	#[test]
	fn keeps_one_blank_line_between_comments() {
		let source = "// one\n\n// two\nlet x = 1\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);

		assert_eq!(
			render(comments.take_comments_before(u32::MAX)),
			"// one\n\n// two\n"
		);
	}

	#[test]
	fn ignores_blank_lines_that_do_not_separate_comments() {
		let source = "let a = 1\n\n\nlet b = 2\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);

		assert_eq!(render(comments.take_comments_before(u32::MAX)), "");
	}

	#[test]
	fn splits_a_trailing_comment_from_the_next_leading_one() {
		let source = "let x = 1 // trailing\n// leading\nlet y = 2\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);
		let split = comments.take_split_after_previous(Some(9), 33);

		assert_eq!(render(split.trailing), "// trailing\n");
		assert_eq!(render(split.leading), "// leading\n");
	}

	#[test]
	fn has_no_trailing_comments_before_the_first_item() {
		let source = "// leading\nlet y = 2\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);
		let split = comments.take_split_after_previous(None, 11);

		assert_eq!(render(split.trailing), "");
		assert_eq!(render(split.leading), "// leading\n");
	}

	#[test]
	fn takes_doc_comments_before_a_position() {
		let source = "/// one\n/// two\nfn f() {}\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);
		let position = source.find("fn").unwrap() as u32;

		assert_eq!(
			render(comments.take_doc_comments_before(position)),
			"/// one\n/// two\n"
		);
		assert_eq!(render(comments.take_doc_comments_before(u32::MAX)), "");
	}

	#[test]
	fn takes_file_comments_verbatim() {
		let source = "//!  spaced\n//!\nfn f() {}\n";
		let arena = Arena::new();
		let mut comments = new_comments(&arena, source);

		assert_eq!(render(comments.take_file_comments()), "//!  spaced\n//!\n");
	}
}
