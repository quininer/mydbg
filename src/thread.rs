use std::mem;
use std::io::{ self, Write };
use std::pin::Pin;
use argh::FromArgs;
use autocxx::moveit::moveit;
use bstr::ByteSlice;
use human_size::SpecificSize;
use human_size::multiples::{ Byte, Kibibyte };
use crate::sys::lldb;


/// MyDbg thread command
#[derive(FromArgs)]
pub struct Command {
    //
}

impl Command {
    pub fn execute(self, debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        let mut stdout = io::stdout().lock();

        moveit!{
            let mut target = debugger.GetSelectedTarget();
            let mut process = target.as_mut().GetProcess();
            let mut thread = process.as_mut().GetSelectedThread();
        }

        let mut sp_range = None;
        let mut last_sp = None;

        let frames = thread.as_mut().GetNumFrames();
        for frame_idx in 0..frames {
            moveit!{
                let mut frame = thread.as_mut().GetFrameAtIndex(frame_idx);
                let mut symbol = frame.as_mut().GetSymbol();
            }

            // find stack scope
            // https://github.com/llvm/llvm-project/blob/main/lldb/examples/darwin/heap_find/heap.py#L1172
            let current_sp = frame.GetSP();
            let sp_range = sp_range.get_or_insert_with(|| current_sp..current_sp);
            sp_range.start = std::cmp::min(sp_range.start, current_sp);
            sp_range.end = std::cmp::max(sp_range.end, current_sp);

            let last_sp = mem::replace(last_sp.get_or_insert(current_sp), current_sp);
            let stack_size = current_sp.saturating_sub(last_sp);

            if frame.IsInlined1() && stack_size == 0 {
                continue;
            }

            let stack_size = SpecificSize::new(stack_size as f64, Byte)?.into::<Kibibyte>();

            let symbol_name = cstr!(unsafe symbol.GetName())
                .map(|name| Vec::from(name.to_bytes()))
                .unwrap_or_default();

            moveit!{
                let variables = frame.as_mut().GetVariables(
                    true,
                    true,
                    false,
                    true
                );
            }
            let mut list = Vec::new();

            let count = variables.GetSize();
            for i in 0..count {
                moveit!{
                    let mut value = variables.GetValueAtIndex(i);
                }

                let ty = cstr!(unsafe value.as_mut().GetTypeName())
                    .map(|name| Vec::from(name.to_bytes()))
                    .unwrap_or_default();
                let name = cstr!(unsafe value.as_mut().GetName())
                    .map(|name| Vec::from(name.to_bytes()))
                    .unwrap_or_default();
                let size = value.as_mut().GetByteSize();
                let size = SpecificSize::new(size as f64, Byte)?.into::<Kibibyte>();
                list.push((ty, name, size));
            }

            writeln!(
                &mut stdout,
                "#{} size= {}; frame= {:?}",
                frame_idx,
                stack_size,
                symbol_name.as_bstr()
            )?;

            for (ty, name, size) in list {
                writeln!(
                    &mut stdout,
                    "let {}: {:?} = {};",
                    name.as_bstr(),
                    ty.as_bstr(),
                    size
                )?;
            }

            writeln!(&mut stdout)?;
        }

        Ok(())
    }
}
