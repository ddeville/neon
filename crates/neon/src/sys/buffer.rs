use std::{mem::MaybeUninit, os::raw::c_void, slice};

use super::{
    bindings as napi,
    raw::{Env, Local},
};

pub unsafe fn new(env: Env, len: usize) -> Result<Local, napi::Status> {
    let (buf, bytes) = uninitialized(env, len)?;

    std::ptr::write_bytes(bytes, 0, len);

    Ok(buf)
}

pub unsafe fn uninitialized(env: Env, len: usize) -> Result<(Local, *mut u8), napi::Status> {
    let mut buf = MaybeUninit::uninit();
    let mut bytes = MaybeUninit::uninit();
    let status = napi::create_buffer(env, len, bytes.as_mut_ptr(), buf.as_mut_ptr());

    if status == napi::Status::PendingException {
        return Err(status);
    }

    assert_eq!(status, napi::Status::Ok);

    Ok((buf.assume_init(), bytes.assume_init().cast()))
}

pub unsafe fn new_external<T>(env: Env, data: T) -> Local
where
    T: AsMut<[u8]> + Send,
{
    // Safety: Boxing could move the data; must box before grabbing a raw pointer
    let mut data = Box::new(data);
    let buf = data.as_mut().as_mut();
    let length = buf.len();
    let mut result = MaybeUninit::uninit();

    assert_eq!(
        napi::create_external_buffer(
            env,
            length,
            buf.as_mut_ptr() as *mut _,
            Some(drop_external::<T>),
            Box::into_raw(data) as *mut _,
            result.as_mut_ptr(),
        ),
        napi::Status::Ok,
    );

    result.assume_init()
}

unsafe extern "C" fn drop_external<T>(_env: Env, _data: *mut c_void, hint: *mut c_void) {
    Box::<T>::from_raw(hint as *mut _);
}

/// # Safety
/// * Caller must ensure `env` and `buf` are valid
/// * The lifetime `'a` does not exceed the lifetime of `Env` or `buf`
pub unsafe fn as_mut_slice<'a>(env: Env, buf: Local) -> &'a mut [u8] {
    let mut data = MaybeUninit::uninit();
    let mut size = 0usize;

    assert_eq!(
        napi::get_buffer_info(env, buf, data.as_mut_ptr(), &mut size as *mut _),
        napi::Status::Ok,
    );

    if size == 0 {
        return &mut [];
    }

    slice::from_raw_parts_mut(data.assume_init().cast(), size)
}

/// # Safety
/// * Caller must ensure `env` and `buf` are valid
pub unsafe fn size(env: Env, buf: Local) -> usize {
    let mut data = MaybeUninit::uninit();
    let mut size = 0usize;

    assert_eq!(
        napi::get_buffer_info(env, buf, data.as_mut_ptr(), &mut size as *mut _),
        napi::Status::Ok,
    );

    size
}
