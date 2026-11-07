// SPDX-License-Identifier: MIT OR Apache-2.0
//! Native projection of the PE64 relay with synthetic COM and CUDA storage.
use crate::{cuda::*,d3d11 as com,pe_interop as api};
use std::{ffi::c_void,sync::{Mutex,OnceLock,atomic::{AtomicUsize,Ordering::*}}};
pub mod os {
    pub fn thread_identity()->usize{1}
    pub fn GetTickCount64()->u64{0}
    pub fn Sleep(_:u32){}
}
pub mod callbacks {pub fn inside()->bool{false}}
static CALLS:AtomicUsize=AtomicUsize::new(0);
static DEPTH:AtomicUsize=AtomicUsize::new(0);
static PROTECTED:AtomicUsize=AtomicUsize::new(0);
#[repr(C)] struct Object {table:*const usize,refs:AtomicUsize}
struct Objects([usize;6]);
static OBJECTS:OnceLock<Objects>=OnceLock::new();
struct Images {texture:[u8;48],stage:[u8;48],cuda:[u8;32]}
static IMAGES:Mutex<Images>=Mutex::new(Images{texture:[3;48],stage:[0;48],cuda:[0;32]});
fn objects()->&'static [usize;6]{&OBJECTS.get_or_init(||{
    let mut objects=[0;6];
    for (i,p) in objects.iter_mut().enumerate(){
        let mut v=vec![0usize;112];
        for (slot,f) in [(0,query as *const ()),(2,release as *const ()),(3,device as *const ()),
            (5,texture as *const ()),(10,description as *const ()),(14,map as *const ()),
            (15,unmap as *const ()),(24,ready as *const ()),(28,end as *const ()),
            (29,data as *const ()),(38,flags as *const ()),(40,immediate as *const ()),
            (47,copy as *const ()),(111,flush as *const ())]{v[slot]=f as usize;}
        if i==4{v[3]=enter as *const () as usize;v[4]=leave as *const () as usize;v[5]=protect as *const () as usize;v[6]=protected as *const () as usize;}
        *p=Box::into_raw(Box::new(Object{table:Box::leak(v.into_boxed_slice()).as_ptr(),refs:AtomicUsize::new(1)})) as usize;
    }Objects(objects)
}).0}
unsafe fn retain(p:usize){(*(p as *const Object)).refs.fetch_add(1,Relaxed);}
unsafe extern "win64" fn query(this:*mut c_void,iid:*const com::Guid,out:*mut usize)->i32{
    let same=|a:*const com::Guid,b:&com::Guid|std::slice::from_raw_parts(a.cast::<u8>(),16)==std::slice::from_raw_parts((b as *const com::Guid).cast::<u8>(),16);
    let p=if same(iid,&com::UNKNOWN){this as usize}else if same(iid,&com::TEXTURE)&&this as usize==objects()[1]{objects()[1]}
        else if same(iid,&com::MULTITHREAD)&&this as usize==objects()[0]{objects()[4]}else{*out=0;return -1;};
    retain(p);*out=p;0
}
unsafe extern "win64" fn release(this:*mut c_void)->u32{let old=(*(this as *const Object)).refs.fetch_sub(1,Relaxed);assert!(old>1);(old-1) as u32}
unsafe extern "win64" fn device(_: *mut c_void,out:*mut usize){*out=objects()[0];retain(*out);}
unsafe extern "win64" fn description(_: *mut c_void,out:*mut com::Description){*out=com::Description{width:2,height:1,mips:1,arrays:1,format:2,samples:1,..Default::default()};}
unsafe extern "win64" fn flags(_: *mut c_void)->u32{0}
unsafe extern "win64" fn immediate(_: *mut c_void,out:*mut usize){*out=objects()[3];retain(*out);}
unsafe extern "win64" fn texture(_: *mut c_void,d:*const com::Description,initial:*const c_void,out:*mut usize)->i32{
    assert!(initial.is_null());assert_eq!(((*d).usage,(*d).cpu,(*d).bind,(*d).misc),(3,0x30000,0,0));*out=objects()[2];retain(*out);0
}
unsafe extern "win64" fn ready(_: *mut c_void,d:*const com::Query,out:*mut usize)->i32{assert_eq!(((*d).kind,(*d).misc),(0,0));*out=objects()[5];retain(*out);0}
unsafe extern "win64" fn protect(_: *mut c_void,enabled:i32)->i32{assert_eq!(enabled,1);PROTECTED.store(1,Relaxed);0}
unsafe extern "win64" fn protected(_: *mut c_void)->i32{PROTECTED.load(Relaxed) as i32}
unsafe extern "win64" fn enter(_: *mut c_void){assert_eq!(DEPTH.fetch_add(1,Relaxed),0);}
unsafe extern "win64" fn leave(_: *mut c_void){assert_eq!(DEPTH.fetch_sub(1,Relaxed),1);}
unsafe extern "win64" fn copy(_: *mut c_void,dst:*mut c_void,src:*mut c_void){
    assert_eq!(DEPTH.load(Relaxed),1);let mut p=IMAGES.lock().unwrap();
    if src as usize==objects()[1]{assert_eq!(dst as usize,objects()[2]);p.stage=p.texture;}else{assert_eq!((src as usize,dst as usize),(objects()[2],objects()[1]));p.texture=p.stage;}
}
unsafe extern "win64" fn map(_: *mut c_void,resource:*mut c_void,sub:u32,kind:u32,flags:u32,out:*mut com::Mapped)->i32{
    assert_eq!(DEPTH.load(Relaxed),1);assert_eq!((resource as usize,sub,flags),(objects()[2],0,0x100000));assert!(kind==1||kind==2);
    let mut p=IMAGES.lock().unwrap();*out=com::Mapped{data:p.stage.as_mut_ptr(),row:48,depth:48};0
}
unsafe extern "win64" fn unmap(_: *mut c_void,_:*mut c_void,_:u32){assert_eq!(DEPTH.load(Relaxed),1);}
unsafe extern "win64" fn end(_: *mut c_void,_:*mut c_void){assert_eq!(DEPTH.load(Relaxed),1);}
unsafe extern "win64" fn flush(_: *mut c_void){assert_eq!(DEPTH.load(Relaxed),1);}
unsafe extern "win64" fn data(_: *mut c_void,_:*mut c_void,out:*mut c_void,bytes:u32,flags:u32)->i32{assert_eq!(DEPTH.load(Relaxed),1);assert!(out.is_null());assert_eq!((bytes,flags),(0,1));0}
pub unsafe fn available(_: &[u8])->CUresult{Ok(())}
pub unsafe fn raw(name:&str,args:&[u64])->CUresult{
    CALLS.fetch_add(1,Relaxed);assert_eq!(DEPTH.load(Relaxed),0);
    match name{
        "cuCtxGetCurrent"=>*(args[0] as *mut usize)=0x99,
        "cuMemHostAlloc"=>{assert_eq!(args[1],32);*(args[0] as *mut usize)=Box::into_raw(Box::new([0u8;32])) as usize;},
        "cuMemFreeHost"=>{drop(Box::from_raw(args[0] as *mut [u8;32]));},
        "cuArrayCreate_v2"=>{let d=&*(args[1] as *const CUDA_ARRAY_DESCRIPTOR);assert_eq!((d.Width,d.Height,d.NumChannels),(2,1,4));*(args[0] as *mut usize)=0x30;},
        "cuEventCreate"=>*(args[0] as *mut usize)=0x20,
        "cuMemcpy2DAsync_v2"|"cuMemcpy2DAsync_v2_ptsz"=>{
            let c=&*(args[0] as *const CUDA_MEMCPY2D);assert_eq!((c.WidthInBytes,c.Height),(32,1));let mut p=IMAGES.lock().unwrap();
            if c.srcMemoryType.0==1{p.cuda.copy_from_slice(std::slice::from_raw_parts(c.srcHost.cast::<u8>(),32));}
            else{std::ptr::copy_nonoverlapping(p.cuda.as_ptr(),c.dstHost.cast::<u8>(),32);}
        },
        "cuEventRecord"|"cuEventRecord_ptsz"|"cuEventSynchronize"|"cuEventDestroy_v2"|"cuStreamSynchronize"|"cuStreamSynchronize_ptsz"|"cuArrayDestroy"=>{},
        _=>return Err(CUerror::NOT_SUPPORTED),
    }Ok(())
}
unsafe fn off(){
    let mut out=0x55usize as CUgraphicsResource;let h=std::ptr::null_mut();let before=CALLS.load(Relaxed);
    assert_eq!(api::cuGraphicsD3D11RegisterResource(&mut out,h,0),Err(CUerror::UNKNOWN));
    assert_eq!(api::cuGraphicsD3D11RegisterResource(&mut out,h,1),Err(CUerror::INVALID_VALUE));assert_eq!(out as usize,0x55);
    for f in [api::cuGraphicsMapResources,api::cuGraphicsMapResources_ptsz,api::cuGraphicsUnmapResources,api::cuGraphicsUnmapResources_ptsz]{assert_eq!(f(1,1usize as *mut _,CUstream(std::ptr::null_mut())),Err(CUerror::NOT_SUPPORTED));}
    let mut canary=[0xfeedbeefdeadbeefu64;4];let p=canary.as_mut_ptr();
    assert_eq!(api::cuGraphicsSubResourceGetMappedArray(p.cast(),out,0,0),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsResourceGetMappedPointer(p.cast(),p.add(1).cast(),out),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsResourceGetMappedPointer_v2(p.cast(),p.add(1).cast(),out),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsResourceGetMappedMipmappedArray(p.cast(),out),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsResourceSetMapFlags(out,0),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsResourceSetMapFlags_v2(out,2),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(api::cuGraphicsUnregisterResource(out),Err(CUerror::NOT_SUPPORTED));
    assert_eq!(canary,[0xfeedbeefdeadbeef;4]);assert_eq!(CALLS.load(Relaxed),before);
}
#[test]
fn d3d11_opt_in_off_on_off_preserves_stubs_and_copies(){unsafe{
    let old=std::env::var_os("ZLUDA_WINE_INTEROP");
    for value in [None,Some("0"),Some("true"),Some("01"),Some("1 ")]{match value{None=>std::env::remove_var("ZLUDA_WINE_INTEROP"),Some(v)=>std::env::set_var("ZLUDA_WINE_INTEROP",v)};off();}
    std::env::set_var("ZLUDA_WINE_INTEROP","1");let mut handle=std::ptr::null_mut();
    assert_eq!(api::cuGraphicsD3D11RegisterResource(&mut handle,objects()[1] as *mut _,0),Ok(()));
    assert_eq!(PROTECTED.load(Relaxed),1);
    assert_eq!(api::cuGraphicsMapResources(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));
    let mut array=std::ptr::null_mut();assert_eq!(api::cuGraphicsSubResourceGetMappedArray(&mut array,handle,0,0),Ok(()));assert_eq!(array as usize,0x30);
    IMAGES.lock().unwrap().cuda[0]=9;
    assert_eq!(api::cuGraphicsUnmapResources(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));assert_eq!(IMAGES.lock().unwrap().texture[0],9);
    IMAGES.lock().unwrap().texture[0]=7;
    assert_eq!(api::cuGraphicsMapResources_ptsz(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));assert_eq!(IMAGES.lock().unwrap().cuda[0],7);
    assert_eq!(api::cuGraphicsUnmapResources_ptsz(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));
    assert_eq!(api::cuGraphicsResourceSetMapFlags(handle,1),Ok(()));
    assert_eq!(api::cuGraphicsMapResources(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));IMAGES.lock().unwrap().cuda[0]=4;
    assert_eq!(api::cuGraphicsUnmapResources(1,&mut handle,CUstream(std::ptr::null_mut())),Ok(()));assert_eq!(IMAGES.lock().unwrap().texture[0],7);
    std::env::set_var("ZLUDA_WINE_INTEROP","0");off();
    std::env::set_var("ZLUDA_WINE_INTEROP","1");assert_eq!(api::cuGraphicsUnregisterResource(handle),Ok(()));
    for &p in objects(){assert_eq!((*(p as *const Object)).refs.load(Relaxed),1);}
    assert_eq!(DEPTH.load(Relaxed),0);
    match old{Some(v)=>std::env::set_var("ZLUDA_WINE_INTEROP",v),None=>std::env::remove_var("ZLUDA_WINE_INTEROP")}
}}
