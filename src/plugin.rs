use dprint_core::configuration::{ConfigKeyMap, GlobalConfiguration};
use dprint_core::generate_plugin_code;
use dprint_core::plugins::{
	CheckConfigUpdatesMessage, ConfigChange, FileMatchingInfo, FormatError, FormatResult, PluginInfo,
	PluginResolveConfigurationResult, SyncFormatRequest, SyncHostFormatRequest, SyncPluginHandler,
};

use crate::config::Configuration;

pub struct LisettePluginHandler;

impl SyncPluginHandler<Configuration> for LisettePluginHandler {
	fn plugin_info(&mut self) -> PluginInfo {
		let repo = "balthild/dprint-plugin-lisette";
		let package = env!("CARGO_PKG_NAME");
		let version = env!("CARGO_PKG_VERSION");

		PluginInfo {
			name: package.to_string(),
			version: version.to_string(),
			config_key: "lisette".to_string(),
			help_url: "".to_string(),
			config_schema_url: format!("https://plugins.dprint.dev/{repo}/{version}/schema.json",),
			update_url: Some(format!("https://plugins.dprint.dev/{repo}/latest.json")),
			supports_range_formatting: None,
		}
	}

	fn license_text(&mut self) -> String {
		include_str!("../LICENSE").to_string()
	}

	fn resolve_config(
		&mut self,
		_config: ConfigKeyMap,
		_global_config: &GlobalConfiguration,
	) -> PluginResolveConfigurationResult<Configuration> {
		PluginResolveConfigurationResult {
			config: Configuration {},
			diagnostics: vec![],
			file_matching: FileMatchingInfo {
				file_extensions: vec!["lis".to_string()],
				file_names: vec![],
				additive: false,
			},
		}
	}

	fn check_config_updates(
		&self,
		_message: CheckConfigUpdatesMessage,
	) -> Result<Vec<ConfigChange>, FormatError> {
		Ok(Vec::new())
	}

	fn format(
		&mut self,
		_request: SyncFormatRequest<Configuration>,
		mut _format_with_host: impl FnMut(SyncHostFormatRequest) -> FormatResult,
	) -> FormatResult {
		// TODO: format through the dprint-core IR once the generator exists.
		Ok(None)
	}
}

generate_plugin_code!(LisettePluginHandler, LisettePluginHandler);
