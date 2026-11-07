// SPDX-License-Identifier: MIT OR Apache-2.0
// Independent opt-in Wine CUDA frontend; see IMPLEMENTATION-NOTES.md.


#[cfg(not(any(target_arch = "x86_64", all(windows, target_arch = "x86"))))]
compile_error!("zluda_wine supports PE64, PE32 and a single ELF64 Unix library");
#[cfg(not(any(target_os = "windows", target_os = "linux")))]
compile_error!("zluda_wine supports Windows x64 PE and Linux ELF64 only");

extern crate self as cuda_types;
pub enum FILE {}
include!(concat!(env!("OUT_DIR"), "/cuda_types.rs"));
#[cfg(target_arch = "x86_64")]
include!(concat!(env!("OUT_DIR"), "/dark_types.rs"));

#[cfg(target_arch = "x86_64")]
mod unixlib;
mod legacy;
#[cfg(target_arch = "x86_64")] mod luid_protocol;
#[cfg(target_arch = "x86_64")]
mod proc_address;
#[cfg(target_arch = "x86_64")]
mod dark_protocol;
#[cfg(target_arch = "x86_64")]
mod integrity_algorithm { include!(concat!(env!("OUT_DIR"), "/integrity_algorithm.rs")); }
#[cfg(target_arch = "x86_64")] mod callback_protocol;
#[cfg(target_arch = "x86_64")] mod context_protocol;
#[cfg(target_os = "linux")] mod unix_callbacks;
#[cfg(target_os = "linux")] mod unix_callback_api;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_callback_api;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_callbacks;
#[cfg(target_os = "linux")] mod dark_unix;
#[cfg(all(windows, target_arch = "x86_64"))] mod dark_pe_thunks;
// Windows entry points are compiled for their respective PE targets.
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_thunks;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_wrappers;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_platform;
#[cfg(windows)] mod pe_sync;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_luid;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_external;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_context_storage;
#[cfg(target_os = "linux")] mod unix_context_storage;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_dark_owned;
#[cfg(all(windows, target_arch = "x86_64"))] mod pe_dark_tables;
#[cfg(target_os = "linux")] mod unix_thunks;
#[cfg(target_os = "linux")] mod unix_wrappers;
mod wow64_protocol;
mod wow64_trace;
#[cfg(any(test, all(windows, target_arch = "x86")))] mod d3d10_copy;
#[cfg(target_os = "linux")] mod wow64_unix;
#[cfg(all(windows, target_arch = "x86"))] mod wow64_pe;
#[cfg(all(windows, target_arch = "x86"))] mod wow64_d3d10;
#[cfg(all(windows, target_arch = "x86"))] mod wow64_local;
#[cfg(all(windows, target_arch = "x86"))]
#[allow(dead_code)]
mod wow64_stubs { include!(concat!(env!("OUT_DIR"), "/wow64_stubs.rs")); }
