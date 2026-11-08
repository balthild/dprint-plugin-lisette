use dprint_core::configuration::{
	ConfigKeyMap, GlobalConfiguration, NewLineKind, RECOMMENDED_GLOBAL_CONFIGURATION, RawNewLineKind,
	ResolveConfigurationResult, get_nullable_value, get_unknown_property_diagnostics, get_value,
};
use serde::Serialize;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Configuration {
	pub line_width: u32,
	pub indent_width: u8,
	pub use_tabs: bool,
	pub new_line_kind: NewLineKind,
}

pub fn resolve_config(
	mut config: ConfigKeyMap,
	global_config: &GlobalConfiguration,
) -> ResolveConfigurationResult<Configuration> {
	let mut diagnostics = Vec::new();

	let line_width = get_value(
		&mut config,
		"lineWidth",
		global_config
			.line_width
			.unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.line_width),
		&mut diagnostics,
	);
	let indent_width = get_value(
		&mut config,
		"indentWidth",
		global_config
			.indent_width
			.unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.indent_width),
		&mut diagnostics,
	);
	let use_tabs = get_value(
		&mut config,
		"useTabs",
		global_config
			.use_tabs
			.unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.use_tabs),
		&mut diagnostics,
	);
	let new_line_kind =
		get_nullable_value::<RawNewLineKind>(&mut config, "newLineKind", &mut diagnostics)
			.map(convert_new_line_kind)
			.or(global_config.new_line_kind)
			.unwrap_or(RECOMMENDED_GLOBAL_CONFIGURATION.new_line_kind);

	diagnostics.extend(get_unknown_property_diagnostics(config));

	ResolveConfigurationResult {
		config: Configuration {
			line_width,
			indent_width,
			use_tabs,
			new_line_kind,
		},
		diagnostics,
	}
}

fn convert_new_line_kind(kind: RawNewLineKind) -> NewLineKind {
	match kind {
		RawNewLineKind::Auto => NewLineKind::Auto,
		RawNewLineKind::LineFeed => NewLineKind::LineFeed,
		RawNewLineKind::CarriageReturnLineFeed => NewLineKind::CarriageReturnLineFeed,
		RawNewLineKind::System => {
			if cfg!(windows) {
				NewLineKind::CarriageReturnLineFeed
			} else {
				NewLineKind::LineFeed
			}
		}
	}
}

#[cfg(test)]
mod tests {
	use dprint_core::configuration::ConfigKeyValue;

	use super::*;

	fn resolve(config: ConfigKeyMap) -> ResolveConfigurationResult<Configuration> {
		resolve_config(config, &GlobalConfiguration::default())
	}

	#[test]
	fn falls_back_to_the_recommended_defaults() {
		let resolved = resolve(ConfigKeyMap::new());

		assert!(resolved.diagnostics.is_empty());
		assert_eq!(
			resolved.config.line_width,
			RECOMMENDED_GLOBAL_CONFIGURATION.line_width
		);
		assert_eq!(
			resolved.config.indent_width,
			RECOMMENDED_GLOBAL_CONFIGURATION.indent_width
		);
		assert_eq!(
			resolved.config.use_tabs,
			RECOMMENDED_GLOBAL_CONFIGURATION.use_tabs
		);
		assert_eq!(
			resolved.config.new_line_kind,
			RECOMMENDED_GLOBAL_CONFIGURATION.new_line_kind
		);
	}

	#[test]
	fn the_plugin_key_wins_over_the_global_one() {
		let global_config = GlobalConfiguration {
			line_width: Some(200),
			..Default::default()
		};
		let mut config = ConfigKeyMap::new();
		config.insert("lineWidth".to_string(), ConfigKeyValue::from_i32(60));

		let resolved = resolve_config(config, &global_config);

		assert_eq!(resolved.config.line_width, 60);
	}

	#[test]
	fn falls_back_to_the_global_value() {
		let global_config = GlobalConfiguration {
			line_width: Some(200),
			..Default::default()
		};

		let resolved = resolve_config(ConfigKeyMap::new(), &global_config);

		assert_eq!(resolved.config.line_width, 200);
	}

	#[test]
	fn reports_unknown_properties() {
		let mut config = ConfigKeyMap::new();
		config.insert("lineWidht".to_string(), ConfigKeyValue::from_i32(60));

		let resolved = resolve(config);

		assert_eq!(resolved.diagnostics.len(), 1);
		assert_eq!(resolved.diagnostics[0].property_name, "lineWidht");
	}
}
