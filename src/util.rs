use std::io::Write;
use std::pin::Pin;
use crate::sys::lldb;


macro_rules! cstr {
    ( unsafe $value:expr ) => {
        unsafe {
            let value = $value;
            if !value.is_null() {
                Some(std::ffi::CStr::from_ptr(value))
            } else {
                None
            }
        }
    }
}


pub fn print_pretty_bytes(
    stdout: &mut dyn Write,
    base: u64,
    bytes: &[u8],
) -> anyhow::Result<()> {
    use std::fmt;

    struct HexPrinter<'a>(&'a [u8]);
    struct AsciiPrinter<'a>(&'a [u8]);

    impl fmt::Display for HexPrinter<'_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            for &b in self.0.iter() {
                write!(f, "{:02x} ", b)?;
            }

            for _ in self.0.len()..16 {
                write!(f, "   ")?;
            }

            Ok(())
        }
    }

    impl fmt::Display for AsciiPrinter<'_> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            use std::fmt::Write;

            for &b in self.0.iter() {
                let c = b as char;
                let c = if c.is_ascii_graphic() {
                    c
                } else {
                    '.'
                };
                f.write_char(c)?;
            }

            Ok(())
        }
    }

    let addr = base as *const u8;

    for (offset, chunk) in bytes.chunks(16).enumerate() {
        let addr = addr.wrapping_add(offset * 16);

        writeln!(
            stdout,
            "{:018p}: {} {}",
            addr,
            HexPrinter(chunk),
            AsciiPrinter(chunk)
        )?;
    }

    Ok(())
}


pub unsafe fn command_from_ptr<T: argh::FromArgs>(name: &str, command: *const *const u8) -> Result<T, String> {
    use std::ffi::CStr;

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

    argh::FromArgs::from_args(&[name], &args).map_err(|err| err.output)
}

pub fn read_memory<'a>(
    process: Pin<&mut lldb::SBProcess>,
    buf: &'a mut Vec<u8>,
    addr: u64,
    size: usize,
    mut error: Pin<&mut lldb::SBError>
) -> anyhow::Result<&'a [u8]> {
    use anyhow::Context;

    buf.clear();
    buf.try_reserve_exact(size).context("oom")?;
    error.as_mut().Clear();

    // # Safety
    //
    // read raw data from memory
    unsafe {
        let len = process.ReadMemory(
            addr,
            buf.as_mut_ptr().cast(),
            size,
            error.as_mut()
        );

        buf.set_len(len);
    }

    if error.Success() {
        anyhow::ensure!(!error.Fail(), "fail?");
        anyhow::ensure!(buf.len() == size, "short read?");

        Ok(buf.as_slice())
    } else {
        let err_msg = cstr!(unsafe error.GetCString());
        anyhow::bail!("read memory failed: {:?}", err_msg)
    }
}

pub fn u64ptr(value: &str) -> anyhow::Result<u64> {
    use anyhow::Context;

    let value = if let Some(value) = value.strip_prefix("0x") {
        let mut buf = [0; 8];
        let n = data_encoding::HEXLOWER_PERMISSIVE.decode_len(value.len())?;
        let n = buf.len().checked_sub(n).context("hex value is greater than 64bit")?;
        data_encoding::HEXLOWER_PERMISSIVE
            .decode_mut(value.as_bytes(), &mut buf[n..])
            .map_err(|err| anyhow::format_err!("hex decode failed: {:?}", err))?;
        u64::from_be_bytes(buf)
    } else {
        value.parse::<u64>().context("number parse failed")?
    };
    Ok(value)
}

#[test]
fn test_u64ptr_from_str() {
    assert_eq!(
        0x01,
        u64ptr_from_str("0x01").unwrap()
    );
    assert_eq!(
        0x000056257f77c380,
        u64ptr_from_str("0x000056257f77c380").unwrap()
    );
    assert_eq!(
        0x0056257f77c38000,
        u64ptr_from_str("0x0056257f77c38000").unwrap()
    );
}
