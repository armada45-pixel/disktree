#[cfg(windows)]
fn main() {
    use std::{env, fs, path::PathBuf, process::Command};

    build_number();

    let icon = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap())
        .join("../../assets/disktree.ico")
        .canonicalize()
        .expect("Windows icon asset is missing");
    let out = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let rc = out.join("disktree.rc");
    let res = out.join("disktree.res");

    // rc.exe treats backslashes inside quoted paths as separators, not escapes.
    let icon_path = icon.to_string_lossy().replace('\\', "/");
    fs::write(&rc, format!("1 ICON \"{icon_path}\"\n"))
        .expect("could not write Windows icon resource script");

    // Cargo can run outside a Visual Studio prompt, where the SDK bin is not
    // on PATH even though the MSVC linker and Windows SDK are installed.
    let rc_exe = env::var_os("DISKTREE_RC").map_or_else(
        || {
            let sdk_bin = env::var_os("WindowsSdkDir")
                .map(PathBuf::from)
                .or_else(|| {
                    env::var_os("ProgramFiles(x86)")
                        .map(|root| PathBuf::from(root).join("Windows Kits/10"))
                })
                .expect("Windows SDK is required to embed the icon")
                .join("bin");
            let host = match env::consts::ARCH {
                "x86_64" => "x64",
                "aarch64" => "arm64",
                "x86" => "x86",
                other => panic!("unsupported Windows build host: {other}"),
            };
            let mut versions: Vec<_> = fs::read_dir(&sdk_bin)
                .expect("Windows SDK bin directory is missing")
                .filter_map(Result::ok)
                .map(|entry| entry.path().join(host).join("rc.exe"))
                .filter(|path| path.is_file())
                .collect();
            versions.sort();
            versions
                .pop()
                .expect("rc.exe is missing from the Windows SDK")
        },
        PathBuf::from,
    );
    let status = Command::new(rc_exe)
        .arg("/nologo")
        .arg(format!("/fo{}", res.display()))
        .arg(&rc)
        .status()
        .expect("could not start rc.exe to embed the Windows icon");
    assert!(
        status.success(),
        "rc.exe failed to compile the Windows icon"
    );

    println!("cargo:rerun-if-changed={}", icon.display());
    println!("cargo:rustc-link-arg-bin=disktree={}", res.display());
}

#[cfg(not(windows))]
fn main() {
    build_number();
}

/// Which build this is, for the settings page: the number of commits it
/// was built from and the last one's short name, `412.6c8d4ce`. A build
/// from a source archive has no history to count and says so.
fn build_number() {
    let git = |args: &[&str]| {
        std::process::Command::new("git")
            .args(args)
            .output()
            .ok()
            .filter(|output| output.status.success())
            .and_then(|output| String::from_utf8(output.stdout).ok())
            .map(|text| text.trim().to_owned())
            .filter(|text| !text.is_empty())
    };
    let build = git(&["rev-list", "--count", "HEAD"])
        .zip(git(&["rev-parse", "--short", "HEAD"]))
        .map_or_else(
            || "unknown".to_owned(),
            |(count, commit)| format!("{count}.{commit}"),
        );
    println!("cargo:rustc-env=DISKTREE_BUILD={build}");
    // Every commit, checkout and merge appends to this log, so the number
    // is never left over from an older one. git is asked where it is: in
    // a worktree it is not under `.git` here. Only a log that exists is
    // named, since cargo takes a missing one as changed on every build.
    if let Some(log) = git(&["rev-parse", "--git-path", "logs/HEAD"])
        .filter(|log| std::path::Path::new(log).exists())
    {
        println!("cargo:rerun-if-changed={log}");
    }
}
