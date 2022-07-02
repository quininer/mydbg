use std::io::Write;


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
            for &b in self.0.iter() {
                let c = b as char;
                if !c.is_ascii_control() {
                    write!(f, "{}", c)?;
                } else {
                    write!(f, ".")?;
                }
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
