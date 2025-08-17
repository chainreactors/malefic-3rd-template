use std::collections::HashMap;

pub mod example;
mod prelude;

use prelude::*;

pub extern "C" fn register_3rd() -> MaleficBundle {
    let mut map: MaleficBundle = HashMap::new();
    register_module!(map, "example", example::Example);

    map
}

#[cfg(feature = "as_cdylib")]
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn register_modules() -> MaleficBundle {
    register_3rd()
}
