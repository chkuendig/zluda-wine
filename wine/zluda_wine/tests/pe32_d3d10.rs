// SPDX-License-Identifier: MIT OR Apache-2.0
//! Real PE32 COM vtables and CUDA packet marshalling; all pixels are synthetic.
use super::{raw,wow64_pe as api,wow64_protocol::*};
use super::wow64_d3d10::com;
use std::{ffi::c_void,sync::{Mutex,OnceLock,atomic::{AtomicU32,AtomicI32,Ordering::*}}};
#[repr(C)] struct Object {table:*const usize,kind:u32,refs:AtomicU32}
struct Fixture {objects:[usize;8]}
static FIXTURE:OnceLock<Fixture>=OnceLock::new();
struct Images {texture:[u8;8],stage:[u8;8],buffer:[u8;7],bstage:[u8;7]}
static IMAGES:Mutex<Images>=Mutex::new(Images{texture:[1,2,0xa5,0xa5,3,4,0xa5,0xa5],stage:[0;8],buffer:[1,2,3,4,5,6,7],bstage:[0;7]});
static GPU:Mutex<Vec<u8>>=Mutex::new(Vec::new());
static CTX:AtomicU32=AtomicU32::new(0x40000001);
static NEXT_CTX:AtomicU32=AtomicU32::new(0x40000010);
static NEXT_ALLOC:AtomicU32=AtomicU32::new(256);
static FLAGS:AtomicU32=AtomicU32::new(0);
static DEPTH:AtomicI32=AtomicI32::new(0);
static PROTECTED:AtomicU32=AtomicU32::new(0);
static QUERIES:AtomicU32=AtomicU32::new(0);
static COPY_IN:AtomicU32=AtomicU32::new(0);
static COPY_OUT:AtomicU32=AtomicU32::new(0);
static FREES:AtomicU32=AtomicU32::new(0);
static POLL:AtomicU32=AtomicU32::new(0);
static MAP_DELAY:AtomicU32=AtomicU32::new(1);
static DATA_ERROR:AtomicU32=AtomicU32::new(0);
fn fixture()->&'static Fixture{FIXTURE.get_or_init(||unsafe{
    let mut table=vec![0usize;112];
    for (slot,f) in [(0,query as *const ()),(2,release as *const ()),(3,get_device as *const ()),
        (5,create_texture as *const ()),(6,protected as *const ()),(10,description as *const ()),
        (14,map as *const ()),(15,unmap as *const ()),(24,create_query as *const ()),
        (28,end as *const ()),(29,data as *const ()),(38,flags as *const ()),
        (40,immediate as *const ()),(47,copy as *const ()),(111,flush as *const ())]{table[slot]=f as usize;}
    let base=Box::leak(table.into_boxed_slice()).as_ptr();
    let mut objects=[0;8];for (i,o) in objects.iter_mut().enumerate(){
        let mut t=std::slice::from_raw_parts(base,112).to_vec();
        if i==0{t[3]=create_buffer as *const () as usize;}
        if i==4{t[3]=enter as *const () as usize;t[4]=leave as *const () as usize;t[5]=protect as *const () as usize;}
        *o=Box::into_raw(Box::new(Object{table:Box::leak(t.into_boxed_slice()).as_ptr(),kind:i as u32,refs:AtomicU32::new(1)})) as usize;
    }Fixture{objects}
})}
pub fn device()->usize{fixture().objects[0]}
pub fn current()->u32{CTX.load(Relaxed)}
pub fn create_context()->u32{let ctx=NEXT_CTX.fetch_add(16,Relaxed);CTX.store(ctx,Relaxed);ctx}
unsafe fn kind(this:usize)->u32{(*(this as *const Object)).kind}
unsafe fn retain(this:usize){(*(this as *const Object)).refs.fetch_add(1,Relaxed);}
unsafe extern "system" fn release(this:usize)->u32{
    let old=(*(this as *const Object)).refs.fetch_sub(1,Relaxed);assert!(old>1,"release of fixture-owned baseline reference");old-1
}
unsafe extern "system" fn query(this:usize,iid:*const com::Guid,out:*mut usize)->i32{
    QUERIES.fetch_add(1,Relaxed);*out=0;let k=kind(this);
    let selected=if *iid==com::IDENTITY{this}else if *iid==com::DEVICE&&k==0{device()}
        else if *iid==com::MULTITHREAD&&k==0{fixture().objects[4]}
        else if *iid==com::TEXTURE&&matches!(k,1|2){this}
        else if *iid==com::BUFFER&&matches!(k,6|7){this}else{return 0x80004002u32 as i32;};
    retain(selected);*out=selected;0
}
unsafe extern "system" fn get_device(_:usize,out:*mut usize){*out=device();retain(*out);}
unsafe extern "system" fn flags(_:usize)->u32{FLAGS.load(Relaxed)}
unsafe extern "system" fn immediate(_:usize,out:*mut usize){*out=fixture().objects[3];retain(*out);}
unsafe extern "system" fn description(this:usize,out:*mut c_void){
    if matches!(kind(this),1|2){out.cast::<com::Texture>().write(com::Texture{width:2,height:2,mips:1,arrays:1,format:61,samples:1,quality:0,usage:0,bind:8,cpu:0,misc:0});}
    else{out.cast::<com::Buffer>().write(com::Buffer{bytes:7,usage:0,bind:1,cpu:0,misc:0,stride:0});}
}
unsafe extern "system" fn create_texture(_:usize,desc:*const com::Texture,initial:*const c_void,out:*mut usize)->i32{
    assert!(initial.is_null());let d=&*desc;assert_eq!((d.usage,d.bind,d.cpu,d.misc),(3,0,0x30000,0));
    *out=fixture().objects[2];retain(*out);0
}
unsafe extern "system" fn create_buffer(_:usize,desc:*const com::Buffer,initial:*const c_void,out:*mut usize)->i32{
    assert!(initial.is_null());let d=&*desc;assert_eq!((d.bytes,d.usage,d.bind,d.cpu,d.misc),(7,3,0,0x30000,0));
    *out=fixture().objects[7];retain(*out);0
}
unsafe extern "system" fn create_query(_:usize,desc:*const u32,out:*mut usize)->i32{
    assert_eq!(std::slice::from_raw_parts(desc,2),&[0,0]);*out=fixture().objects[5];retain(*out);0
}
unsafe extern "system" fn protect(_:usize,value:i32)->i32{assert_eq!(value,1);PROTECTED.swap(1,Relaxed) as i32}
unsafe extern "system" fn protected(_:usize)->i32{PROTECTED.load(Relaxed) as i32}
unsafe extern "system" fn enter(_:usize){assert_eq!(DEPTH.fetch_add(1,Relaxed),0);}
unsafe extern "system" fn leave(_:usize){assert_eq!(DEPTH.fetch_sub(1,Relaxed),1);}
fn guarded(){assert_eq!(DEPTH.load(Relaxed),if FLAGS.load(Relaxed)&1==0{1}else{0});}
unsafe extern "system" fn copy(_:usize,dst:usize,src:usize){guarded();let mut i=IMAGES.lock().unwrap();
    match (kind(dst),kind(src)){
        (2,1)=>{i.stage=i.texture;COPY_IN.fetch_add(1,Relaxed);}
        (1,2)=>{i.texture=i.stage;COPY_OUT.fetch_add(1,Relaxed);}
        (7,6)=>i.bstage=i.buffer,(6,7)=>i.buffer=i.bstage,_=>panic!("wrong resource copy"),
    }
}
unsafe extern "system" fn map(_:usize,resource:usize,sub:u32,mode:u32,flags:u32,out:*mut com::Mapped)->i32{
    guarded();assert_eq!(sub,0);assert!(mode==1||mode==2);assert_eq!(flags,0x100000);
    if MAP_DELAY.swap(0,Relaxed)!=0{return 0x887a000au32 as i32;}
    let mut images=IMAGES.lock().unwrap();let (data,row)=match kind(resource){2=>(images.stage.as_mut_ptr(),4),7=>(images.bstage.as_mut_ptr(),0),_=>panic!("not staging")};
    out.write(com::Mapped{data,row,depth:0});0
}
unsafe extern "system" fn unmap(_:usize,_:usize,sub:u32){guarded();assert_eq!(sub,0);}
unsafe extern "system" fn end(_:usize,_:usize){guarded();POLL.store(1,Relaxed);}
unsafe extern "system" fn flush(_:usize){guarded();}
unsafe extern "system" fn data(_:usize,_:usize,out:*mut c_void,bytes:u32,flags:u32)->i32{
    guarded();assert!(out.is_null());assert_eq!((bytes,flags),(0,1));
    if DATA_ERROR.swap(0,Relaxed)!=0{return 0x887a0005u32 as i32;}POLL.swap(0,Relaxed) as i32
}
pub unsafe fn submit(c:&Call32)->Option<i32>{
    let a=&c.args;match c.opcode{
        C32_CTX_DEVICE=>{ptr::<u32>(a[0]).write(0);Some(0)}
        C32_DEVICE_ATTR if matches!(a[1],14|51)=>{ptr::<u32>(a[0]).write(256);Some(0)}
        C32_ALLOC=>{if a[0]==0||a[2]!=4{return Some(INVALID);}assert_eq!(DEPTH.load(Relaxed),0);let p=NEXT_ALLOC.fetch_add((a[1]+255)&!255,Relaxed);
            let mut gpu=GPU.lock().unwrap();gpu.resize((p+a[1]) as usize,0x77);ptr::<u32>(a[0]).write(p);Some(0)}
        C32_FREE=>{assert_eq!(DEPTH.load(Relaxed),0);FREES.fetch_add(1,Relaxed);Some(0)}
        C32_HTOD|C32_DTOH=>{assert_eq!(DEPTH.load(Relaxed),0);let mut g=GPU.lock().unwrap();
            if c.opcode==C32_HTOD{g[a[0] as usize..(a[0]+a[2]) as usize].copy_from_slice(std::slice::from_raw_parts(ptr(a[1]),a[2] as usize));}
            else{std::ptr::copy_nonoverlapping(g.as_ptr().add(a[1] as usize),ptr(a[0]),a[2] as usize);}Some(0)}
        C32_CTX_SYNC=>{assert_eq!(DEPTH.load(Relaxed),0);Some(0)}
        C32_CTX_DESTROY=>{CTX.store(0,Relaxed);Some(0)}
        _=>None,
    }
}
unsafe fn map_one(resource:usize,unmap:bool)->i32{
    let mut p=resource as *mut c_void;raw(if unmap{api::cuD3D10UnmapResources as *const ()}else{api::cuD3D10MapResources as *const ()},&[1,pointer(&mut p)])
}
unsafe fn pointer_of(resource:usize)->u32{let mut p=0;assert_eq!(raw(api::cuD3D10ResourceGetMappedPointer as *const (),&[pointer(&mut p),resource as u32,0]),0);p}
unsafe fn registration_none_roundtrips(){
    let frees=FREES.load(Relaxed);
    for (resource,texture,name) in [(fixture().objects[1],true,"Texture2D"),(fixture().objects[6],false,"Buffer")]{
        let initial={let i=IMAGES.lock().unwrap();if texture{i.texture[0]}else{i.buffer[0]}};
        assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[resource as u32,0]),0);
        assert_eq!(map_one(resource,false),0);let device=pointer_of(resource) as usize;
        {let mut g=GPU.lock().unwrap();assert_eq!(g[device],initial);g[device]=initial^0x5a;}
        if texture{let (mut pitch,mut slice)=(0,0);
            assert_eq!(raw(api::cuD3D10ResourceGetMappedPitch as *const (),&[pointer(&mut pitch),pointer(&mut slice),resource as u32,0]),0);
            assert_eq!((pitch,slice),(256,512));}
        assert_eq!(map_one(resource,true),0);
        {let mut i=IMAGES.lock().unwrap();let value=if texture{&mut i.texture[0]}else{&mut i.buffer[0]};assert_eq!(*value,initial^0x5a);*value=initial;}
        assert_eq!(raw(api::cuD3D10UnregisterResource as *const (),&[resource as u32]),0);
        println!("PE32 D3D10 {name}/NONE PASS: registration, pointer/pitch, fresh copy roundtrip, unregister");
    }
    assert_eq!(FREES.load(Relaxed),frees+2);
}
pub unsafe fn off(){
    let before=QUERIES.load(Relaxed);let mut canary=[0xdeadbeefu32;4];let p=pointer(canary.as_mut_ptr());
    for value in [None,Some("0"),Some("true"),Some("01"),Some("1 ")]{
        match value{None=>std::env::remove_var("ZLUDA_WINE_INTEROP"),Some(v)=>std::env::set_var("ZLUDA_WINE_INTEROP",v)}
        for (f,a) in [(api::cuD3D10RegisterResource as *const (),vec![1,0]),
            (api::cuD3D10UnregisterResource as *const (),vec![1]),
            (api::cuD3D10MapResources as *const (),vec![1,1]),(api::cuD3D10UnmapResources as *const (),vec![1,1]),
            (api::cuD3D10ResourceSetMapFlags as *const (),vec![1,2]),
            (api::cuD3D10ResourceGetMappedPointer as *const (),vec![p,1,0]),
            (api::cuD3D10ResourceGetMappedSize as *const (),vec![p,1,0]),
            (api::cuD3D10ResourceGetMappedPitch as *const (),vec![p,p+4,1,0]),
            (api::cuD3D10ResourceGetSurfaceDimensions as *const (),vec![p,p+4,p+8,1,0]),
            (super::wow64_stubs::cuD3D10ResourceGetMappedArray as *const (),vec![p,1,0])]{assert_eq!(raw(f,&a),801);}
        assert_eq!(canary,[0xdeadbeef;4]);
    }
    let mut context=0u32;let mut ordinal=-1i32;
    assert_eq!(raw(api::cuD3D10CtxCreate as *const (),&[pointer(&mut context),pointer(&mut ordinal),4,1]),0);
    assert_eq!(ordinal,0);assert_eq!(raw(api::cuCtxDestroy_v2 as *const (),&[context]),0);
    assert_eq!(QUERIES.load(Relaxed),before,"OFF context and resource calls never touch COM");
    std::env::set_var("ZLUDA_WINE_INTEROP","1");
    println!("PE32 D3D10 OFF PASS: stage1 context, all801 resource results, untouched outputs and no COM");
}
pub unsafe fn run(first_context:u32){
    let texture=fixture().objects[1];let buffer=fixture().objects[6];
    assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[texture as u32,1]),801);
    assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[texture as u32,0]),0);
    assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[texture as u32,0]),400);
    let (mut width,mut height,mut depth)=(0,0,1);
    assert_eq!(raw(api::cuD3D10ResourceGetSurfaceDimensions as *const (),&[pointer(&mut width),pointer(&mut height),pointer(&mut depth),texture as u32,0]),0);
    assert_eq!((width,height,depth),(2,2,0));
    let mut p=0xdeadbeefu32;
    assert_eq!(raw(api::cuD3D10ResourceGetMappedPointer as *const (),&[pointer(&mut p),texture as u32,0]),211);assert_eq!(p,0xdeadbeef);
    let duplicates=[texture as *mut c_void;2];
    assert_eq!(raw(api::cuD3D10MapResources as *const (),&[2,pointer(duplicates.as_ptr())]),400);
    assert_eq!(map_one(texture,false),0);let offset=pointer_of(texture) as usize;
    let (mut pitch,mut slice,mut bytes)=(0,0,0);
    assert_eq!(raw(api::cuD3D10ResourceGetMappedPitch as *const (),&[pointer(&mut pitch),pointer(&mut slice),texture as u32,0]),0);
    assert_eq!(raw(api::cuD3D10ResourceGetMappedSize as *const (),&[pointer(&mut bytes),texture as u32,0]),0);
    assert_eq!((pitch,slice,bytes),(256,512,512));
    {let mut g=GPU.lock().unwrap();assert_eq!(&g[offset..offset+2],&[1,2]);assert_eq!(&g[offset+256..offset+258],&[3,4]);g[offset+1]=8;g[offset+257]=7;}
    assert_eq!(map_one(texture,false),208);
    assert_eq!(raw(api::cuD3D10ResourceSetMapFlags as *const (),&[texture as u32,1]),208);
    assert_eq!(raw(api::cuD3D10UnregisterResource as *const (),&[texture as u32]),208);
    assert_eq!(map_one(texture,true),0);assert_eq!(IMAGES.lock().unwrap().texture,[1,8,0xa5,0xa5,3,7,0xa5,0xa5]);
    IMAGES.lock().unwrap().texture[0]=9;
    assert_eq!(map_one(texture,false),0);assert_eq!(GPU.lock().unwrap()[offset],9);assert_eq!(map_one(texture,true),0);
    assert_eq!(raw(api::cuD3D10ResourceSetMapFlags as *const (),&[texture as u32,1]),0);
    let outputs=COPY_OUT.load(Relaxed);assert_eq!(map_one(texture,false),0);GPU.lock().unwrap()[offset]=55;
    assert_eq!(map_one(texture,true),0);assert_eq!(IMAGES.lock().unwrap().texture[0],9);assert_eq!(COPY_OUT.load(Relaxed),outputs);
    assert_eq!(raw(api::cuD3D10ResourceSetMapFlags as *const (),&[texture as u32,2]),0);
    let inputs=COPY_IN.load(Relaxed);assert_eq!(map_one(texture,false),0);assert_eq!(COPY_IN.load(Relaxed),inputs);
    {let mut g=GPU.lock().unwrap();assert_eq!(g[offset],55);g[offset..offset+2].copy_from_slice(&[6,7]);g[offset+256..offset+258].copy_from_slice(&[8,9]);}
    assert_eq!(map_one(texture,true),0);assert_eq!(IMAGES.lock().unwrap().texture,[6,7,0xa5,0xa5,8,9,0xa5,0xa5]);
    assert_eq!(raw(api::cuD3D10UnregisterResource as *const (),&[texture as u32]),0);
    assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[buffer as u32,0]),0);
    assert_eq!(map_one(buffer,false),0);let b=pointer_of(buffer) as usize;GPU.lock().unwrap()[b]=99;
    assert_eq!(raw(api::cuD3D10ResourceGetMappedPitch as *const (),&[pointer(&mut pitch),0,buffer as u32,0]),400);
    assert_eq!(map_one(buffer,true),0);assert_eq!(IMAGES.lock().unwrap().buffer[0],99);
    assert_eq!(raw(api::cuD3D10UnregisterResource as *const (),&[buffer as u32]),0);assert_eq!(FREES.load(Relaxed),2);
    registration_none_roundtrips();
    FLAGS.store(1,Relaxed);let mut second=0;let mut ordinal=0;
    assert_eq!(raw(api::cuD3D10CtxCreate as *const (),&[pointer(&mut second),pointer(&mut ordinal),4,device() as u32]),0);
    assert_eq!(raw(api::cuD3D10RegisterResource as *const (),&[texture as u32,0]),0);
    let queries=QUERIES.load(Relaxed);
    assert_eq!(std::thread::spawn(move||unsafe{map_one(texture,false)}).join().unwrap(),801);
    assert_eq!(QUERIES.load(Relaxed),queries,"wrong-thread single-threaded access stops before COM");
    DATA_ERROR.store(1,Relaxed);assert_eq!(map_one(texture,false),999);
    assert_eq!(raw(api::cuD3D10ResourceGetMappedPointer as *const (),&[pointer(&mut p),texture as u32,0]),999);
    assert_eq!(raw(api::cuCtxDestroy_v2 as *const (),&[second]),0);
    assert_eq!(raw(api::cuCtxDestroy_v2 as *const (),&[first_context]),0);
    for &p in &fixture().objects{assert_eq!((*(p as *const Object)).refs.load(Relaxed),1,"COM reference leak");}
    assert_eq!(DEPTH.load(Relaxed),0);
    println!("PE32 D3D10 PASS: texture+buffer staging, fresh maps, read-only/discard, pitch/padding, duplicates/lifecycle, protected calls, single-thread owner, failure poison, context/COM cleanup");
}
