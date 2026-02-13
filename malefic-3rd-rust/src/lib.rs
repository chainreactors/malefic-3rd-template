use async_trait::async_trait;
use malefic_trait::module_impl;
use malefic_proto::prelude::*;

pub struct RustModule {}

#[async_trait]
#[module_impl("rust_module")]
impl Module for RustModule {}

#[async_trait]
impl ModuleImpl for RustModule {
    #[allow(unused_variables)]
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        sender: &mut Output,
    ) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;
        let response = Response {
            output: "this is rust module".to_string(),
            ..Default::default()
        };
        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}

/// Register the Rust module into the bundle.
pub fn register(map: &mut MaleficBundle) {
    let module = RustModule::new();
    map.insert(<RustModule as Module>::name().to_string(), Box::new(module));
}
