use std::path::PathBuf;


fn main() -> anyhow::Result<()> {
    let path = "src/sys.rs";

    let mut b = autocxx_build::Builder::new(
        path,
        &[PathBuf::from("/usr/include/lldb/API/")],
    )
    .build()?;
    b.flag_if_supported("-std=c++14").compile("lldb");

    println!("cargo:rerun-if-changed={}", path);
    Ok(())
}
