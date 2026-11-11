use crate::accounts::Account;
use crate::links;
use crate::log::Log;
use crate::roblox_api;
use crate::seal;
use crate::win;
use serde_json::Value;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::{HANDLE, WAIT_ABANDONED, WAIT_OBJECT_0};
use windows_sys::Win32::System::Threading::{CreateMutexW, WaitForSingleObject};

const CLOSE_TRAY_COPIES: &str = "Get-CimInstance Win32_Process -Filter \"Name='RobloxPlayerBeta.exe'\" | Where-Object { $_.CommandLine -like '*--launch-to-tray*' } | ForEach-Object { $p = Get-Process -Id $_.ProcessId -ErrorAction SilentlyContinue; if ($p -and $p.MainWindowHandle -eq 0) { Stop-Process -Id $p.Id -Force -ErrorAction SilentlyContinue } }";

static LOCK: AtomicUsize = AtomicUsize::new(0);
static STORAGE_LOCKED: AtomicBool = AtomicBool::new(false);

fn hold_lock(wait_ms: u32) -> bool {
    let mut handle = LOCK.load(Ordering::SeqCst);
    if handle == 0 {
        let name = win::wide("ROBLOX_singletonMutex");
        handle = unsafe { CreateMutexW(std::ptr::null(), 0, name.as_ptr()) } as usize;
        if handle == 0 {
            return false;
        }
        LOCK.store(handle, Ordering::SeqCst);
    }
    let result = unsafe { WaitForSingleObject(handle as HANDLE, wait_ms) };
    result == WAIT_OBJECT_0 || result == WAIT_ABANDONED
}

fn prepare_lock(log: &Log) {
    if hold_lock(0) {
        return;
    }
    win::run_hidden_powershell(CLOSE_TRAY_COPIES);
    if !hold_lock(2000) {
        log.warn("Another program holds the Roblox lock - joining anyway");
    }
}

fn client_storage() -> PathBuf {
    PathBuf::from(std::env::var_os("LOCALAPPDATA").unwrap_or_default()).join("Roblox").join("LocalStorage").join("appStorage.json")
}

fn clear_saved_accounts(text: &str) -> Option<String> {
    let mut root: Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).ok()?;
    let map = root.as_object_mut()?;
    let empty = Value::String("{}".to_string());
    let previous_set = map.get("PreviousAccountsList").is_some_and(|value| *value != empty);
    if previous_set {
        map.insert("PreviousAccountsList".to_string(), empty);
    }
    let blob_removed = map.shift_remove("AccountBlob").is_some();
    (previous_set || blob_removed).then(|| root.to_string())
}

fn set_read_only(path: &PathBuf, read_only: bool) -> std::io::Result<()> {
    let mut permissions = fs::metadata(path)?.permissions();
    if permissions.readonly() != read_only {
        permissions.set_readonly(read_only);
        fs::set_permissions(path, permissions)?;
    }
    Ok(())
}

fn forget_other_accounts() -> Result<(), String> {
    let path = client_storage();
    if !path.exists() {
        return Ok(());
    }
    let mut failure = String::new();
    for _ in 0..5 {
        let attempt = fs::read_to_string(&path).and_then(|text| {
            if let Some(cleared) = clear_saved_accounts(&text) {
                set_read_only(&path, false)?;
                fs::write(&path, cleared)?;
            }
            set_read_only(&path, true)
        });
        match attempt {
            Ok(()) => {
                STORAGE_LOCKED.store(true, Ordering::SeqCst);
                return Ok(());
            }
            Err(error) => failure = error.to_string(),
        }
        thread::sleep(Duration::from_millis(100));
    }
    Err(format!("Couldn't prepare Roblox's account storage: {failure}"))
}

pub fn release_client_storage() {
    if STORAGE_LOCKED.swap(false, Ordering::SeqCst) {
        let _ = set_read_only(&client_storage(), false);
    }
}

pub fn join_private(account: &Account, link: &str, log: &Log) -> Result<(), String> {
    prepare_lock(log);
    forget_other_accounts()?;
    let cookie = seal::unseal(&account.sealed_cookie)?;
    let server = roblox_api::resolve_private_server(&cookie, link)?;
    let query = links::private_query(&account.tracker, &server);
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    win::open_uri(&links::launch_uri(&account.tracker, &roblox_api::auth_ticket(&cookie)?, &query, now))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn saved_accounts_are_cleared_and_the_rest_kept() {
        let text = r#"{"A":1,"PreviousAccountsList":"{\"1\":\"x\"}","AccountBlob":"abc","Z":"keep"}"#;
        assert_eq!(clear_saved_accounts(text).unwrap(), r#"{"A":1,"PreviousAccountsList":"{}","Z":"keep"}"#);
    }

    #[test]
    fn already_clear_storage_is_left_alone() {
        assert_eq!(clear_saved_accounts(r#"{"A":1,"PreviousAccountsList":"{}"}"#), None);
        assert_eq!(clear_saved_accounts(r#"{"A":1}"#), None);
        assert_eq!(clear_saved_accounts("not json"), None);
    }
}
