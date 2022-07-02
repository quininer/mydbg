use std::io::{ self, Write };
use std::ffi::CStr;
use std::pin::Pin;
use std::ops::Range;
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
    #[argh(switch, short = 'x')]
    is_hex: bool,

    /// value is 64bit pointer
    #[argh(switch, short = 'p')]
    is_64bit_pointer: bool,

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
        let mut stdout = io::stdout().lock();

        let value = if self.is_hex || self.is_64bit_pointer {
            let value = self.value.as_str();
            let value = value.strip_prefix("0x").unwrap_or(value);
            match data_encoding::HEXLOWER_PERMISSIVE.decode(value.as_bytes()) {
                Ok(value) if self.is_64bit_pointer => {
                    let value: [u8; 8] = value.try_into().ok().context("value length does not meet 64bit")?;
                    Value::U64(u64::from_be_bytes(value))
                },
                Ok(value) => Value::Bytes(value),
                Err(_) if self.is_64bit_pointer => {
                    let value = value.parse::<u64>().context("invalid u64 value")?;
                    Value::U64(value)
                },
                Err(err) => anyhow::bail!("invalid hex value: {:?}", err)
            }
        } else {
            Value::Bytes(self.value.into())
        };

        let thread_list = scan_threads_and_search_by_registers(&mut stdout, debugger.as_mut(), &value)?;

        if !self.register_only {
            search_by_memory(&mut stdout, debugger, &value, &thread_list)?;
        }

        Ok(())
    }
}

pub enum Value {
    U64(u64),
    Bytes(Vec<u8>)
}

pub struct Thread {
    name: Vec<u8>,
    index: usize,
    range: Option<Range<u64>>
}

pub fn scan_threads_and_search_by_registers(
    stdout: &mut dyn Write,
    debugger: Pin<&mut lldb::SBDebugger>,
    value: &Value,
) -> anyhow::Result<Vec<Thread>> {
    moveit!{
        let mut target = debugger.GetSelectedTarget();
        let mut process = target.as_mut().GetProcess();
        let mut error = lldb::SBError::new();
    }

    let mut thread_list = Vec::new();
    let mut buf: Vec<u8> = Vec::new();

    let threads = process.as_mut().GetNumThreads() as usize;
    for thread_idx in 0..threads {
        moveit!(let mut thread = process.as_mut().GetThreadAtIndex(thread_idx));

        let thread_name = cstr!(unsafe thread.GetName())
            .map(|name| Vec::from(name.to_bytes()));
        let mut sp_range = None;

        let frames = thread.as_mut().GetNumFrames();
        for frame_idx in 0..frames {
            moveit!{
                let mut frame = thread.as_mut().GetFrameAtIndex(frame_idx);
                let registers = frame.as_mut().GetRegisters();
            }

            // find stack scope
            // https://github.com/llvm/llvm-project/blob/main/lldb/examples/darwin/heap_find/heap.py#L1172
            let current_sp = frame.GetSP();
            let sp_range = sp_range.get_or_insert_with(|| current_sp..current_sp);
            sp_range.start = std::cmp::max(sp_range.start, current_sp);
            sp_range.end = std::cmp::min(sp_range.end, current_sp);

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

                    let hint = match value {
                        Value::U64(v) => {
                            let reg_data = reg_data.as_mut().GetUnsignedInt64(error.as_mut(), 0);
                            reg_data == *v
                        },
                        Value::Bytes(v) => {
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

                            memchr::memmem::find(&buf, v).is_some()
                        }
                    };

                    if hint {
                        moveit!(let symbol = frame.GetSymbol());
                        let frame_name = cstr!(unsafe symbol.GetName());

                        writeln!(
                            stdout,
                            "thread: {:?}; frame: {:?}; reg_name: {:?}: reg_value: {:?}",
                            thread_name.as_deref().unwrap_or(b"<unknown>").as_bstr(),
                            frame_name,
                            reg_name,
                            buf
                        )?;
                    }
                }
            }
        }

        thread_list.push(Thread {
            name: thread_name.unwrap_or_default(),
            index: thread_idx,
            range: sp_range
        });
    }

    Ok(thread_list)
}

pub fn search_by_memory(
    stdout: &mut dyn Write,
    debugger: Pin<&mut lldb::SBDebugger>,
    value: &Value,
    thread_list: &[Thread]
) -> anyhow::Result<()> {
    moveit!{
        let mut target = debugger.GetSelectedTarget();
        let mut process = target.as_mut().GetProcess();
        let mut mem_list = process.as_mut().GetMemoryRegions();
        let mut mem = lldb::SBMemoryRegionInfo::new();
        let mut error = lldb::SBError::new();
    }

    let mut buf: Vec<u8> = Vec::new();

    let mem_len = mem_list.GetSize();
    for mem_idx in 0..mem_len {
        let ret = mem_list.as_mut().GetMemoryRegionAtIndex(mem_idx, mem.as_mut());

        if !ret {
            continue // warn ?
        }

        if !mem.as_mut().IsReadable() {
            continue
        }

        let start_addr = mem.as_mut().GetRegionBase();
        let end_addr = mem.as_mut().GetRegionEnd();
        let mem_size: usize = end_addr
            .checked_sub(start_addr)
            .and_then(|size| size.try_into().ok())
            .with_context(|| format!("invalid region addr: {}..{}", start_addr, end_addr))?;

        // # Safety
        //
        // read raw data from memory
        unsafe {
            buf.clear();
            buf.reserve_exact(mem_size);

            let buf_len = process.as_mut().ReadMemory(
                start_addr,
                buf.as_mut_ptr().cast(),
                mem_size,
                error.as_mut()
            );

            buf.set_len(buf_len);
        }

        let iter = match value {
            Value::U64(v) => {
                // assume it's always aligned to a u64 pointer
                let buf = unsafe {
                    let (prefix, buf, _) = buf.align_to::<u64>();
                    assert!(prefix.is_empty());
                    buf
                };

                // TODO simd it
                let iter = buf.iter()
                    .enumerate()
                    .filter(|(_, x)| **x == *v)
                    .map(|(i, _)| i * std::mem::size_of::<u64>());
                either::Left(iter)
            },
            Value::Bytes(v) => {
                either::Right(memchr::memmem::find_iter(&buf, v))
            }
        };

        for offset in iter {
            let addr = start_addr + offset as u64;

            writeln!(
                stdout,
                "addr: {:p}", addr as *const ()
            )?;
        }
    }

    Ok(())
}

impl Value {
    pub fn len(&self) -> usize {
        match self {
            Value::U64(_) => std::mem::size_of::<u64>(),
            Value::Bytes(v) => v.len()
        }
    }
}
