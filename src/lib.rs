use std::collections::HashMap;

#[cfg(feature = "rust_module")]
pub mod rust_module;
#[cfg(feature = "golang_module")]
pub mod golang_module;
mod prelude;

use prelude::*;

#[allow(improper_ctypes_definitions)]
pub extern "C" fn register_3rd() -> MaleficBundle {
    let mut map: MaleficBundle = HashMap::new();

    register_module!(map, "rust_module", rust_module::RustModule);

    #[cfg(feature = "golang_module")]
    golang_module::register(&mut map);

    map
}

#[cfg(feature = "as_cdylib")]
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn register_modules() -> MaleficBundle {
    register_3rd()
}
