pub mod bindings;

pub use bindings::exports::pruner::plugin_api::formatter::{FormatError, FormatOpts};

pub trait PluginApi {
  fn format(source: Vec<u8>, opts: FormatOpts) -> Result<Vec<u8>, FormatError>;
}

impl<T: PluginApi> bindings::exports::pruner::plugin_api::formatter::Guest for T {
  fn format(source: Vec<u8>, opts: FormatOpts) -> Result<Vec<u8>, FormatError> {
    T::format(source, opts)
  }
}
