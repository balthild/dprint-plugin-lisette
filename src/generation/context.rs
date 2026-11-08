use dprint_core::formatting::{Arena, PrintItems, ir_helpers};

use crate::generation::Comments;

pub(crate) struct Context<'a> {
	pub(crate) arena: &'a Arena,
	pub(crate) source: &'a str,
	pub(crate) comments: Comments<'a>,
}

impl<'a> Context<'a> {
	pub(crate) fn new(arena: &'a Arena, source: &'a str, comments: Comments<'a>) -> Self {
		Self {
			arena,
			source,
			comments,
		}
	}

	/// The source from `start` to `end` as written.
	pub(crate) fn verbatim(&self, start: u32, end: u32) -> PrintItems<'a> {
		let start = (start as usize).min(self.source.len());
		let end = (end as usize).min(self.source.len()).max(start);

		ir_helpers::gen_from_raw_string(self.arena, &self.source[start..end])
	}
}
