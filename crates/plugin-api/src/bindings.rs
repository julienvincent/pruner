wit_bindgen::generate!({
  world: "pruner:pruner/pruner@1.0.0",
  path: "../../wit",
  pub_export_macro: true,
  default_bindings_module: "pruner_plugin_api::bindings",
});
