use anyhow::{Context, Result};
use std::{path::PathBuf, time::Instant};
use wasmtime::{Engine, component::Linker};
use wasmtime_wasi::{ResourceTable, WasiCtx, WasiCtxView, WasiView};

use super::registry;
use crate::{
  api::format::FormatOpts,
  config::Config,
  wasm::bindings::{Plugin, exports::pruner::plugin_api},
};

struct ComponentState {
  table: ResourceTable,
  wasi: WasiCtx,
}

impl WasiView for ComponentState {
  fn ctx(&mut self) -> WasiCtxView<'_> {
    WasiCtxView {
      ctx: &mut self.wasi,
      table: &mut self.table,
    }
  }
}

impl ComponentState {
  pub fn new() -> Self {
    Self {
      table: ResourceTable::new(),
      wasi: WasiCtx::builder().build(),
    }
  }
}

pub struct WasmFormatter {
  engine: Engine,
  linker: Linker<ComponentState>,
  registry: registry::ComponentRegistry,
}

impl WasmFormatter {
  pub fn new(cache_dir: PathBuf) -> Result<Self> {
    let engine = Engine::default();

    let mut linker = wasmtime::component::Linker::new(&engine);
    wasmtime_wasi::p2::add_to_linker_sync(&mut linker)
      .context("Failed to add wasi interface to linker")?;

    let registry = registry::ComponentRegistry::new(engine.clone(), cache_dir);

    Ok(Self {
      engine,
      linker,
      registry,
    })
  }

  pub fn from_config(config: &Config) -> Result<Self> {
    let mut formatter = Self::new(config.cache_dir.clone())?;
    for (name, spec) in &config.plugins {
      formatter.registry.load_component(name, spec.url())?;
    }
    Ok(formatter)
  }

  pub fn has_formatter(&self, name: &str) -> bool {
    self.registry.has_component(name)
  }

  pub fn format(&self, name: &str, source: &[u8], opts: &FormatOpts) -> Result<Vec<u8>> {
    let start = Instant::now();

    let mut store = wasmtime::Store::new(&self.engine, ComponentState::new());
    let Some(component) = self.registry.get_component(name) else {
      anyhow::bail!("Unknown formatter {name}");
    };
    let plugin = Plugin::instantiate(&mut store, component, &self.linker)?;

    log::trace!(
      "Component [{name}] instantiated in: {:?}",
      Instant::now().duration_since(start)
    );

    let res = plugin
      .pruner_plugin_api_formatter()
      .call_format(
        &mut store,
        source,
        &plugin_api::formatter::FormatOpts {
          print_width: opts.printwidth,
          lang: opts.language.into(),
        },
      )?
      .map_err(anyhow::Error::from);

    log::debug!(
      "Formatted using [{name}] in {:?}",
      Instant::now().duration_since(start)
    );

    res
  }
}
