use dprint_core::formatting::{Arena, PrintItems, Signal, ir_helpers};

use crate::config::Configuration;
use crate::generation::Comments;

pub(crate) struct Context<'a> {
	pub(crate) arena: &'a Arena,
	pub(crate) source: &'a str,
	pub(crate) config: &'a Configuration,
	pub(crate) comments: Comments<'a>,
}

impl<'a> Context<'a> {
	pub(crate) fn new(
		arena: &'a Arena,
		source: &'a str,
		config: &'a Configuration,
		comments: Comments<'a>,
	) -> Self {
		Self {
			arena,
			source,
			config,
			comments,
		}
	}

	/// The source from `start` to `end` as written.
	pub(crate) fn verbatim(&self, start: u32, end: u32) -> PrintItems<'a> {
		let start = (start as usize).min(self.source.len());
		let end = (end as usize).min(self.source.len()).max(start);

		ir_helpers::gen_from_raw_string(self.arena, &self.source[start..end])
	}

	/// Whether the source between two positions has a line break, which is how a body the
	/// author wrote over several lines stays that way.
	pub(crate) fn has_line_break_between(&self, start: u32, end: u32) -> bool {
		self.newline_count_between(start, end) > 0
	}

	/// How many line breaks the source has between two positions.
	pub(crate) fn newline_count_between(&self, start: u32, end: u32) -> usize {
		if start >= end {
			return 0;
		}

		let start = (start as usize).min(self.source.len());
		let end = (end as usize).min(self.source.len()).max(start);

		self.source.as_bytes()[start..end]
			.iter()
			.filter(|byte| **byte == b'\n')
			.count()
	}

	/// Puts leading comments on their own lines before the items they belong to.
	pub(crate) fn with_leading(
		&self,
		leading: Option<PrintItems<'a>>,
		items: PrintItems<'a>,
	) -> PrintItems<'a> {
		match leading {
			Some(mut leading) => {
				leading.push_signal(Signal::NewLine);
				leading.extend(items);
				leading
			}
			None => items,
		}
	}
}
