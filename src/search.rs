use std::ffi::CStr;
use std::pin::Pin;
use argh::FromArgs;
use anyhow::Context;
use autocxx::moveit::moveit;
use bstr::ByteSlice;
use crate::sys::lldb;


/// MyDbg Search command
#[derive(FromArgs)]
pub struct Command {
    /// search by value
    #[argh(positional)]
    value: String,

    /// value is hex encoded
    #[argh(switch)]
    is_hex: bool,

    /// search register only
    #[argh(switch)]
    register_only: bool,

    /// search memory start address
    #[argh(option)]
    memory_start: Option<u64>,

    /// search memory end address
    #[argh(option)]
    memory_end: Option<u64>
}

impl Command {
    pub unsafe fn from_ptr(command: *const *const u8) -> Result<Command, String> {
        let mut args = Vec::new();

        if !command.is_null() {
            let mut arg: *const u8 = *command;
            while !arg.is_null() {
                let argx = CStr::from_ptr(arg.cast()).to_str()
                    .map_err(|err| format!("invalid argument: {:?}", err))?;
                args.push(argx);
                arg = *command.add(args.len());
            }
        }

        Command::from_args(&["search"], &args)
            .map_err(|err| err.output)
    }

    pub fn execute(self, mut debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        let value = if self.is_hex {
            let value = self.value.as_str();
            let value = value.strip_prefix("0x").unwrap_or(value);
            data_encoding::HEXLOWER_PERMISSIVE
                .decode(value.as_bytes())
                .context("invalid hex value")?
        } else {
            self.value.into()
        };

        search_by_registers(debugger.as_mut(), &value)?;

        Ok(())
    }
}

pub fn search_by_registers(debugger: Pin<&mut lldb::SBDebugger>, value: &[u8]) -> anyhow::Result<()> {
    moveit!{
        let mut target = debugger.GetSelectedTarget();
        let mut process = target.as_mut().GetProcess();
        let mut error = lldb::SBError::new();
    }

    let mut buf: Vec<u8> = Vec::new();

    let threads = process.as_mut().GetNumThreads() as usize;
    for thread_idx in 0..threads {
        moveit!(let mut thread = process.as_mut().GetThreadAtIndex(thread_idx));
        let mut maybe_thread_name = None;

        let frames = thread.as_mut().GetNumFrames();
        for frame_idx in 0..frames {
            moveit!{
                let mut frame = thread.as_mut().GetFrameAtIndex(frame_idx);
                let registers = frame.as_mut().GetRegisters();
            }
            let mut maybe_frame_name = None;

            let regs_list_len = registers.GetSize();
            for regs_list_idx in 0..regs_list_len {
                moveit!(let mut regs = registers.GetValueAtIndex(regs_list_idx));

                let regs_len = regs.as_mut().GetNumChildren();
                for regs_idx in 0..regs_len {
                    moveit!{
                        let mut reg = regs.as_mut().GetChildAtIndex(regs_idx);
                        let mut reg_data = reg.as_mut().GetData();
                    };

                    let reg_name = cstr!(unsafe reg.as_mut().GetName());
                    let reg_data_size = reg_data.as_mut().GetByteSize();

                    if reg_data_size == 0 || reg_data_size > 128 || reg_data_size < value.len() {
                        continue
                    }

                    // # Safety
                    //
                    // read raw data from register
                    unsafe {
                        buf.clear();
                        buf.reserve_exact(reg_data_size);

                        let reg_data_size = reg_data.as_mut().ReadRawData(
                            error.as_mut(),
                            0,
                            buf.as_mut_ptr().cast(),
                            reg_data_size
                        );

                        buf.set_len(reg_data_size);
                    }

                    if memchr::memmem::find(&buf, value).is_some() {
                        let thread_name = if let Some(thread_name) = maybe_thread_name.as_ref() {
                            thread_name
                        } else {
                            let thread_name2 = cstr!(unsafe thread.GetName());
                            maybe_thread_name.get_or_insert(thread_name2)
                        };

                        let frame_name = if let Some(frame_name) = maybe_frame_name.as_ref() {
                            frame_name
                        } else {
                            moveit!(let symbol = frame.GetSymbol());
                            let frame_name2 = cstr!(unsafe symbol.GetName());

                            maybe_frame_name.get_or_insert(frame_name2)
                        };

                        println!(
                            "thread: {:?}; frame: {:?}; reg_name: {:?}: reg_value: {:?}",
                            thread_name,
                            frame_name,
                            reg_name,
                            buf
                        );
                    }
                }
            }
        }
    }

    Ok(())
}


pub fn search_by_stacks(debugger: Pin<&mut lldb::SBDebugger>, value: &[u8]) -> anyhow::Result<()> {
    todo!()
}
