// build.rs — Finds Qt 6, compiles the C++ OpenGL widget, links Qt modules.
//
// Detection order:
//   1. Environment variables QT_INCLUDE_DIR / QT_LIB_DIR  (always wins)
//   2. pkg-config Qt6Widgets (Linux / WSL — needs: apt install qt6-base-dev qt6-opengl-dev)
//   3. qmake6 / qmake --query    (Windows Qt installer, Qt Creator installations)
//   4. Hard-coded common paths   (last resort)
//
// WSL quick start:
//   sudo apt update && sudo apt install -y \
//     qt6-base-dev qt6-opengl-dev libqt6opengl6-dev \
//     libgl1-mesa-dev libglu1-mesa-dev
//
// Windows quick start:
//   Install Qt 6 from https://www.qt.io/download-qt-installer
//   Add  C:\Qt\6.x.x\msvc2022_64\bin  to PATH so qmake is found.

use std::env;
use std::process::Command;

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn run(prog: &str, args: &[&str]) -> Option<String> {
    Command::new(prog)
        .args(args)
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .filter(|s| !s.is_empty() && !s.starts_with("**"))
}

fn pkg_config_var(pkg: &str, flag: &str) -> Option<String> {
    run("pkg-config", &[pkg, flag])
}

// ─── Qt path discovery ────────────────────────────────────────────────────────

struct QtPaths {
    include: String,
    lib:     String,
}

fn find_qt() -> QtPaths {
    // 1. Explicit env vars
    if let (Ok(inc), Ok(lib)) = (env::var("QT_INCLUDE_DIR"), env::var("QT_LIB_DIR")) {
        return QtPaths { include: inc, lib };
    }

    // 2. pkg-config (Linux / WSL)
    if let Some(raw) = pkg_config_var("Qt6Widgets", "--cflags-only-I") {
        // Extract first -I path
        let inc = raw.split_whitespace()
            .find(|s| s.starts_with("-I"))
            .map(|s| s.trim_start_matches("-I").to_string())
            // drop the trailing "/QtWidgets" component to get the root include dir
            .map(|s| {
                let p = std::path::Path::new(&s);
                p.parent()
                    .map(|pp| pp.to_string_lossy().to_string())
                    .unwrap_or(s)
            });
        if let Some(inc) = inc {
            let lib = pkg_config_var("Qt6Widgets", "--libs-only-L")
                .unwrap_or_default()
                .split_whitespace()
                .find(|s| s.starts_with("-L"))
                .map(|s| s.trim_start_matches("-L").to_string())
                .unwrap_or_default();
            return QtPaths { include: inc, lib };
        }
    }

    // 3. qmake
    for prog in &["qmake6", "qmake"] {
        if let (Some(inc), Some(lib)) = (
            run(prog, &["--query", "QT_INSTALL_HEADERS"]),
            run(prog, &["--query", "QT_INSTALL_LIBS"]),
        ) {
            return QtPaths { include: inc, lib };
        }
    }

    // 4. Common hard-coded paths
    let candidates = [
        ("/usr/include/qt6",                  "/usr/lib"),
        ("/usr/include/x86_64-linux-gnu/qt6", "/usr/lib/x86_64-linux-gnu"),
        ("/usr/local/include/qt6",             "/usr/local/lib"),
    ];
    for (inc, lib) in &candidates {
        if std::path::Path::new(inc).exists() {
            return QtPaths { include: inc.to_string(), lib: lib.to_string() };
        }
    }

    panic!(
        "\n\
        ╔══════════════════════════════════════════════════════════════╗\n\
        ║  Qt 6 not found!  Please do one of:                         ║\n\
        ║                                                              ║\n\
        ║  Linux/WSL:                                                  ║\n\
        ║    sudo apt install qt6-base-dev qt6-opengl-dev              ║\n\
        ║                      libqt6opengl6-dev libgl1-mesa-dev       ║\n\
        ║                                                              ║\n\
        ║  Windows (after Qt installer):                               ║\n\
        ║    add  <Qt>\\<ver>\\msvc2022_64\\bin  to PATH                 ║\n\
        ║                                                              ║\n\
        ║  Or set env vars:  QT_INCLUDE_DIR  and  QT_LIB_DIR          ║\n\
        ╚══════════════════════════════════════════════════════════════╝"
    );
}

// ─── Main ─────────────────────────────────────────────────────────────────────

fn main() {
    let qt = find_qt();

    // ── Compile C++ shim + cxx bridge ─────────────────────────────────────────
    // Qt headers live in per-module subdirs: <root>/QtWidgets/QApplication etc.
    cxx_build::bridge("src/client/bridge.rs")
        .file("cpp/vqwidget.cpp")
        .include(&qt.include)
        .include(format!("{}/QtCore", qt.include))
        .include(format!("{}/QtGui", qt.include))
        .include(format!("{}/QtWidgets", qt.include))
        .include(format!("{}/QtOpenGL", qt.include))
        .include(format!("{}/QtOpenGLWidgets", qt.include))
        .include("cpp")
        .std("c++17")
        .flag_if_supported("-fPIC")
        // silence Qt macro warnings when QT_NO_KEYWORDS is defined
        .define("QT_NO_KEYWORDS", None)
        .compile("vqwidget");

    // ── Link Qt modules ───────────────────────────────────────────────────────
    if !qt.lib.is_empty() {
        println!("cargo:rustc-link-search=native={}", qt.lib);
    }

    let target_os = env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let (prefix, suffix) = match target_os.as_str() {
        "windows" => ("",   ""),
        _         => ("",   ""),
    };

    for module in &["Qt6Core", "Qt6Gui", "Qt6Widgets", "Qt6OpenGL", "Qt6OpenGLWidgets"] {
        println!("cargo:rustc-link-lib={}{}{}", prefix, module, suffix);
    }

    // Platform extras
    match target_os.as_str() {
        "windows" => {
            println!("cargo:rustc-link-lib=opengl32");
            println!("cargo:rustc-link-lib=user32");
            println!("cargo:rustc-link-lib=gdi32");
        }
        "linux" => {
            println!("cargo:rustc-link-lib=GL");
        }
        _ => {}
    }

    // ── Rebuild triggers ──────────────────────────────────────────────────────
    for f in &[
        "src/client/bridge.rs",
        "cpp/vqwidget.h",
        "cpp/vqwidget.cpp",
    ] {
        println!("cargo:rerun-if-changed={}", f);
    }
    for v in &["QT_INCLUDE_DIR", "QT_LIB_DIR"] {
        println!("cargo:rerun-if-env-changed={}", v);
    }
}
