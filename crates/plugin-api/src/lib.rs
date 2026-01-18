pub mod bindings;

pub use bindings::exports::pruner::pruner::formatter::{FormatError, FormatOpts};

pub trait PluginApi {
  fn format(source: Vec<u8>, opts: FormatOpts) -> Result<Vec<u8>, FormatError>;
}

impl<T: PluginApi> bindings::exports::pruner::pruner::formatter::Guest for T {
  fn format(source: Vec<u8>, opts: FormatOpts) -> Result<Vec<u8>, FormatError> {
    T::format(source, opts)
  }
}
