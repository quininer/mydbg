use std::io;
use std::pin::Pin;
use argh::FromArgs;
use anyhow::Context;
use autocxx::moveit::moveit;
use crate::sys::lldb;
use crate::util::print_pretty_bytes;


/// MyDbg Read command
#[derive(FromArgs)]
pub struct Command {
    /// read address
    #[argh(positional)]
    address: String,

    /// read size, default 64
    #[argh(option, short = 's')]
    size: Option<usize>,
}

impl Command {
    pub fn execute(self, debugger: Pin<&mut lldb::SBDebugger>) -> anyhow::Result<()> {
        let addr = if let Some(value) = self.address.strip_prefix("0x") {
            let value = data_encoding::HEXLOWER_PERMISSIVE
                .decode(value.as_bytes())
                .context("invalid hex value")?;
            let value: [u8; 8] = value.try_into().ok().context("value length does not meet 64bit")?;
            u64::from_be_bytes(value)
        } else {
            self.address.parse::<u64>().context("invalid u64 value")?
        };
        let size = self.size.unwrap_or(64);

        moveit!{
            let mut target = debugger.GetSelectedTarget();
            let mut process = target.as_mut().GetProcess();
            let mut error = lldb::SBError::new();
        }

        let mut buf: Vec<u8> = Vec::new();

        // # Safety
        //
        // read raw data from memory
        unsafe {
            buf.reserve_exact(size);

            let len = process.as_mut().ReadMemory(
                addr,
                buf.as_mut_ptr().cast(),
                size,
                error.as_mut()
            );

            buf.set_len(len);
        }

        let mut stdout = io::stdout().lock();

        print_pretty_bytes(&mut stdout, addr, &buf)?;

        Ok(())
    }
}
