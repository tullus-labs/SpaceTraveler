pub trait StarShipPluginAPI {
    fn name(&self) -> &'static str;
    fn execute(&self);
}

#[macro_export]
macro_rules! export_plugin {
    ($plugin_type:ty) => {
        #[no_mangle]
        pub extern "C" fn export_plugin() -> Box<dyn $crate::StarShipPluginAPI> {
            Box::new($plugin_type)
        }
    };
}