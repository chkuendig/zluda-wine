// SPDX-License-Identifier: MIT OR Apache-2.0
//! PE32 legacy D3D10 pointer exchange through DXVK's D3D11 staging objects.
use crate::{d3d10_copy::{self,Layout},pe_sync::Mutex,wow64_pe::{call,traced},wow64_protocol::*};
use std::ffi::c_void;
type Answer<T=()> = Result<T,i32>;
const UNKNOWN:i32=999;
const ALREADY_MAPPED:i32=208;
const NOT_MAPPED:i32=211;
extern "system" {fn GetCurrentThreadId()->u32;fn GetTickCount64()->u64;fn Sleep(ms:u32);}

pub(super) mod com {
    use super::*;
    #[derive(PartialEq,Eq)] #[repr(C)] pub struct Guid {pub a:u32,pub b:u16,pub c:u16,pub d:[u8;8]}
    pub const IDENTITY:Guid=Guid{a:0,b:0,c:0,d:[0xc0,0,0,0,0,0,0,0x46]};
    pub const DEVICE:Guid=Guid{a:0xdb6f6ddb,b:0xac77,c:0x4e88,d:[0x82,0x53,0x81,0x9d,0xf9,0xbb,0xf1,0x40]};
    pub const TEXTURE:Guid=Guid{a:0x6f15aaf2,b:0xd208,c:0x4e89,d:[0x9a,0xb4,0x48,0x95,0x35,0xd3,0x4f,0x9c]};
    pub const BUFFER:Guid=Guid{a:0x48570b85,b:0xd1ee,c:0x4fcd,d:[0xa2,0x50,0xeb,0x35,0x07,0x22,0xb0,0x37]};
    pub const MULTITHREAD:Guid=Guid{a:0x9b7e4e00,b:0x342c,c:0x4106,d:[0xa1,0x9f,0x4f,0x27,0x04,0xf6,0x89,0xf0]};
    #[derive(Default)] #[repr(C)] pub struct Texture {pub width:u32,pub height:u32,pub mips:u32,pub arrays:u32,pub format:u32,pub samples:u32,pub quality:u32,pub usage:u32,pub bind:u32,pub cpu:u32,pub misc:u32}
    #[derive(Default)] #[repr(C)] pub struct Buffer {pub bytes:u32,pub usage:u32,pub bind:u32,pub cpu:u32,pub misc:u32,pub stride:u32}
    #[repr(C)] pub struct Mapped {pub data:*mut u8,pub row:u32,pub depth:u32}
    impl Default for Mapped {fn default()->Self {Self{data:std::ptr::null_mut(),row:0,depth:0}}}
    unsafe fn slot(object:usize,index:usize)->*const c_void {(*(*(object as *const *const usize)).add(index)) as *const c_void}
    macro_rules! invoke {($o:expr,$s:expr,($($t:ty),*)->$r:ty,$($a:expr),*)=>{{
        let f:unsafe extern "system" fn(usize,$($t),*)->$r=std::mem::transmute(slot($o,$s));f($o,$($a),*)
    }};}
    pub unsafe fn query(object:usize,iid:&Guid)->Answer<Owned> {
        if object==0{return Err(INVALID);}let mut out=0;
        let hr=invoke!(object,0,(*const Guid,*mut usize)->i32,iid,&mut out);
        if hr<0||out==0{Err(UNSUPPORTED)}else{Ok(Owned(out))}
    }
    pub struct Owned(pub usize);
    impl Drop for Owned {fn drop(&mut self){if self.0!=0{unsafe{invoke!(self.0,2,()->u32,);}}}}
    pub unsafe fn device(object:usize)->Answer<Owned>{let mut out=0;invoke!(object,3,(*mut usize)->(),&mut out);if out==0{Err(UNKNOWN)}else{Ok(Owned(out))}}
    pub unsafe fn texture_desc(object:usize)->Texture{let mut d=Texture::default();invoke!(object,10,(*mut Texture)->(),&mut d);d}
    pub unsafe fn buffer_desc(object:usize)->Buffer{let mut d=Buffer::default();invoke!(object,10,(*mut Buffer)->(),&mut d);d}
    pub unsafe fn flags(device:usize)->u32{invoke!(device,38,()->u32,)}
    pub unsafe fn immediate(device:usize)->Answer<Owned>{let mut out=0;invoke!(device,40,(*mut usize)->(),&mut out);if out==0{Err(UNKNOWN)}else{Ok(Owned(out))}}
    pub unsafe fn staging(device:usize,source:usize,texture:bool)->Answer<Owned>{
        let mut out=0;
        let hr=if texture {let mut d=texture_desc(source);d.usage=3;d.bind=0;d.cpu=0x30000;d.misc=0;
            invoke!(device,5,(*const Texture,*const c_void,*mut usize)->i32,&d,std::ptr::null(),&mut out)
        }else{let mut d=buffer_desc(source);d.usage=3;d.bind=0;d.cpu=0x30000;d.misc=0;
            invoke!(device,3,(*const Buffer,*const c_void,*mut usize)->i32,&d,std::ptr::null(),&mut out)};
        if hr<0||out==0{Err(UNKNOWN)}else{Ok(Owned(out))}
    }
    pub unsafe fn ready(device:usize)->Answer<Owned>{let desc=[0u32;2];let mut out=0;
        let hr=invoke!(device,24,(*const u32,*mut usize)->i32,desc.as_ptr(),&mut out);
        if hr<0||out==0{Err(UNKNOWN)}else{Ok(Owned(out))}}
    pub unsafe fn protect(object:usize)->Answer{invoke!(object,5,(i32)->i32,1);if invoke!(object,6,()->i32,)==0{Err(UNSUPPORTED)}else{Ok(())}}
    pub unsafe fn enter(object:usize){if object!=0{invoke!(object,3,()->(),);}}
    pub unsafe fn leave(object:usize){if object!=0{invoke!(object,4,()->(),);}}
    pub unsafe fn copy(context:usize,dst:usize,src:usize){invoke!(context,47,(usize,usize)->(),dst,src)}
    pub unsafe fn map(context:usize,resource:usize,input:bool,out:&mut Mapped)->i32{invoke!(context,14,(usize,u32,u32,u32,*mut Mapped)->i32,resource,0,if input{1}else{2},0x100000,out)}
    pub unsafe fn unmap(context:usize,resource:usize){invoke!(context,15,(usize,u32)->(),resource,0)}
    pub unsafe fn end(context:usize,query:usize){invoke!(context,28,(usize)->(),query)}
    pub unsafe fn flush(context:usize){invoke!(context,111,()->(),)}
    pub unsafe fn data(context:usize,query:usize)->i32{invoke!(context,29,(usize,*mut c_void,u32,u32)->i32,query,std::ptr::null_mut(),0,1)}
    const _:()={assert!(std::mem::size_of::<Guid>()==16);assert!(std::mem::size_of::<Texture>()==44);
        assert!(std::mem::size_of::<Buffer>()==24);assert!(std::mem::size_of::<Mapped>()==12);};
}
struct Binding {ctx:u32,device:com::Owned,thread:u32}
struct Resource {identity:com::Owned,source:com::Owned,staging:com::Owned,context:com::Owned,
    multithread:com::Owned,ready:com::Owned,ctx:u32,layout:Layout,allocation:u32,device:u32,
    host:Vec<u8>,flags:u32,mapped:bool,poisoned:bool}
struct Registry {owner:u32,bindings:Vec<Binding>,resources:Vec<Box<Resource>>}
static REGISTRY:Mutex<Registry>=Mutex::new(Registry{owner:0,bindings:Vec::new(),resources:Vec::new()});
struct Lease;
impl Lease {
    unsafe fn acquire()->Answer<Self>{let thread=GetCurrentThreadId();let start=GetTickCount64();loop{
        {let mut s=REGISTRY.lock();if s.owner==0{s.owner=thread;return Ok(Self);}if s.owner==thread{return Err(800);}}
        if GetTickCount64().wrapping_sub(start)>=30000{return Err(UNKNOWN);}Sleep(1);
    }}
}
impl Drop for Lease {fn drop(&mut self){REGISTRY.lock().owner=0;}}
unsafe fn cuda(op:u32,args:&[u32])->Answer{match call(op,args){0=>Ok(()),n=>Err(n)}}
unsafe fn current()->Answer<u32>{let mut ctx=0u32;cuda(C32_CTX_CURRENT,&[pointer(&mut ctx)])?;if ctx==0{Err(INVALID_CONTEXT)}else{Ok(ctx)}}
fn binding(ctx:u32)->Answer<(usize,u32)>{let s=REGISTRY.lock();let b=s.bindings.iter().find(|b|b.ctx==ctx).ok_or(INVALID_CONTEXT)?;
    if b.thread!=0&&b.thread!=unsafe{GetCurrentThreadId()}{return Err(UNSUPPORTED);}Ok((b.device.0,b.thread))}
unsafe fn find(resource:usize,ctx:u32)->Answer<*mut Resource>{binding(ctx)?;let identity=com::query(resource,&com::IDENTITY)?;
    let s=REGISTRY.lock();let r=s.resources.iter().find(|r|r.identity.0==identity.0).ok_or(INVALID_HANDLE)?;
    if r.ctx!=ctx{return Err(INVALID_CONTEXT);}Ok((&**r as *const Resource).cast_mut())}
unsafe fn guarded<T>(r:&Resource,f:impl FnOnce()->T)->T{
    struct Exit(usize);impl Drop for Exit{fn drop(&mut self){unsafe{com::leave(self.0);}}}
    com::enter(r.multithread.0);let _exit=Exit(r.multithread.0);f()
}
unsafe fn wait(r:&Resource)->Answer{guarded(r,||{com::end(r.context.0,r.ready.0);com::flush(r.context.0);});
    let start=GetTickCount64();loop{let hr=guarded(r,||com::data(r.context.0,r.ready.0));
        if hr==0{return Ok(());}if hr!=1||GetTickCount64().wrapping_sub(start)>=30000{return Err(UNKNOWN);}Sleep(1);}}
unsafe fn transfer(r:&mut Resource,input:bool)->Answer{
    let mut mapped=com::Mapped::default();let start=GetTickCount64();
    loop{let hr=guarded(r,||com::map(r.context.0,r.staging.0,input,&mut mapped));
        if hr as u32!=0x887a000a{if hr<0{return Err(UNKNOWN);}break;}
        if GetTickCount64().wrapping_sub(start)>=30000{return Err(UNKNOWN);}Sleep(1);
    }
    let copy=r.layout.copy(mapped.data,mapped.row as usize,&mut r.host,input);
    guarded(r,||com::unmap(r.context.0,r.staging.0));copy?;
    if input{cuda(C32_HTOD,&[r.device,pointer(r.host.as_ptr()),r.layout.bytes])}else{Ok(())}
}
unsafe fn register(resource:usize,flags:u32)->Answer{
    if resource==0{return Err(INVALID);}if flags!=0{return Err(if flags==1{UNSUPPORTED}else{INVALID});}
    let _lease=Lease::acquire()?;let ctx=current()?;let (expected,single_thread)=binding(ctx)?;
    let identity=com::query(resource,&com::IDENTITY)?;
    if REGISTRY.lock().resources.iter().any(|r|r.identity.0==identity.0){return Err(INVALID_HANDLE);}
    let (source,texture)=match com::query(resource,&com::TEXTURE){Ok(r)=>(r,true),Err(_)=>(com::query(resource,&com::BUFFER)?,false)};
    let device=com::device(source.0)?;let device_identity=com::query(device.0,&com::IDENTITY)?;
    if device_identity.0!=expected{return Err(INVALID_CONTEXT);}
    let mut alignment=256u32;let mut pitch_alignment=256u32;
    if texture{let mut ordinal=0;cuda(C32_CTX_DEVICE,&[pointer(&mut ordinal)])?;
        cuda(C32_DEVICE_ATTR,&[pointer(&mut alignment),14,ordinal])?;
        cuda(C32_DEVICE_ATTR,&[pointer(&mut pitch_alignment),51,ordinal])?;
        if !alignment.is_power_of_two()||!pitch_alignment.is_power_of_two(){return Err(UNSUPPORTED);}}
    let layout=if texture{let d=com::texture_desc(source.0);
        if d.mips!=1||d.arrays!=1||d.samples!=1{return Err(UNSUPPORTED);}
        Layout::texture(d.width,d.height,d.format,pitch_alignment.max(256))?
    }else{Layout::buffer(com::buffer_desc(source.0).bytes)?};
    let multithread=if single_thread!=0{com::Owned(0)}else{let m=com::query(device.0,&com::MULTITHREAD)?;com::protect(m.0)?;m};
    let context=com::immediate(device.0)?;let staging=com::staging(device.0,source.0,texture)?;let ready=com::ready(device.0)?;
    let mut host=Vec::new();host.try_reserve_exact(layout.bytes as usize).map_err(|_|OOM)?;host.resize(layout.bytes as usize,0);
    REGISTRY.lock().resources.try_reserve(1).map_err(|_|OOM)?;
    let bytes=layout.bytes.checked_add(alignment-1).ok_or(INVALID)?;let mut allocation=0;
    cuda(C32_ALLOC,&[pointer(&mut allocation),bytes])?;
    let device=d3d10_copy::align(allocation,alignment)?;
    let r=Resource{identity,source,staging,context,multithread,ready,ctx,layout,allocation,device,host,flags:0,mapped:false,poisoned:false};
    REGISTRY.lock().resources.push(Box::new(r));Ok(())
}
unsafe fn batch(count:u32,resources:*mut *mut c_void,unmap:bool)->Answer{
    if count==0{return Ok(());}if count>32||resources.is_null()||
        resources as usize as u64+count as u64*4>1u64<<32{return Err(INVALID);}
    let _lease=Lease::acquire()?;let ctx=current()?;let mut found=Vec::with_capacity(count as usize);
    for i in 0..count as usize{let r=find(std::ptr::read_unaligned(resources.add(i)) as usize,ctx)?;
        if found.contains(&r){return Err(INVALID_HANDLE);}let rref=&*r;
        if rref.poisoned{return Err(UNKNOWN);}if rref.mapped!=unmap{return Err(if unmap{NOT_MAPPED}else{ALREADY_MAPPED});}found.push(r);}
    cuda(C32_CTX_SYNC,&[])?;
    for &p in &found{let r=&mut *p;let result=if unmap{
        if d3d10_copy::output(r.flags){cuda(C32_DTOH,&[pointer(r.host.as_mut_ptr()),r.device,r.layout.bytes])
            .and_then(|()|transfer(r,false)).and_then(|()|{guarded(r,||com::copy(r.context.0,r.source.0,r.staging.0));wait(r)})}else{Ok(())}
    }else{
        if d3d10_copy::input(r.flags){guarded(r,||com::copy(r.context.0,r.staging.0,r.source.0));}
        wait(r).and_then(|()|if d3d10_copy::input(r.flags){transfer(r,true)}else{Ok(())})
    };if let Err(e)=result{for &p in &found{(*p).poisoned=true;}return Err(e);}}
    for p in found{(*p).mapped=!unmap;}Ok(())
}
unsafe fn property(resource:usize,sub:u32,kind:u32,out:[*mut u32;3])->Answer{
    let _lease=Lease::acquire()?;let r=&*find(resource,current()?)?;
    if sub!=0{return Err(INVALID);}if r.poisoned{return Err(UNKNOWN);}
    if kind<2&&out[0].is_null(){return Err(INVALID);}
    if kind!=3&&!r.mapped{return Err(NOT_MAPPED);}
    let values=match kind{
        0 if !out[0].is_null()=>[r.device,0,0],
        1 if !out[0].is_null()=>[r.layout.bytes,0,0],
        2 if r.layout.texture=>[r.layout.pitch,r.layout.bytes,0],
        3 if r.layout.texture=>[r.layout.width,r.layout.height,0],
        _=>return Err(INVALID_HANDLE),
    };for (p,n) in out.into_iter().zip(values){if !p.is_null(){std::ptr::write_unaligned(p,n);}}Ok(())
}
unsafe fn set_flags(resource:usize,flags:u32)->Answer{
    let _lease=Lease::acquire()?;let r=&mut *find(resource,current()?)?;
    if flags>2{return Err(INVALID);}if r.mapped{return Err(ALREADY_MAPPED);}if r.poisoned{return Err(UNKNOWN);}r.flags=flags;Ok(())
}
unsafe fn unregister(resource:usize)->Answer{
    let _lease=Lease::acquire()?;let ctx=current()?;let r=find(resource,ctx)?;
    if (*r).mapped{return Err(ALREADY_MAPPED);}cuda(C32_CTX_SYNC,&[])?;
    cuda(C32_FREE,&[(*r).allocation])?;
    let removed={let mut s=REGISTRY.lock();let i=s.resources.iter().position(|p|std::ptr::eq(&**p,r)).ok_or(INVALID_HANDLE)?;s.resources.swap_remove(i)};
    drop(removed);Ok(())
}
pub(super) unsafe fn destroyed(ctx:u32){let Ok(_lease)=Lease::acquire()else{return;};
    let (resources,bindings)={let mut s=REGISTRY.lock();let mut r=Vec::new();let mut b=Vec::new();
        let mut i=0;while i<s.resources.len(){if s.resources[i].ctx==ctx{r.push(s.resources.swap_remove(i));}else{i+=1;}}
        let mut i=0;while i<s.bindings.len(){if s.bindings[i].ctx==ctx{b.push(s.bindings.swap_remove(i));}else{i+=1;}}(r,b)};
    drop(resources);drop(bindings);
}
#[no_mangle]
pub unsafe extern "system" fn cuD3D10CtxCreate(out:*mut *mut c_void,device:*mut i32,flags:u32,graphics:*mut c_void)->i32{
    traced("cuD3D10CtxCreate",&[pointer(out),pointer(device),flags,pointer(graphics)],||(||{
        if out.is_null()||device.is_null()||graphics.is_null(){return Err(INVALID);}
        let _lease=Lease::acquire()?;let d=com::query(graphics as usize,&com::DEVICE)?;
        let identity=com::query(d.0,&com::IDENTITY)?;let thread=if com::flags(d.0)&1!=0{GetCurrentThreadId()}else{0};
        REGISTRY.lock().bindings.try_reserve(1).map_err(|_|OOM)?;
        let mut ordinal=0;cuda(C32_DEVICE_GET,&[pointer(&mut ordinal),0])?;let mut ctx=0;
        cuda(C32_CTX_CREATE,&[pointer(&mut ctx),flags,ordinal])?;
        REGISTRY.lock().bindings.push(Binding{ctx,device:identity,thread});
        std::ptr::write_unaligned(out,ptr(ctx));std::ptr::write_unaligned(device,ordinal as i32);Ok(())
    })().err().unwrap_or(0))
}
macro_rules! entry {($name:ident($($arg:ident:$ty:ty),*)[$($word:expr),*]=>$body:expr)=>{
    #[no_mangle]pub unsafe extern "system" fn $name($($arg:$ty),*)->i32{traced(stringify!($name),&[$($word),*],||($body).err().unwrap_or(0))}
};}
entry!(cuD3D10RegisterResource(resource:*mut c_void,flags:u32)[pointer(resource),flags]=>register(resource as usize,flags));
entry!(cuD3D10UnregisterResource(resource:*mut c_void)[pointer(resource)]=>unregister(resource as usize));
entry!(cuD3D10MapResources(count:u32,resources:*mut *mut c_void)[count,pointer(resources)]=>batch(count,resources,false));
entry!(cuD3D10UnmapResources(count:u32,resources:*mut *mut c_void)[count,pointer(resources)]=>batch(count,resources,true));
entry!(cuD3D10ResourceSetMapFlags(resource:*mut c_void,flags:u32)[pointer(resource),flags]=>set_flags(resource as usize,flags));
entry!(cuD3D10ResourceGetMappedPointer(out:*mut u32,resource:*mut c_void,sub:u32)[pointer(out),pointer(resource),sub]=>property(resource as usize,sub,0,[out,std::ptr::null_mut(),std::ptr::null_mut()]));
entry!(cuD3D10ResourceGetMappedSize(out:*mut u32,resource:*mut c_void,sub:u32)[pointer(out),pointer(resource),sub]=>property(resource as usize,sub,1,[out,std::ptr::null_mut(),std::ptr::null_mut()]));
entry!(cuD3D10ResourceGetMappedPitch(pitch:*mut u32,slice:*mut u32,resource:*mut c_void,sub:u32)[pointer(pitch),pointer(slice),pointer(resource),sub]=>property(resource as usize,sub,2,[pitch,slice,std::ptr::null_mut()]));
entry!(cuD3D10ResourceGetSurfaceDimensions(width:*mut u32,height:*mut u32,depth:*mut u32,resource:*mut c_void,sub:u32)[pointer(width),pointer(height),pointer(depth),pointer(resource),sub]=>property(resource as usize,sub,3,[width,height,depth]));
