use std::borrow::Cow;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration};
use dprint_plugin_lisette::{Configuration, format_text, resolve_config};

fn main() -> ExitCode {
	let mut args = std::env::args_os().skip(1);
	let Some(path) = args.next().map(PathBuf::from) else {
		eprintln!("usage: format <file.lis> [--write]");
		return ExitCode::FAILURE;
	};
	let write = args.any(|arg| arg == "--write");

	let text = match std::fs::read_to_string(&path) {
		Ok(text) => text,
		Err(error) => {
			eprintln!("failed to read {}: {error}", path.display());
			return ExitCode::FAILURE;
		}
	};

	let resolved = resolve_config(ConfigKeyMap::new(), &GlobalConfiguration::default());
	for diagnostic in &resolved.diagnostics {
		eprintln!("config warning: {diagnostic}");
	}

	let (formatted, changed) = match format_text(&text, &resolved.config, None) {
		Ok(Some(formatted)) => (Cow::Owned(formatted), true),
		Ok(None) => (Cow::Borrowed(text.as_str()), false),
		Err(errors) => {
			for error in errors {
				report_error(&path, &text, &error);
			}
			return ExitCode::FAILURE;
		}
	};

	if write {
		if changed {
			if let Err(error) = std::fs::write(&path, &*formatted) {
				eprintln!("failed to write {}: {error}", path.display());
				return ExitCode::FAILURE;
			}
			eprintln!("{}: formatted", path.display());
		} else {
			eprintln!("{}: already formatted", path.display());
		}
	} else {
		if !changed {
			eprintln!("{}: already formatted", path.display());
		}
		print!("{formatted}");
	}

	match stability_problem(&formatted, &resolved.config) {
		Some(problem) => {
			eprintln!("warning: {problem}");
			ExitCode::FAILURE
		}
		None => ExitCode::SUCCESS,
	}
}

fn stability_problem(text: &str, config: &Configuration) -> Option<String> {
	match format_text(text, config, None) {
		Ok(None) => None,
		Ok(Some(_)) => Some("formatting the formatted text changes it again".to_string()),
		Err(errors) => Some(match errors.first() {
			Some(error) => format!("the formatted text no longer parses: {}", error.message),
			None => "the formatted text no longer parses".to_string(),
		}),
	}
}

fn report_error(path: &Path, text: &str, error: &lisette_syntax::ParseError) {
	eprintln!("{}: {} ({})", path.display(), error.message, error.code);
	for (span, label) in &error.labels {
		let (line, column) = line_and_column(text, span.byte_offset);
		eprintln!("  {line}:{column}: {label}");
	}
	if let Some(help) = &error.help {
		eprintln!("  help: {help}");
	}
	if let Some(note) = &error.note {
		eprintln!("  note: {note}");
	}
}

fn line_and_column(text: &str, offset: u32) -> (usize, usize) {
	let offset = (offset as usize).min(text.len());
	let mut line = 1;
	let mut line_start = 0;
	for (index, byte) in text.as_bytes()[..offset].iter().enumerate() {
		if *byte == b'\n' {
			line += 1;
			line_start = index + 1;
		}
	}
	(line, offset - line_start + 1)
}
