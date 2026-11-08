use dprint_core::configuration::resolve_new_line_kind;
use dprint_core::formatting::{Arena, ArenaPool, PrintOptions, print};
use lisette_syntax::ParseError;
use lisette_syntax::lex::Lexer;
use lisette_syntax::parse::{IMPORT_AFTER_ITEM_CODE, Parser};

use crate::config::Configuration;
use crate::generation::Context;

/// Returns `None` when the text is already formatted.
pub fn format_text(
	text: &str,
	config: &Configuration,
	arena_pool: Option<&ArenaPool>,
) -> Result<Option<String>, Vec<ParseError>> {
	let lexed = Lexer::new(text, 0).lex();
	if lexed.failed() {
		return Err(lexed.errors);
	}

	let parsed = Parser::new(lexed.tokens, text).parse();
	let only_recovered_imports = parsed
		.errors
		.iter()
		.all(|error| error.code == IMPORT_AFTER_ITEM_CODE);
	if parsed.truncated || !only_recovered_imports {
		return Err(parsed.errors);
	}

	let arena = match arena_pool {
		Some(arena_pool) => arena_pool.create_arena(),
		None => Arena::new(),
	};
	let context = Context::new(&arena, text);
	let print_items = context.generate(&parsed.ast);
	let output = print(print_items, print_options(text, config));

	if output == text {
		Ok(None)
	} else {
		Ok(Some(output))
	}
}

fn print_options(text: &str, config: &Configuration) -> PrintOptions {
	PrintOptions {
		max_width: config.line_width,
		indent_width: config.indent_width,
		use_tabs: config.use_tabs,
		new_line_text: resolve_new_line_kind(text, config.new_line_kind),
	}
}
