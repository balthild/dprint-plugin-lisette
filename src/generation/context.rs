use dprint_core::formatting::{Arena, PrintItems, ir_helpers};

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
		if start >= end {
			return false;
		}

		let start = (start as usize).min(self.source.len());
		let end = (end as usize).min(self.source.len()).max(start);

		self.source.as_bytes()[start..end].contains(&b'\n')
	}
}
