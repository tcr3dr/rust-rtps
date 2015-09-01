#[no_mangle]
pub extern fn rmw_get_implementation_identifier() -> *const u8 {
    "libdds\0".as_ptr()
}
