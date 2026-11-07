// SPDX-License-Identifier: MIT OR Apache-2.0
use super::*;
use crate::cuda::CUresult;
extern "C" {
    fn mmap(p: *mut c_void, n: usize, prot: i32, flags: i32, fd: i32, offset: i64) -> *mut c_void;
    fn munmap(p: *mut c_void, n: usize) -> i32;
}
struct Fixture { state: State, desc: *mut u32, args: [u32; 16] }
impl Fixture {
    unsafe fn new() -> Self {
        let desc = mmap(std::ptr::null_mut(),4096,3,2|0x20|0x40,-1,0).cast::<u32>();
        assert!(!desc.is_null() && (desc as usize) < 0x80000000);
        let mut state = State::default(); state.active = Some(0);
        state.contexts[0].id = 1; state.contexts[0].pool = 0x1_0000_0000;
        state.contexts[0].ranges[0] = Allocation { offset: 256, bytes: 512, reserved: 512, owner: 0 };
        let tex = state.new_handle(0x5000,5).unwrap();
        for (i, n) in [3,2,0x20,1].into_iter().enumerate() { desc.add(i).write(n); }
        let mut args = [0;16];args[..4].copy_from_slice(&[state.handles[tex].id,desc as usize as u32,256,256]);
        Self { state, desc, args }
    }
}
impl Drop for Fixture { fn drop(&mut self) { unsafe { assert_eq!(munmap(self.desc.cast(),4096),0); } } }
unsafe extern "C" fn bind(tex: usize, desc: *const ArrayDescriptor, device: u64, pitch: usize) -> CUresult {
    assert_eq!(tex,0x5000); let d = &*desc;
    assert_eq!([d.width,d.height,d.format as usize,d.channels as usize],[3,2,0x20,1]);
    assert_eq!(device,0x1_0000_0100);assert_eq!(pitch,256);
    Ok(())
}
#[test]
fn pitched_texref_widens_descriptor_and_translates_device_offset() { unsafe {
    let mut f = Fixture::new();
    unsafe fn resolve(id: u64) -> usize {
        assert_eq!(id,unixlib::function_id("cuTexRefSetAddress2D_v3"));bind as *const () as usize
    }
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(f.state.dispatch_current(C32_TEX_ADDRESS_2D,&f.args),Ok(()));
    });
}}
#[test]
fn pitched_texref_rejects_bad_shapes_and_extents_before_backend() { unsafe {
    let mut f = Fixture::new();
    for (index,value) in [(1,0),(1,u32::MAX-8),(2,768),(3,11),(3,u32::MAX)] {
        let mut a = f.args;a[index]=value;
        assert_eq!(f.state.dispatch_current(C32_TEX_ADDRESS_2D,&a),Err(INVALID));
    }
    for (index,value) in [(0,0),(1,u32::MAX),(2,0x21),(3,3)] {
        let old = f.desc.add(index).read();f.desc.add(index).write(value);
        assert_eq!(f.state.dispatch_current(C32_TEX_ADDRESS_2D,&f.args),Err(INVALID));
        f.desc.add(index).write(old);
    }
}}
