#![allow(unused_imports, improper_ctypes, unnecessary_transmutes)]

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
    generate!("lldb::SBMemoryRegionInfoList")
    generate!("lldb::SBMemoryRegionInfo")
    generate!("lldb::SBBreakpoint")
    generate!("lldb::SBBroadcaster")
    generate!("lldb::SBListener")
    generate!("lldb::SBEvent")
    safety!(unsafe_ffi)
}

pub use ffi::*;
