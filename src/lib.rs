#[macro_use]
mod util;
pub mod sys;
mod search;

use std::pin::Pin;


#[no_mangle]
pub unsafe extern "C" fn mydbg_search_do_execute(
    debugger: *mut libc::c_void,
    command: *const *const u8,
    _result: *mut libc::c_void
) -> bool {
    let debugger =
        Pin::new_unchecked(&mut *(debugger as *mut sys::lldb::SBDebugger));

    let cmd = match search::Command::from_ptr(command) {
        Ok(cmd) => cmd,
        Err(output) => {
            println!("{}", output);
            return false
        }
    };

    match cmd.execute(debugger) {
        Ok(()) => true,
        Err(err) => {
            println!("search failed: {:?}", err);
            false
        }
    }
}
