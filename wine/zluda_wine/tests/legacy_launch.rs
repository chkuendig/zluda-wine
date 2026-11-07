// SPDX-License-Identifier: MIT OR Apache-2.0
//! Legacy buffers are checked independently of the PE32 packet generator.
use super::*;
use crate::cuda::CUresult;

fn state() -> (State, usize, usize) {
    let mut state = State::default();
    state.active = Some(0);
    state.contexts[0].id = 0x40000010;
    state.contexts[0].pool = 0x1_0000_0000;
    let module = state.new_handle(0x2000, 2).unwrap();
    state.handles[module].globals.push(Global { name: CString::new("global").unwrap(), offset: 256, bytes: 4 });
    let function = state.new_handle(0x3000, 3).unwrap();
    state.handles[function].owner = state.handles[module].id;
    state.handles[function].nargs = 2;
    state.handles[function].args[..2].copy_from_slice(&[ArgInfo { size: 4, align: 4 }, ArgInfo { size: 8, align: 8 }]);
    (state, module, function)
}
fn args(function: u32, rest: &[u32]) -> [u32; 16] {
    let mut a = [0; 16]; a[0] = function; a[1..1+rest.len()].copy_from_slice(rest); a
}

#[test]
fn parameters_keep_alignment_gaps_and_reject_out_of_bounds_writes() {
    let mut legacy = LegacyLaunch::default();
    legacy.write(0, &0xfedcba98u32.to_ne_bytes()).unwrap();
    legacy.write(8, &3.5f64.to_ne_bytes()).unwrap();
    legacy.size(16).unwrap();
    let layout = [ArgInfo { size: 4, align: 4 }, ArgInfo { size: 8, align: 8 }];
    let mut params = [0; 2];
    legacy.arguments(&layout, &mut params).unwrap();
    unsafe { assert_eq!(std::ptr::read_unaligned(params[0] as *const u32), 0xfedcba98);
        assert_eq!(std::ptr::read_unaligned(params[1] as *const f64), 3.5); }
    assert_eq!(&legacy.parameters[4..8], &[0; 4]);
    let saved = legacy.parameters.clone();
    assert_eq!(legacy.write(u32::MAX, &[0]), Err(INVALID));
    assert_eq!(legacy.write(4095, &[0; 2]), Err(INVALID));
    assert_eq!(legacy.size(4097), Err(INVALID));
    assert_eq!(legacy.parameters, saved);
    legacy.size(15).unwrap();
    assert_eq!(legacy.arguments(&layout, &mut params), Err(INVALID));
    assert_eq!(legacy.arguments(&[ArgInfo { size: 4, align: 3 }], &mut params[..1]), Err(INVALID));
}

unsafe extern "C" fn launch(function: usize, gx: u32, gy: u32, gz: u32,
    bx: u32, by: u32, bz: u32, shared: u32, stream: usize, params: *const *const c_void,
    extra: *const *const c_void) -> CUresult {
    assert_eq!(function, 0x3000);
    assert_eq!([gx,gy,gz,bx,by,bz,shared], [2,3,1,8,4,1,128]);
    assert_eq!(stream, 0x6000); assert!(extra.is_null());
    assert_eq!(std::ptr::read_unaligned((*params).cast::<u32>()), 0xfedcba98);
    assert_eq!(std::ptr::read_unaligned((*params.add(1)).cast::<f64>()), 3.5);
    assert_eq!(*(*params.add(2)).cast::<u64>(), 0x1_0000_0000);
    assert_eq!(*(*params.add(3)).cast::<u32>(), 256);
    Ok(())
}

#[test]
fn legacy_launch_preserves_dimensions_stream_pool_and_module_globals() { unsafe {
    let (mut state, _, h) = state(); let id = state.handles[h].id;
    let stream = state.new_handle(0x6000, 6).unwrap();
    state.dispatch_legacy(C32_LEGACY_CONFIG, &args(id, &[0,8,4,1])).unwrap();
    state.dispatch_legacy(C32_LEGACY_CONFIG, &args(id, &[1,128])).unwrap();
    state.dispatch_legacy(C32_LEGACY_PARAM, &args(id, &[1,0,0xfedcba98])).unwrap();
    state.handles[h].legacy.write(8, &3.5f64.to_ne_bytes()).unwrap();
    state.dispatch_legacy(C32_LEGACY_PARAM, &args(id, &[0,16])).unwrap();
    unsafe fn resolve(id: u64) -> usize {
        assert_eq!(id, unixlib::function_id("cuLaunchKernel")); launch as *const () as usize
    }
    let request = args(id, &[2,3,state.handles[stream].id]);
    crate::unix_thunks::with_symbols(resolve, || {
        assert_eq!(state.dispatch_legacy(C32_LEGACY_LAUNCH, &request), Ok(()));
    });
}}

#[test]
fn legacy_operations_reject_bad_handles_dimensions_and_buffers_before_launch() { unsafe {
    let (mut state, _, h) = state(); let id = state.handles[h].id;
    assert_eq!(state.dispatch_legacy(C32_LEGACY_CONFIG, &args(0, &[0,1,1,1])), Err(INVALID_HANDLE));
    for dims in [[0,1,1],[1,u32::MAX,1],[1,1,0]] {
        assert_eq!(state.dispatch_legacy(C32_LEGACY_CONFIG, &args(id, &[0,dims[0],dims[1],dims[2]])), Err(INVALID));
    }
    assert_eq!(state.dispatch_legacy(C32_LEGACY_PARAM, &args(id, &[1,u32::MAX,0])), Err(INVALID));
    assert_eq!(state.dispatch_legacy(C32_LEGACY_PARAM, &args(id, &[2,0,0,4])), Err(INVALID));
    assert_eq!(state.dispatch_legacy(C32_LEGACY_PARAM, &args(id, &[2,0,u32::MAX,4])), Err(INVALID));
    assert_eq!(state.dispatch_legacy(C32_LEGACY_LAUNCH, &args(id, &[0,1,0])), Err(INVALID));
    assert_eq!(state.dispatch_legacy(C32_LEGACY_LAUNCH, &args(id, &[1,1,42])), Err(INVALID_HANDLE));
    assert_eq!(state.dispatch_legacy(C32_LEGACY_LAUNCH, &args(id, &[1,1,0])), Err(INVALID));
}}

#[test]
fn param_texref_requires_the_functions_module_and_default_unit() { unsafe {
    let (mut state, module, h) = state(); let id = state.handles[h].id;
    let tex = state.new_handle(0x5000, 5).unwrap();
    let texture = state.handles[tex].id;
    assert_eq!(state.dispatch_legacy(C32_PARAM_TEXREF, &args(id, &[u32::MAX,texture])), Err(INVALID));
    state.handles[tex].owner = state.handles[module].id;
    assert_eq!(state.dispatch_legacy(C32_PARAM_TEXREF, &args(id, &[0,texture])), Err(INVALID));
    assert_eq!(state.dispatch_legacy(C32_PARAM_TEXREF, &args(id, &[u32::MAX,texture])), Ok(()));
    state.handles[h] = Handle::default();
    assert_eq!(state.dispatch_legacy(C32_PARAM_TEXREF, &args(id, &[u32::MAX,texture])), Err(INVALID_HANDLE));
}}
