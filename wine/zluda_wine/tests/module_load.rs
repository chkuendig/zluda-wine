// SPDX-License-Identifier: MIT OR Apache-2.0
//! Native JIT ABI calls with independently typed backend functions.
use super::*;
use crate::cuda::{CUerror, CUjit_option, CUresult};

extern "C" {
    fn mmap(p: *mut c_void, n: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
    fn munmap(p: *mut c_void, n: usize) -> i32;
}
struct Memory(*mut u32);
impl Memory {
    unsafe fn new() -> Self {
        let p = mmap(std::ptr::null_mut(), 4096, 3, 2 | 0x20 | 0x40, -1, 0);
        assert!(!p.is_null() && (p as usize) < 0x80000000);
        Self(p.cast())
    }
    fn address(&self, word: usize) -> u32 { unsafe { self.0.add(word) as usize as u32 } }
}
impl Drop for Memory { fn drop(&mut self) { unsafe { assert_eq!(munmap(self.0.cast(), 4096), 0); } } }

unsafe extern "C" fn backend(module: *mut usize, image: *const c_void, count: u32,
    options: *mut CUjit_option, values: *mut *mut c_void) -> CUresult {
    assert_eq!(image as usize, 0xf1234567);
    if count == 0 {
        assert!(options.is_null() && values.is_null());
    } else {
        assert_eq!(count, 7);
        let keys = std::slice::from_raw_parts(options.cast::<u32>(), count as usize);
        let values = std::slice::from_raw_parts_mut(values, count as usize);
        assert_eq!(keys, [1, 2, 3, 4, 5, 6, 9]);
        assert_eq!(values[0] as usize, 256);
        assert_eq!(values[3] as usize, 16);
        assert_eq!(values[5] as usize, 16);
        assert_eq!(values[6] as usize, 0xfedcba98, "zero extension, not sign extension");
        values[1].cast::<f32>().write(12.5);
        std::ptr::copy_nonoverlapping(b"info\0".as_ptr(), values[2].cast(), 5);
        std::ptr::copy_nonoverlapping(b"error\0".as_ptr(), values[4].cast(), 6);
        values[0] = 192usize as *mut c_void;
        values[3] = 5usize as *mut c_void;
        values[5] = 6usize as *mut c_void;
    }
    *module = 0x123456789;
    Ok(())
}
unsafe fn resolve(id: u64) -> usize {
    assert_eq!(id, unixlib::function_id("cuModuleLoadDataEx"));
    backend as *const () as usize
}

#[test]
fn jit_values_widen_and_outputs_keep_pe32_stride() { unsafe {
    let memory = Memory::new();
    let options = [1u32, 2, 3, 4, 5, 6, 9];
    std::ptr::copy_nonoverlapping(options.as_ptr(), memory.0, options.len());
    let values = [256, memory.address(32), memory.address(40), 16, memory.address(48), 16, 0xfedcba98];
    memory.0.add(15).write(0xaabbccdd);
    std::ptr::copy_nonoverlapping(values.as_ptr(), memory.0.add(16), values.len());
    memory.0.add(23).write(0x11223344);
    let mut a = [0u32; 16];
    a[..5].copy_from_slice(&[1, 0xf1234567, 7, memory.address(0), memory.address(16)]);
    let mut module = 0;
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(load_data(&mut module, ptr(a[1]), &a, true), Ok(()));
    });
    assert_eq!(module, 0x123456789);
    let mut expected = values; expected[0] = 192; expected[3] = 5; expected[5] = 6;
    assert_eq!(std::slice::from_raw_parts(memory.0.add(16), 7), expected);
    assert_eq!(memory.0.add(15).read(), 0xaabbccdd);
    assert_eq!(memory.0.add(23).read(), 0x11223344);
    assert_eq!(memory.0.add(32).cast::<f32>().read(), 12.5);
    assert_eq!(CStr::from_ptr(memory.0.add(40).cast()).to_bytes(), b"info");
    assert_eq!(CStr::from_ptr(memory.0.add(48).cast()).to_bytes(), b"error");
}}

#[test]
fn zero_jit_options_pass_null_arrays() { unsafe {
    let mut a = [0u32; 16]; a[1] = 0xf1234567;
    let mut module = 0;
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(load_data(&mut module, ptr(a[1]), &a, true), Ok(()));
    });
    assert_eq!(module, 0x123456789);
}}

#[test]
fn ordinary_module_load_keeps_its_native_entrypoint() { unsafe {
    unsafe extern "C" fn load(module: *mut usize, image: *const c_void) -> CUresult {
        assert_eq!(image as usize, 0xf1234567); *module = 0x123456789; Ok(())
    }
    unsafe fn resolve(id: u64) -> usize {
        assert_eq!(id, unixlib::function_id("cuModuleLoadData")); load as *const () as usize
    }
    let mut module = 0;
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(load_data(&mut module, ptr(0xf1234567), &[0; 16], false), Ok(()));
    });
    assert_eq!(module, 0x123456789);
}}

#[test]
fn bad_jit_arrays_and_nested_pointer_options_stop_before_backend() { unsafe {
    let memory = Memory::new();
    let mut a = [0u32; 16]; a[2] = 1;
    let mut module = 0x11223344;
    assert_eq!(load_data(&mut module, ptr(1), &a, true), Err(INVALID));
    a[3] = memory.address(0);
    assert_eq!(load_data(&mut module, ptr(1), &a, true), Err(INVALID));
    a[4] = u32::MAX - 2;
    assert_eq!(load_data(&mut module, ptr(1), &a, true), Err(INVALID));
    a[4] = memory.address(16); memory.0.write(17);
    assert_eq!(load_data(&mut module, ptr(1), &a, true), Err(UNSUPPORTED));
    a[2] = 65;
    assert_eq!(load_data(&mut module, ptr(1), &a, true), Err(UNSUPPORTED));
    assert_eq!(module, 0x11223344);
}}

#[test]
fn jit_backend_error_is_preserved() { unsafe {
    unsafe extern "C" fn fail(_: *mut usize, _: *const c_void, _: u32,
        _: *mut CUjit_option, _: *mut *mut c_void) -> CUresult { Err(CUerror::NO_BINARY_FOR_GPU) }
    unsafe fn resolve(_: u64) -> usize { fail as *const () as usize }
    let mut module = 0x11223344;
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(load_data(&mut module, ptr(1), &[0; 16], true), Err(209));
    });
    assert_eq!(module, 0x11223344);
}}
