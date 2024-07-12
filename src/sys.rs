autocxx::include_cpp! {
    #include "lldb/API/LLDB.h"
    generate!("lldb::SBDebugger")
    generate!("lldb::SBTarget")
    generate!("lldb::SBProcess")
    generate!("lldb::SBThread")
    generate!("lldb::SBFrame")
    generate!("lldb::SBSymbol")
    generate!("lldb::SBFunction")
    generate!("lldb::SBAddress")
    generate!("lldb::SBValue")
    generate!("lldb::SBValueList")
    generate!("lldb::SBData")
    generate!("lldb::SBError")
    generate!("lldb::SBMemoryRegionInfoList")
    generate!("lldb::SBMemoryRegionInfo")
    generate!("lldb::SBSymbolContext")
    generate!("lldb::SBSymbolContextList")
    safety!(unsafe_ffi)
}

pub use ffi::*;
