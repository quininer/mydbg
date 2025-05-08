use std::io::{ self, Write };
use std::pin::Pin;
use std::ops::Range;
use argh::FromArgs;
use anyhow::Context;
use autocxx::moveit::moveit;
use bstr::ByteSlice;
use crate::sys::lldb;
use crate::util::{ print_pretty_bytes, read_memory, u64ptr };


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
    register_only: bool
}

impl Command {
    pub fn execute(self, mut debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        let mut stdout = io::stdout().lock();

        let value = if self.is_64bit_pointer {
            Value::U64(u64ptr(self.value.as_str())?)
        } else if self.is_hex {
            let value = self.value.as_str();
            if let Some(value) = value.strip_prefix("0x") {
                let mut buf = data_encoding::HEXLOWER_PERMISSIVE.decode(value.as_bytes())
                    .context("hex decode failed")?;
                buf.reverse();
                Value::Bytes(buf)
            } else {
                let buf = data_encoding::HEXLOWER_PERMISSIVE.decode(value.as_bytes())
                    .context("hex decode failed")?;
                Value::Bytes(buf)
            }
        } else {
            Value::Bytes(self.value.into())
        };

        let thread_list = scan_threads_and_search_by_registers(&mut stdout, debugger.as_mut(), &value)?;

        if !self.register_only {
            search_by_all_memory_region(&mut stdout, debugger, &value, &thread_list)?;
        }

        stdout.flush()?;

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
    range: Range<u64>
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
            sp_range.start = std::cmp::min(sp_range.start, current_sp);
            sp_range.end = std::cmp::max(sp_range.end, current_sp);

            if frame.IsInlined1() {
                continue
            }

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
                            error.as_mut().Clear();
                            let reg_data = reg_data.as_mut().GetUnsignedInt64(error.as_mut(), 0);
                            error.Success() && reg_data == *v
                        },
                        Value::Bytes(v) => {
                            // # Safety
                            //
                            // read raw data from register
                            unsafe {
                                error.as_mut().Clear();
                                buf.clear();
                                buf.try_reserve_exact(reg_data_size).context("oom")?;

                                let reg_data_size = reg_data.as_mut().ReadRawData(
                                    error.as_mut(),
                                    0,
                                    buf.as_mut_ptr().cast(),
                                    reg_data_size
                                );

                                buf.set_len(reg_data_size);
                            }

                            error.Success() && memchr::memmem::find(&buf, v).is_some()
                        }
                    };

                    if hint {
                        writeln!(
                            stdout,"thread #{} {:?}, frame #{}, register {:?}",
                            thread_idx,
                            thread_name.as_ref().map(|b| b.as_bstr()),
                            frame_idx,
                            reg_name,
                        )?;

                        if let Value::U64(v) = value {
                            buf.clear();
                            buf.extend_from_slice(&v.to_le_bytes());
                        }

                        print_pretty_bytes(stdout, 0, &buf)?;
                        writeln!(stdout)?;
                    }
                }
            }
        }

        thread_list.push(Thread {
            name: thread_name.unwrap_or_default(),
            index: thread_idx,
            range: sp_range.context("no frame thread ?")?
        });
    }

    Ok(thread_list)
}

pub fn search_by_all_memory_region(
    stdout: &mut dyn Write,
    debugger: Pin<&mut lldb::SBDebugger>,
    value: &Value,
    thread_list: &[Thread],
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

        if mem_size > 4 * 1024 * 1024 * 1024 {
            writeln!(stdout, "memory region too large: {:?}", start_addr..end_addr)?;
            continue
        }

        let buf = match read_memory(
            process.as_mut(),
            &mut buf,
            start_addr,
            mem_size,
            error.as_mut()
        ) {
            Ok(buf) => buf,
            Err(_) => continue
        };

        let mut iter = match value {
            Value::U64(v) => {
                // # Safety
                //
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
            Value::Bytes(v) => either::Right(memchr::memmem::find_iter(buf, v))
        };

        let item = iter.next();
        if item.is_some() {
            writeln!(
                stdout,
                "[{:018p}-{:018p}] {}{}{} {:?}",
                start_addr as *const u8,
                end_addr as *const u8,
                if mem.as_mut().IsReadable() { 'r' } else { '-' },
                if mem.as_mut().IsWritable() { 'w' } else { '-' },
                if mem.as_mut().IsExecutable() { 'x' } else { '-' },
                cstr!(unsafe mem.as_mut().GetName()),
            )?;
        }

        for offset in item.into_iter().chain(iter) {
            let addr = start_addr + offset as u64;

            if let Some(thread) = thread_list.iter()
                .find(|thread| thread.range.contains(&addr))
            {
                writeln!(stdout, "by thread #{} {:?}", thread.index, thread.name.as_bstr())?;
            }

            let show_start = offset.checked_sub(16).unwrap_or(offset);
            let show_end = offset.saturating_add(value.len()).saturating_add(16);
            let show_end = std::cmp::min(show_end, buf.len());
            let show_addr_base = start_addr + show_start as u64;

            print_pretty_bytes(stdout, show_addr_base, &buf[show_start..show_end])?;
            writeln!(stdout)?;
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
