use dprint_core::formatting::{Arena, PrintItems, ir_helpers};
use lisette_syntax::ast::Expression;

pub(crate) struct Context<'a> {
	pub(crate) arena: &'a Arena,
	pub(crate) source: &'a str,
}

impl<'a> Context<'a> {
	pub(crate) fn new(arena: &'a Arena, source: &'a str) -> Self {
		Self { arena, source }
	}

	pub(crate) fn generate(&self, _top_level_items: &[Expression]) -> PrintItems<'a> {
		ir_helpers::gen_from_raw_string(self.arena, self.source)
	}
}
