// SPDX-License-Identifier: MIT OR Apache-2.0
//! Legacy CUDA parameter buffers belong to the virtual PE32 function handle.
use super::*;
const PARAM_BYTES: usize = 4096;

pub(super) struct LegacyLaunch { block: [u32; 3], shared: u32, bytes: usize, parameters: Vec<u8> }
impl Default for LegacyLaunch {
    fn default() -> Self { Self { block: [1; 3], shared: 0, bytes: 0, parameters: Vec::new() } }
}
impl LegacyLaunch {
    fn size(&mut self, bytes: u32) -> Answer {
        if bytes as usize > PARAM_BYTES { return Err(INVALID); }
        self.parameters.resize(self.parameters.len().max(bytes as usize), 0);
        self.bytes = bytes as usize;
        Ok(())
    }
    fn write(&mut self, offset: u32, data: &[u8]) -> Answer {
        let end = (offset as usize).checked_add(data.len()).filter(|&n| n <= PARAM_BYTES).ok_or(INVALID)?;
        self.parameters.resize(self.parameters.len().max(end), 0);
        self.parameters[offset as usize..end].copy_from_slice(data);
        Ok(())
    }
    fn arguments(&self, layout: &[ArgInfo], params: &mut [usize]) -> Answer {
        let mut offset = 0usize;
        for (arg, param) in layout.iter().zip(params) {
            if arg.align == 0 || !arg.align.is_power_of_two() || arg.align > 256 { return Err(INVALID); }
            offset = (offset + arg.align as usize - 1) & !(arg.align as usize - 1);
            let end = offset.checked_add(arg.size as usize).filter(|&n| n <= self.bytes).ok_or(INVALID)?;
            *param = self.parameters.as_ptr() as usize + offset;
            offset = end;
        }
        Ok(())
    }
}
impl State {
    pub(super) unsafe fn dispatch_legacy(&mut self, op: u32, a: &[u32; 16]) -> Answer {
        let h = self.handle(a[0], 3).ok_or(INVALID_HANDLE)?;
        match op {
            C32_LEGACY_CONFIG => match a[1] {
                0 if a[2..5].iter().all(|&n| n != 0 && n <= i32::MAX as u32) => {
                    self.handles[h].legacy.block.copy_from_slice(&a[2..5]); Ok(())
                }
                1 => { self.handles[h].legacy.shared = a[2]; Ok(()) }
                _ => Err(INVALID),
            },
            C32_LEGACY_PARAM => match a[1] {
                0 => self.handles[h].legacy.size(a[2]),
                1 => self.handles[h].legacy.write(a[2], &a[3].to_ne_bytes()),
                2 if a[4] as usize <= PARAM_BYTES && (a[4] == 0 ||
                    a[3] != 0 && a[3] as u64 + a[4] as u64 <= 1 << 32) => {
                    let bytes = if a[4] == 0 { &[] } else { std::slice::from_raw_parts(ptr(a[3]), a[4] as usize) };
                    self.handles[h].legacy.write(a[2], bytes)
                }
                _ => Err(INVALID),
            },
            C32_LEGACY_LAUNCH => {
                if a[1] == 0 || a[2] == 0 || a[1] > i32::MAX as u32 || a[2] > i32::MAX as u32 {
                    return Err(INVALID);
                }
                let stream = self.stream(a[3])?;
                let legacy = &self.handles[h].legacy;
                let nargs = self.handles[h].nargs as usize;
                let mut params = [0usize; MAX_ARGS + 1 + MAX_GLOBALS];
                legacy.arguments(&self.handles[h].args[..nargs], &mut params[..nargs])?;
                self.launch_parameters(h, &[a[1], a[2], 1, legacy.block[0], legacy.block[1],
                    legacy.block[2], legacy.shared], stream, &mut params)
            }
            C32_FUNC_ATTRIBUTE => {
                if a[1] == 0 { return Err(INVALID); }
                native("cuFuncGetAttribute", &[a[1] as u64, a[2] as u64, self.handles[h].native as u64])
            }
            C32_PARAM_TEXREF => {
                let tex = self.handle(a[2], 5).ok_or(INVALID_HANDLE)?;
                if a[1] != u32::MAX || self.handles[tex].owner != self.handles[h].owner { return Err(INVALID); }
                Ok(()) // The module's texref is already bound through cuTexRefSet*.
            }
            _ => Err(UNSUPPORTED),
        }
    }
}

#[cfg(test)] #[path = "../tests/legacy_launch.rs"] mod tests;
