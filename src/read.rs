use std::io::{ self, Write };
use std::pin::Pin;
use std::path::PathBuf;
use argh::FromArgs;
use anyhow::Context;
use autocxx::moveit::moveit;
use crate::sys::lldb;
use crate::util::{ print_pretty_bytes, read_memory };


/// MyDbg Read command
#[derive(FromArgs)]
pub struct Command {
    /// read address
    #[argh(positional)]
    address: String,

    /// read size, default 64
    #[argh(option, short = 's')]
    size: Option<usize>,

    /// read bytes to output file
    #[argh(option, short = 'o')]
    output: Option<PathBuf>
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

        if let Some(path) = self.output {
            let mut output = std::fs::File::create(&path)?;

            const CHUNK_SIZE: usize = 16 * 1024;

            for offset in (0..size).step_by(CHUNK_SIZE) {
                let addr = addr + offset as u64;
                let size = std::cmp::min(size - offset, CHUNK_SIZE);

                let buf = read_memory(
                    process.as_mut(),
                    &mut buf,
                    addr + offset as u64,
                    size,
                    error.as_mut()
                ).with_context(|| format!("addr={:p},size={}", addr as *const u8, size))?;

                output.write_all(buf)?;
            }

            output.flush()?;
        } else {
            let buf = read_memory(
                process.as_mut(),
                &mut buf,
                addr,
                size,
                error.as_mut()
            )?;

            let mut stdout = io::stdout().lock();
            print_pretty_bytes(&mut stdout, addr, buf)?;
            stdout.flush()?;
        }

        Ok(())
    }
}
