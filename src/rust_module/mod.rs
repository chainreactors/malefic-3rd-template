use crate::prelude::*;

pub struct RustModule {}

#[async_trait]
#[module_impl("rust_module")]
impl Module for RustModule {}

#[async_trait]
impl ModuleImpl for RustModule {
    #[allow(unused_variables)]
    async fn run(&mut self, id: u32, receiver: &mut crate::Input, sender: &mut crate::Output) -> ModuleResult {
        let request = check_request!(receiver, Body::Request)?;
        let response = Response {
            output: "this is rust module".to_string(),
            ..Default::default()
        };
        Ok(TaskResult::new_with_body(id, Body::Response(response)))
    }
}