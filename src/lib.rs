#[macro_use]
mod util;
mod sys;
mod search;
mod read;
mod thread;
mod trace;

use std::pin::Pin;


macro_rules! command {
    ( $sym:ident = $cmd:ident ) => {
        #[no_mangle]
        pub unsafe extern "C" fn $sym(
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
                    println!("{} failed: {:?}", stringify!($cmd), err);
                    false
                }
            }
        }
    };
}

command!(mydbg_search_do_execute = search);
command!(mydbg_read_do_execute = read);
command!(mydbg_thread_do_execute = thread);
command!(mydbg_trace_do_execute = trace);
