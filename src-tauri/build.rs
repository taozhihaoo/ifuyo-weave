fn main() {
    tauri_build::build();

    // tauri-build 只把 common-controls v6 manifest 嵌入 bin 目标；测试可执行文件
    // 链接的是同一套 wry/tauri 栈，没有 manifest 时加载器会拿 comctl32 v5，
    // 进程启动即失败（STATUS_ENTRYPOINT_NOT_FOUND）。给 test 目标补上 manifest。
    let out_dir = std::env::var("OUT_DIR").expect("OUT_DIR is set by cargo");
    let manifest_path = std::path::Path::new(&out_dir).join("weave-test.manifest");
    std::fs::write(&manifest_path, TEST_MANIFEST).expect("write test manifest");
    // /MANIFESTINPUT 需要显式 /MANIFEST:EMBED（LNK1220）。只作用于 test 目标，
    // 避免干扰 bin（tauri-build 已注入 manifest）与 cdylib 链接。
    println!("cargo:rustc-link-arg-tests=/MANIFEST:EMBED");
    println!(
        "cargo:rustc-link-arg-tests=/MANIFESTINPUT:{}",
        manifest_path.display()
    );
}

const TEST_MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0" processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;
