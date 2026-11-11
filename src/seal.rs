use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptUnprotectData};

const UNREADABLE: &str = "Saved login can't be read on this Windows user. Add the account again in welffi accounts.";

pub fn unseal(sealed: &str) -> Result<String, String> {
    let bytes = STANDARD.decode(sealed.trim()).map_err(|_| UNREADABLE.to_string())?;
    let input = CRYPT_INTEGER_BLOB { cbData: bytes.len() as u32, pbData: bytes.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB { cbData: 0, pbData: null_mut() };
    let ok = unsafe { CryptUnprotectData(&input, null_mut(), null(), null(), null(), CRYPTPROTECT_UI_FORBIDDEN, &mut output) };
    if ok == 0 {
        return Err(UNREADABLE.to_string());
    }
    let plain = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) }.to_vec();
    unsafe { LocalFree(output.pbData as _) };
    String::from_utf8(plain).map_err(|_| UNREADABLE.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn junk_is_unreadable() {
        assert_eq!(unseal("not base64!").unwrap_err(), UNREADABLE);
        assert_eq!(unseal("AAAA").unwrap_err(), UNREADABLE);
    }
}
