//! (ours) Gives the executable the KLBF Lab icon (`klbf-lab.rc`, resource 1), which the windows
//! use too. Built with the Windows SDK's `rc.exe` (`RC` names another); without it the build goes
//! on with no icon and one warning.
use std::path::PathBuf;
use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=klbf-lab.rc");
    println!("cargo:rerun-if-changed=klbf-lab.ico");
    println!("cargo:rerun-if-env-changed=RC");
    if !std::env::var("CARGO_CFG_TARGET_ENV").is_ok_and(|v| v == "msvc") {
        return;
    }
    let Some(rc) = rc() else {
        println!("cargo:warning=no rc.exe (Windows SDK): the executable has no KLBF Lab icon");
        return;
    };
    let res = PathBuf::from(std::env::var("OUT_DIR").unwrap()).join("klbf-lab.res");
    match Command::new(&rc).arg("/nologo").arg("/fo").arg(&res).arg("klbf-lab.rc").status() {
        Ok(s) if s.success() => println!("cargo:rustc-link-arg-bins={}", res.display()),
        other => println!("cargo:warning={} failed ({other:?}): no KLBF Lab icon", rc.display()),
    }
}

/// `RC`, else the newest x64 `rc.exe` of the Windows 10/11 SDK.
fn rc() -> Option<PathBuf> {
    if let Some(rc) = std::env::var_os("RC") {
        return Some(rc.into());
    }
    let bin = PathBuf::from(std::env::var_os("ProgramFiles(x86)")?).join("Windows Kits/10/bin");
    let mut found: Vec<PathBuf> = std::fs::read_dir(bin)
        .ok()?
        .flatten()
        .map(|e| e.path().join("x64/rc.exe"))
        .filter(|p| p.is_file())
        .collect();
    found.sort();
    found.pop()
}
