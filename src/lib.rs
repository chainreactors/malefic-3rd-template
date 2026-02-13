use std::collections::HashMap;

mod prelude;
use prelude::*;

#[allow(improper_ctypes_definitions)]
pub extern "C" fn register_3rd() -> MaleficBundle {
    let mut map: MaleficBundle = HashMap::new();

    #[cfg(feature = "rust_module")]
    malefic_3rd_rust::register(&mut map);

    #[cfg(feature = "golang_module")]
    malefic_3rd_go::register(&mut map);

    #[cfg(feature = "c_module")]
    malefic_3rd_c::register(&mut map);

    map
}

#[cfg(feature = "as_cdylib")]
#[no_mangle]
#[allow(improper_ctypes_definitions)]
pub extern "C" fn register_modules() -> MaleficBundle {
    register_3rd()
}
