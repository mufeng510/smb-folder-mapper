fn main() {
    let mut windows = tauri_build::WindowsAttributes::new();
    // 嵌入 requireAdministrator 清单（含 Common Controls v6 依赖，dialog 插件需要），
    // 开机后双击 exe 即可触发 UAC 提权，否则 mklink /d 会以“权限不足”失败
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=app.manifest");
        let manifest = std::fs::read_to_string("app.manifest").expect("failed to read app.manifest");
        windows = windows.app_manifest(manifest);
    }
    let attributes = tauri_build::Attributes::new().windows_attributes(windows);
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
