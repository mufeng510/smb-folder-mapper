#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use serde::{Deserialize, Serialize};
use std::ffi::{c_void, OsStr};
use std::fs;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::ptr;

fn app_data_dir() -> Result<PathBuf, String> {
    let appdata = std::env::var("APPDATA").map_err(|e| format!("无法获取 APPDATA：{}", e))?;
    let dir = Path::new(&appdata).join("SMB Folder Mapper");
    fs::create_dir_all(&dir).map_err(|e| format!("无法创建应用数据目录：{}", e))?;
    Ok(dir)
}

fn links_file_path() -> Result<PathBuf, String> {
    Ok(app_data_dir()?.join("links.json"))
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct LinkRecord {
    local_path: String,
    unc_path: String,
}

fn load_links() -> Result<Vec<LinkRecord>, String> {
    let path = links_file_path()?;
    if !path.exists() {
        return Ok(vec![]);
    }
    let content = fs::read_to_string(&path)
        .map_err(|e| format!("无法读取链接记录文件：{}", e))?;
    if content.trim().is_empty() {
        return Ok(vec![]);
    }
    serde_json::from_str::<Vec<LinkRecord>>(&content)
        .map_err(|e| format!("解析链接记录失败：{}", e))
}

fn save_links(links: &[LinkRecord]) -> Result<(), String> {
    let path = links_file_path()?;
    let content = serde_json::to_string_pretty(links)
        .map_err(|e| format!("序列化链接记录失败：{}", e))?;
    fs::write(&path, content).map_err(|e| format!("无法写入链接记录文件：{}", e))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            mount,
            unmount,
            check_link,
            get_links,
            remove_link,
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

    let local = Path::new(&local_path);

    // 校验本地路径
    if local.exists() {
        let metadata = fs::symlink_metadata(local)
            .map_err(|e| format!("无法读取本地路径元数据：{}", e))?;

        if metadata.file_type().is_symlink() {
            return Err(format!("{} 已是符号链接，请先点击“卸载 / 断开链接”", local_path));
        }

        if metadata.is_dir() {
            // 必须为空文件夹；清空后再建立链接
            let mut entries = fs::read_dir(local)
                .map_err(|e| format!("无法读取本地文件夹：{}", e))?;
            if entries.next().is_some() {
                return Err(format!(
                    "{} 不是空文件夹，请清空后再挂载",
                    local_path
                ));
            }
            fs::remove_dir(local).map_err(|e| format!("无法清空本地文件夹：{}", e))?;
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
        // 将链接记录写入本地文件，便于软件再次打开时查看和管理
        let mut links = load_links()?;
        // 避免重复记录同一本地路径
        links.retain(|l| l.local_path != local_path);
        links.push(LinkRecord {
            local_path: local_path.clone(),
            unc_path,
        });
        save_links(&links)?;

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
    let local = Path::new(&local_path);

    if !local.exists() {
        return Err(format!("{} 不存在，无需卸载", local_path));
    }

    let metadata = fs::symlink_metadata(local)
        .map_err(|e| format!("无法读取元数据：{}", e))?;

    if !metadata.file_type().is_symlink() {
        return Err(format!(
            "{} 不是符号链接，出于安全考虑拒绝删除非链接目录",
            local_path
        ));
    }

    // remove_dir 对符号链接目录只删除链接本身，不删除远程目标
    fs::remove_dir(local)
        .map(|_| {
            // 从记录文件中移除
            let _ = remove_link_record(&local_path);
            format!("已卸载：{}", local_path)
        })
        .map_err(|e| format!("卸载失败：{}", e))
}

/// 从记录文件中移除指定本地路径的链接记录（内部使用，不卸载实际链接）
fn remove_link_record(local_path: &str) -> Result<(), String> {
    let mut links = load_links()?;
    links.retain(|l| l.local_path != local_path);
    save_links(&links)
}

/* -------------------------------------------------------------------------- */
/*  检查本地路径是否已是符号链接，返回链接指向的目标                          */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn check_link(local_path: String) -> Result<Option<String>, String> {
    let local = Path::new(&local_path);

    if !local.exists() {
        return Ok(None);
    }

    let metadata = fs::symlink_metadata(local)
        .map_err(|e| format!("无法读取元数据：{}", e))?;

    if !metadata.file_type().is_symlink() {
        return Ok(None);
    }

    // 读取符号链接指向的目标
    match fs::read_link(local) {
        Ok(target) => Ok(Some(target.display().to_string())),
        Err(e) => Err(format!("无法读取符号链接目标：{}", e)),
    }
}

/* -------------------------------------------------------------------------- */
/*  获取所有已记录的链接列表                                                    */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn get_links() -> Result<Vec<LinkRecord>, String> {
    load_links()
}

/* -------------------------------------------------------------------------- */
/*  删除指定的链接记录并卸载符号链接                                            */
/* -------------------------------------------------------------------------- */
#[tauri::command]
fn remove_link(local_path: String) -> Result<String, String> {
    // 先尝试卸载实际符号链接；链接可能已被外部删除，此时只清理记录
    match unmount(local_path.clone()) {
        Ok(_) => {}
        Err(e) => eprintln!("remove_link: 卸载失败（继续清理记录）：{}", e),
    }
    // 再从记录中移除（unmount 内部已调用 remove_link_record，这里再调用一次也是幂等的）
    let _ = remove_link_record(&local_path);
    Ok(format!("已删除链接记录：{}", local_path))
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
