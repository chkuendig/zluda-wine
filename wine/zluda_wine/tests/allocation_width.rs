// SPDX-License-Identifier: MIT OR Apache-2.0
use super::*;
extern "C" {
    fn mmap(p: *mut c_void, n: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
    fn munmap(p: *mut c_void, n: usize) -> i32;
}

#[test]
fn allocation_requires_four_byte_pointer_width_before_allocating() { unsafe {
    let out = mmap(std::ptr::null_mut(),4096,3,2|0x20|0x40,-1,0).cast::<u32>();
    assert!(!out.is_null() && (out as usize) < 0x80000000);
    let mut state = State::default();state.active = Some(0);state.contexts[0].pool = 0x1_0000_0000;
    let mut args = [0;16];args[0] = out as usize as u32;args[1] = 767;out.write(0xdeadbeef);
    for width in [0,1,2,8] {
        args[2] = width;
        assert_eq!(state.dispatch_current(C32_ALLOC,&args),Err(INVALID));
        assert_eq!(out.read(),0xdeadbeef);
        assert!(state.contexts[0].ranges.iter().all(|r|r.bytes==0));
    }
    args[2] = 4;args[0] = 0;
    assert_eq!(state.dispatch_current(C32_ALLOC,&args),Err(INVALID));
    args[0] = out as usize as u32;
    assert_eq!(state.dispatch_current(C32_ALLOC,&args),Ok(()));
    assert_eq!(out.read(),256);assert_eq!(state.contexts[0].ranges[0].bytes,767);
    assert_eq!(munmap(out.cast(),4096),0);
}}
