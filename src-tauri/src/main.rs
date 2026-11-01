#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::ffi::{c_void, OsStr};
use std::os::windows::ffi::OsStrExt;
use std::ptr;

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            mount,
            unmount,
            check_link,
            save_credential,
            delete_credential,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

/* -------------------------------------------------------------------------- */
/*  挂载：将 UNC SMB 路径创建为本地目录符号链接（Directory Symbolic Link）     */
/*  注意：mklink 是 cmd.exe 内置命令，必须通过 `cmd /c mklink /d` 调用         */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn mount(
    local_path: String,
    unc_path: String,
    save_creds: bool,
    username: Option<String>,
    password: Option<String>,
) -> Result<String, String> {
    // 可选：将凭据写入 Windows 凭据管理器，供系统在重启后自动鉴权 SMB 共享
    if save_creds {
        if let (Some(user), Some(pass)) = (username, password) {
            let target = format!("SMBFolderMapper:{}", unc_path);
            let _ = save_credential(target, user, pass);
        }
    }

    let local = std::path::Path::new(&local_path);

    // 校验本地路径
    if local.exists() {
        let metadata = std::fs::symlink_metadata(local)
            .map_err(|e| format!("无法读取本地路径元数据：{}", e))?;

        if metadata.file_type().is_symlink() {
            return Err(format!("{} 已是符号链接，请先点击“卸载 / 断开链接”", local_path));
        }

        if metadata.is_dir() {
            // 必须为空文件夹；清空后再建立链接
            let mut entries = std::fs::read_dir(local)
                .map_err(|e| format!("无法读取本地文件夹：{}", e))?;
            if entries.next().is_some() {
                return Err(format!(
                    "{} 不是空文件夹，请清空后再挂载",
                    local_path
                ));
            }
            std::fs::remove_dir(local).map_err(|e| format!("无法清空本地文件夹：{}", e))?;
        } else {
            return Err(format!("{} 已存在且不是文件夹", local_path));
        }
    } else {
        // 路径不存在时，其上级目录必须已经存在（mklink 本身不会创建中间目录）
        if let Some(parent) = local.parent() {
            if !parent.exists() {
                return Err(format!(
                    "本地路径的上级文件夹不存在：{}",
                    parent.display()
                ));
            }
        }
    }

    // 关键：mklink 是 cmd.exe 内置命令，绝不能直接 Command::new("mklink")
    let output = std::process::Command::new("cmd")
        .args(["/c", "mklink", "/d", &local_path, &unc_path])
        .output()
        .map_err(|e| format!("调用 mklink 失败：{}", e))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        Err(if err.is_empty() {
            "mklink 执行失败，可能未获得管理员权限或远程共享不可达".to_string()
        } else {
            err
        })
    }
}

/* -------------------------------------------------------------------------- */
/*  卸载：仅删除本地符号链接目录，不影响远程 SMB 共享                          */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn unmount(local_path: String) -> Result<String, String> {
    let local = std::path::Path::new(&local_path);

    if !local.exists() {
        return Err(format!("{} 不存在，无需卸载", local_path));
    }

    let metadata = std::fs::symlink_metadata(local)
        .map_err(|e| format!("无法读取元数据：{}", e))?;

    if !metadata.file_type().is_symlink() {
        return Err(format!(
            "{} 不是符号链接，出于安全考虑拒绝删除非链接目录",
            local_path
        ));
    }

    // remove_dir 对符号链接目录只删除链接本身，不删除远程目标
    std::fs::remove_dir(local)
        .map(|_| format!("已卸载：{}", local_path))
        .map_err(|e| format!("卸载失败：{}", e))
}

/* -------------------------------------------------------------------------- */
/*  检查本地路径是否已是符号链接，返回链接指向的目标                          */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn check_link(local_path: String) -> Result<Option<String>, String> {
    let local = std::path::Path::new(&local_path);

    if !local.exists() {
        return Ok(None);
    }

    let metadata = std::fs::symlink_metadata(local)
        .map_err(|e| format!("无法读取元数据：{}", e))?;

    if !metadata.file_type().is_symlink() {
        return Ok(None);
    }

    // 读取符号链接指向的目标
    match std::fs::read_link(local) {
        Ok(target) => Ok(Some(target.display().to_string())),
        Err(e) => Err(format!("无法读取符号链接目标：{}", e)),
    }
}

/* -------------------------------------------------------------------------- */
/*  Windows 凭据管理器（Credential Manager）读写                               */
/*  基于 advapi32.dll 的 CredWriteW / CredDeleteW                             */
/* -------------------------------------------------------------------------- */
#[link(name = "advapi32", kind = "dylib")]
extern "system" {
    fn CredWriteW(credential: *const CREDENTIALW, flags: u32) -> i32;
    fn CredDeleteW(target_name: *const u16, flags: u32, reserved: u32) -> i32;
    fn CredFree(buffer: *mut c_void);
}

const CRED_TYPE_GENERIC: u32 = 1;
const CRED_PERSIST_LOCAL_MACHINE: u32 = 2;

#[repr(C)]
struct FILETIME {
    dw_low_date_time: u32,
    dw_high_date_time: u32,
}

#[repr(C)]
struct CREDENTIALW {
    flags: u32,
    type_: u32,
    target_name: *mut u16,
    comment: *mut u16,
    last_written: FILETIME,
    credential_blob_size: u32,
    credential_blob: *mut u8,
    persist: u32,
    attribute_count: u32,
    attributes: *mut c_void,
    target_alias: *mut u16,
    username: *mut u16,
}

/// 将 Rust 字符串转换为以空结尾的 UTF-16 宽字符数组
fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(Some(0)).collect()
}

#[tauri::command]
fn save_credential(target: String, username: String, password: String) -> Result<(), String> {
    let target_w = to_wide(&target);
    let user_w = to_wide(&username);
    let pass_w = to_wide(&password);

    // 密码以 UTF-16 字节形式存入 credential_blob
    let blob_size = (pass_w.len() * 2) as u32;

    let mut cred = CREDENTIALW {
        flags: 0,
        type_: CRED_TYPE_GENERIC,
        target_name: target_w.as_ptr() as *mut u16,
        comment: ptr::null_mut(),
        last_written: FILETIME {
            dw_low_date_time: 0,
            dw_high_date_time: 0,
        },
        credential_blob_size: blob_size,
        credential_blob: pass_w.as_ptr() as *mut u8,
        persist: CRED_PERSIST_LOCAL_MACHINE,
        attribute_count: 0,
        attributes: ptr::null_mut(),
        target_alias: ptr::null_mut(),
        username: user_w.as_ptr() as *mut u16,
    };

    let ok = unsafe { CredWriteW(&mut cred as *mut CREDENTIALW, 0) };
    if ok == 1 {
        Ok(())
    } else {
        Err(format!(
            "写入凭据管理器失败，系统错误代码：{}",
            std::io::Error::last_os_error()
        ))
    }
}

#[tauri::command]
fn delete_credential(target: String) -> Result<(), String> {
    let target_w = to_wide(&target);
    let ok = unsafe { CredDeleteW(target_w.as_ptr(), 0, 0) };
    if ok == 1 {
        Ok(())
    } else {
        Err(format!(
            "删除凭据失败，系统错误代码：{}",
            std::io::Error::last_os_error()
        ))
    }
}
