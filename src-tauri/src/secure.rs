use anyhow::{anyhow, Result};
use std::ffi::c_void;

#[repr(C)]
struct Blob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[link(name = "crypt32")]
extern "system" {
    fn CryptProtectData(
        data_in: *const Blob,
        data_desc: *const u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut Blob,
    ) -> i32;
    fn CryptUnprotectData(
        data_in: *const Blob,
        data_desc: *mut *mut u16,
        entropy: *const Blob,
        reserved: *mut c_void,
        prompt: *mut c_void,
        flags: u32,
        data_out: *mut Blob,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn LocalFree(mem: *mut c_void) -> *mut c_void;
}

const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

fn free_blob(b: &Blob) {
    if !b.pb_data.is_null() {
        unsafe { LocalFree(b.pb_data as *mut c_void) };
    }
}

pub fn protect(plain: &[u8]) -> Result<Vec<u8>> {
    let data_in = Blob { cb_data: plain.len() as u32, pb_data: plain.as_ptr() as *mut u8 };
    let mut data_out = Blob { cb_data: 0, pb_data: std::ptr::null_mut() };
    let ok = unsafe {
        CryptProtectData(
            &data_in,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
    };
    if ok == 0 {
        free_blob(&data_out);
        return Err(anyhow!("DPAPI 加密失败 (错误码 {})", std::io::Error::last_os_error()));
    }
    let out = unsafe { std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize) }.to_vec();
    free_blob(&data_out);
    Ok(out)
}

pub fn unprotect(cipher: &[u8]) -> Result<Vec<u8>> {
    let data_in = Blob { cb_data: cipher.len() as u32, pb_data: cipher.as_ptr() as *mut u8 };
    let mut data_out = Blob { cb_data: 0, pb_data: std::ptr::null_mut() };
    let ok = unsafe {
        CryptUnprotectData(
            &data_in,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null_mut(),
            std::ptr::null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut data_out,
        )
    };
    if ok == 0 {
        free_blob(&data_out);
        return Err(anyhow!("DPAPI 解密失败 (错误码 {})", std::io::Error::last_os_error()));
    }
    let out = unsafe { std::slice::from_raw_parts(data_out.pb_data, data_out.cb_data as usize) }.to_vec();
    free_blob(&data_out);
    Ok(out)
}
