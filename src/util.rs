macro_rules! cstr {
    ( unsafe $value:expr ) => {
        unsafe {
            let value = $value;
            if !value.is_null() {
                Some(std::ffi::CStr::from_ptr(value))
            } else {
                None
            }
        }
    }
}


fn pretty_print() -> anyhow::Result<()> {
    Ok(())
}
