autocxx::include_cpp! {
    #include "lldb/API/LLDB.h"
    generate!("lldb::SBDebugger")
    generate!("lldb::SBTarget")
    generate!("lldb::SBProcess")
    generate!("lldb::SBThread")
    generate!("lldb::SBFrame")
    generate!("lldb::SBSymbol")
    generate!("lldb::SBValueList")
    generate!("lldb::SBValue")
    generate!("lldb::SBData")
    generate!("lldb::SBError")
    generate!("lldb::SBType")
    generate!("lldb::SBTypeFormat")
    generate!("lldb::Format")
    safety!(unsafe_ffi)
}

pub use ffi::*;
