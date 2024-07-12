#[macro_use]
mod util;
mod sys;
mod search;
mod read;
mod thread;
mod debug;

use std::pin::Pin;


macro_rules! define {
    ( $name:ident, $cmd:ident ) => {
        #[no_mangle]
        pub unsafe extern "C" fn $name(
            debugger: *mut libc::c_void,
            command: *const *const u8,
            _result: *mut libc::c_void
        ) -> bool {
            let debugger =
                Pin::new_unchecked(&mut *(debugger as *mut sys::lldb::SBDebugger));

            let cmd = match util::command_from_ptr::<$cmd::Command>(stringify!($cmd), command) {
                Ok(cmd) => cmd,
                Err(output) => {
                    println!("{}", output);
                    return false
                }
            };

            match cmd.execute(debugger) {
                Ok(()) => true,
                Err(err) => {
                    println!("call failed: {:?}", err);
                    false
                }
            }
        }
    }
}

define!(mydbg_search_do_execute, search);
define!(mydbg_read_do_execute, read);
define!(mydbg_thread_do_execute, search);
define!(mydbg_debug_do_execute, debug);
