use std::pin::Pin;
use std::ffi::CString;
use argh::FromArgs;
use anyhow::Context;
use autocxx::moveit::moveit;
use crate::sys::lldb;


/// MyDbg Debug command
#[derive(FromArgs)]
pub struct Command {
    /// debug variable
    #[argh(positional)]
    variable: String,
}

impl Command {
    pub fn execute(self, mut debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        moveit!{
            let mut target = debugger.as_mut().GetSelectedTarget();
            let mut process = target.as_mut().GetProcess();
            let mut thread = process.as_mut().GetSelectedThread();
            let mut frame = thread.as_mut().GetSelectedFrame();
        }

        let (value, func) = {
            let var = CString::new(self.variable)?;
            let var = var.as_c_str();

            moveit!{
                let mut value = unsafe {
                    frame.as_mut().FindVariable(var.as_ptr())
                };
            }

            if !value.as_mut().IsValid() || !value.as_mut().TypeIsPointerType() {
                anyhow::bail!("the variable must be a reference");
            }

            moveit!{
                let mut value = value.as_mut().Dereference(); // TODO deref all ?
                let mut value_addr = value.as_mut().GetAddress();
            }
            let value_addr = value_addr.as_mut().GetOffset();

            let type_name_cstr = value.as_mut().GetDisplayTypeName();
            let type_name_cstr = cstr!(unsafe type_name_cstr).context("emtpy type name")?;
            let type_name = type_name_cstr.to_str().context("non-utf8 type name")?;
            let debug_func_name = format!("<{} as mydbg_debug::MyDebug>::debug_to_stdout", type_name);
            let debug_func_name = CString::new(debug_func_name)?;
            let debug_func_name = debug_func_name.as_c_str();

            moveit!{
                let mut list = unsafe {
                    target.as_mut().FindFunctions(debug_func_name.as_ptr(), 1 << 1) // method (?)
                };
            }

            let len = list.GetSize();
            let mut maybe_func = None;
            for idx in 0..len {
                moveit!{
                    let mut symbol = list.as_mut().GetContextAtIndex(idx);
                    let mut symbol = symbol.as_mut().GetSymbol();
                    let mut addr = symbol.as_mut().GetStartAddress();
                }

                if !addr.IsValid() {
                    continue
                }

                maybe_func = Some(addr.as_mut().GetLoadAddress(&target));
                break
            }

            let func = maybe_func.context("debug function not found")?;

            (value_addr, func)
        };

        let mut eval = |expr| -> anyhow::Result<()> {
            let expr = CString::new(expr)?;
            let expr = expr.as_c_str();

            moveit!{
                let mut value = unsafe {
                    target.as_mut().EvaluateExpression(expr.as_ptr())
                };
                let error = value.as_mut().GetError();
            }

            if value.as_mut().IsValid() || error.Success() {
                Ok(())
            } else {
                let err_msg = cstr!(unsafe error.GetCString());
                anyhow::bail!("eval failed: {:?}", err_msg)
            }
        };

        // https://stackoverflow.com/questions/21096045/lldb-how-to-call-a-function-from-a-specific-library-framework
        // https://stackoverflow.com/questions/25482687/how-to-execute-a-function-identified-by-pointer-from-lldb
        eval(format!("((void (*)(uintptr_t)){})({})", func, value)).context("define debug method")?;

        Ok(())
    }
}
