// SPDX-License-Identifier: MIT OR Apache-2.0
//! Linear layouts for single-subresource legacy D3D10 pointer registrations.
use crate::wow64_protocol::{INVALID, UNSUPPORTED};
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Layout { pub width: u32, pub height: u32, pub row: u32, pub pitch: u32, pub bytes: u32, pub texture: bool }
pub fn align(value: u32, alignment: u32) -> Result<u32,i32> {
    if !alignment.is_power_of_two() { return Err(INVALID); }
    value.checked_add(alignment-1).map(|n| n & !(alignment-1)).ok_or(INVALID)
}
impl Layout {
    pub fn texture(width: u32, height: u32, format: u32, alignment: u32) -> Result<Self,i32> {
        let pixel = match format {
            2..=4 => 16, 10..=14|16..=18 => 8,
            24..=32|34..=38|41..=43|87|88 => 4, 49..=52|54..=59 => 2, 61..=65 => 1,
            _ => return Err(UNSUPPORTED),
        };
        if width == 0 || height == 0 { return Err(INVALID); }
        let row = width.checked_mul(pixel).ok_or(INVALID)?;
        let pitch = align(row,alignment)?;
        let bytes = pitch.checked_mul(height).filter(|&n| n <= 512*1024*1024-256).ok_or(INVALID)?;
        Ok(Self { width,height,row,pitch,bytes,texture:true })
    }
    pub fn buffer(bytes: u32) -> Result<Self,i32> {
        if bytes == 0 || bytes > 512*1024*1024-256 { return Err(INVALID); }
        Ok(Self { width:bytes,height:1,row:bytes,pitch:bytes,bytes,texture:false })
    }
    pub unsafe fn copy(self, mapped: *mut u8, mapped_pitch: usize, host: &mut [u8], input: bool) -> Result<(),i32> {
        let pitch = if self.texture { mapped_pitch } else { self.row as usize };
        if mapped.is_null() || pitch < self.row as usize || host.len() < self.bytes as usize ||
            pitch.checked_mul((self.height-1) as usize).and_then(|n| n.checked_add(self.row as usize)).is_none() {
            return Err(INVALID);
        }
        for y in 0..self.height as usize {
            let a = mapped.add(y*pitch);let b = host.as_mut_ptr().add(y*self.pitch as usize);
            let (src,dst) = if input { (a,b) } else { (b,a) };
            std::ptr::copy_nonoverlapping(src,dst,self.row as usize);
        }
        Ok(())
    }
}
pub fn input(flags: u32) -> bool { flags != 2 }
pub fn output(flags: u32) -> bool { flags != 1 }

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn layouts_keep_pixel_rows_and_aligned_pitches() {
        for (format,pixel) in [(2,16),(10,8),(28,4),(41,4),(54,2),(61,1)] {
            let l=Layout::texture(3,2,format,256).unwrap();
            assert_eq!((l.row,l.pitch,l.bytes),(3*pixel,256,512));
        }
        assert_eq!(Layout::texture(3,2,71,256),Err(UNSUPPORTED));
        assert!(Layout::texture(u32::MAX,2,2,256).is_err());
        assert!(Layout::texture(1,u32::MAX,2,256).is_err());
        assert!(Layout::texture(0,2,2,256).is_err());
        assert!(Layout::texture(1,2,2,3).is_err());
        assert_eq!(Layout::buffer(17).unwrap().bytes,17);
    }
    #[test]
    fn each_map_refreshes_input_and_each_unmap_preserves_padding() { unsafe {
        let l=Layout::texture(2,2,61,4).unwrap();
        let mut graphics=[1,2,0xa5,0xa5,3,4,0xa5,0xa5];let mut host=[0xcc;8];
        l.copy(graphics.as_mut_ptr(),4,&mut host,true).unwrap();
        assert_eq!(host,[1,2,0xcc,0xcc,3,4,0xcc,0xcc]);
        graphics[0]=9;
        l.copy(graphics.as_mut_ptr(),4,&mut host,true).unwrap();assert_eq!(host[0],9);
        host[1]=8;host[5]=7;
        l.copy(graphics.as_mut_ptr(),4,&mut host,false).unwrap();
        assert_eq!(graphics,[9,8,0xa5,0xa5,3,7,0xa5,0xa5]);
        assert!(l.copy(graphics.as_mut_ptr(),1,&mut host,true).is_err());
        assert!(l.copy(std::ptr::null_mut(),4,&mut host,true).is_err());
    }}
    #[test]
    fn map_flags_do_not_reuse_primed_pixels() {
        assert!(input(0)&&output(0));assert!(input(1)&&!output(1));assert!(!input(2)&&output(2));
        assert_eq!(align(257,256),Ok(512));assert!(align(u32::MAX,256).is_err());
    }
}
