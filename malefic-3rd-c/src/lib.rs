use malefic_module::prelude::*;
use malefic_module::ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn CModuleName() -> *const c_char;
    fn CModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;
}

pub struct CModule {
    name: String,
}

impl CModule {
    fn init() -> Self {
        let name = unsafe { ffi_module_name(CModuleName, false) };
        Self { name }
    }
}

#[async_trait]
impl Module for CModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "c_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        CModule::init()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(CModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for CModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        sender: &mut Output,
    ) -> ModuleResult {
        ffi_run_loop(id, receiver, sender, CModuleHandle, "CModuleHandle").await
    }
}

pub fn register(map: &mut MaleficBundle) {
    let module = CModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
