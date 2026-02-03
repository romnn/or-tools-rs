use std::path::{Path, PathBuf};

const ORTOOLS_VERSION: &str = "9.15";
const ORTOOLS_PREBUILT_BUILD: &str = "6755";

fn ortools_sys_cache_dir() -> Option<PathBuf> {
    std::env::var("OR_TOOLS_SYS_CACHE_DIR")
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .map(PathBuf::from)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-changed=src/cp_sat_wrapper.cpp");
    println!("cargo:rerun-if-env-changed=ORTOOLS_PREFIX");
    println!("cargo:rerun-if-env-changed=ORTOOL_PREFIX");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_STATIC");
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_BUILD_FROM_SOURCE");
    println!("cargo:rerun-if-env-changed=OR_TOOLS_SYS_SOURCE_DIR");
    println!("cargo:rerun-if-env-changed=OR_TOOLS_SYS_PREBUILT_VERSION");
    println!("cargo:rerun-if-env-changed=OR_TOOLS_SYS_PREBUILT_BUILD");
    println!("cargo:rerun-if-env-changed=OR_TOOLS_SYS_BACKEND");
    println!("cargo:rerun-if-env-changed=OR_TOOLS_SYS_CACHE_DIR");

    let target_os = std::env::var("CARGO_CFG_TARGET_OS")?;

    let (include_dir, lib_dir, emit_rpath, link_static) = resolve_ortools()?;

    cc::Build::new()
        .cpp(true)
        .warnings(false)
        .extra_warnings(false)
        .flags(["-std=c++17", "-DOR_PROTO_DLL="])
        .file("src/cp_sat_wrapper.cpp")
        .include(&include_dir)
        .compile("or_tools_cp_sat_wrapper");

    if target_os == "macos" {
        println!("cargo:rustc-link-lib=dylib=c++");
    } else if target_os == "linux" {
        println!("cargo:rustc-link-lib=dylib=stdc++");
        if link_static {
            println!("cargo:rustc-link-arg=-static-libstdc++");
            println!("cargo:rustc-link-arg=-static-libgcc");
        }
    }

    let link_kind = if link_static { "static" } else { "dylib" };

    println!("cargo:rustc-link-lib={link_kind}=ortools");

    // OR-Tools' static builds don't always provide static archives for every
    // dependency. Prefer static linking, but fall back to dynamic when needed.
    let protobuf_kind = if link_static && !lib_dir.join("libprotobuf.a").is_file() {
        "dylib"
    } else {
        link_kind
    };
    let protobuf_lite_kind = if link_static && !lib_dir.join("libprotobuf-lite.a").is_file() {
        "dylib"
    } else {
        link_kind
    };

    println!("cargo:rustc-link-lib={protobuf_kind}=protobuf");
    println!("cargo:rustc-link-lib={protobuf_lite_kind}=protobuf-lite");

    if link_static {
        emit_static_dep_links(&lib_dir)?;

        // OR-Tools static builds typically still require system libs.
        // We do not attempt full glibc-static linking.
        println!("cargo:rustc-link-lib=dylib=pthread");
        println!("cargo:rustc-link-lib=dylib=dl");
        println!("cargo:rustc-link-lib=dylib=m");
    }

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    if emit_rpath {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
    }

    println!(
        "cargo:metadata=ortools_include_dir={}",
        include_dir.display()
    );
    println!("cargo:metadata=ortools_lib_dir={}", lib_dir.display());
    println!("cargo:metadata=ortools_link_static={link_static}");

    Ok(())
}

fn emit_static_dep_links(lib_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    // Protobuf built by OR-Tools depends on Abseil (and other helper archives). When we
    // link protobuf statically, we must also link these dependent archives.
    //
    // We discover them from the OR-Tools install prefix and emit them in a stable order.
    let mut absl_libs = Vec::new();
    for entry in std::fs::read_dir(lib_dir)? {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };

        if let Some(name) = file_name
            .strip_prefix("libabsl_")
            .and_then(|s| s.strip_suffix(".a"))
        {
            absl_libs.push(format!("absl_{name}"));
        }
    }
    absl_libs.sort();

    if absl_libs.is_empty() {
        // Some OR-Tools "static" builds still install Abseil as shared libraries.
        // In that case, link the Abseil components dynamically.
        let mut absl_dylibs = Vec::new();
        for entry in std::fs::read_dir(lib_dir)? {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }

            let Some(file_name) = path.file_name().and_then(|s| s.to_str()) else {
                continue;
            };

            let suffix = if std::path::Path::new(file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("so"))
            {
                ".so"
            } else if std::path::Path::new(file_name)
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("dylib"))
            {
                ".dylib"
            } else {
                continue;
            };

            if let Some(name) = file_name
                .strip_prefix("libabsl_")
                .and_then(|s| s.strip_suffix(suffix))
            {
                absl_dylibs.push(format!("absl_{name}"));
            }
        }
        absl_dylibs.sort();
        absl_dylibs.dedup();
        for lib in absl_dylibs {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    } else {
        for lib in absl_libs {
            println!("cargo:rustc-link-lib=static={lib}");
        }
    }

    // Other dependency archives built/installed by OR-Tools that may be needed for
    // fully-static OR-Tools+protobuf linking.
    for lib in ["re2", "utf8_range", "utf8_validity", "upb", "z", "bz2"] {
        let static_archive = lib_dir.join(format!("lib{lib}.a"));
        if static_archive.is_file() {
            println!("cargo:rustc-link-lib=static={lib}");
            continue;
        }

        // Some deps (notably bz2) may only be available as shared objects even
        // when OR-Tools itself is built as static. In that case we still need
        // to link them dynamically to satisfy symbols.
        if glob_exists(lib_dir, &format!("lib{lib}.so"))?
            || glob_exists(lib_dir, &format!("lib{lib}.dylib"))?
        {
            println!("cargo:rustc-link-lib=dylib={lib}");
        }
    }

    Ok(())
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
enum Backend {
    System,
    VendorPrebuilt,
    BuildFromSource,
}

fn resolve_ortools() -> Result<(PathBuf, PathBuf, bool, bool), Box<dyn std::error::Error>> {
    let feature_build_from_source = std::env::var("CARGO_FEATURE_BUILD_FROM_SOURCE").is_ok();
    let feature_vendor_prebuilt = std::env::var("CARGO_FEATURE_VENDOR_PREBUILT").is_ok();
    let feature_system = std::env::var("CARGO_FEATURE_SYSTEM").is_ok();
    let feature_static = std::env::var("CARGO_FEATURE_STATIC").is_ok();

    let backend = match std::env::var("OR_TOOLS_SYS_BACKEND").as_deref() {
        Ok("system") => Backend::System,
        Ok("vendor-prebuilt") => Backend::VendorPrebuilt,
        Ok("build-from-source") => Backend::BuildFromSource,
        Ok(other) => {
            return Err(format!(
                "invalid OR_TOOLS_SYS_BACKEND={other:?} (expected system|vendor-prebuilt|build-from-source)"
            )
            .into());
        }
        Err(_) => {
            // Multiple backends may be enabled (e.g. via dependency defaults +
            // explicit feature selection). Prefer an explicitly-requested backend
            // over defaults.
            if feature_system {
                Backend::System
            } else if feature_build_from_source {
                Backend::BuildFromSource
            } else if feature_vendor_prebuilt {
                Backend::VendorPrebuilt
            } else {
                Backend::System
            }
        }
    };

    let link_static = feature_static && backend == Backend::BuildFromSource;

    let prefix = match backend {
        Backend::BuildFromSource => build_ortools_from_source(link_static)?,
        Backend::VendorPrebuilt => download_ortools_prebuilt()?,
        Backend::System => std::env::var("ORTOOLS_PREFIX")
            .or_else(|_| std::env::var("ORTOOL_PREFIX"))
            .unwrap_or_else(|_| "/opt/ortools".into()),
    };

    let include_dir = PathBuf::from(&prefix).join("include");
    let lib_dir = ortools_lib_dir(&prefix);

    let emit_rpath = backend != Backend::System;
    Ok((include_dir, lib_dir, emit_rpath, link_static))
}

fn ortools_lib_dir(prefix: &str) -> PathBuf {
    let lib = PathBuf::from(format!("{prefix}/lib"));
    if lib.is_dir() {
        lib
    } else {
        PathBuf::from(format!("{prefix}/lib64"))
    }
}

fn workspace_root() -> Result<PathBuf, Box<dyn std::error::Error>> {
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?);
    Ok(manifest_dir
        .parent()
        .and_then(|p| p.parent())
        .ok_or("failed to locate workspace root")?
        .to_path_buf())
}

fn build_ortools_from_source(link_static: bool) -> Result<String, Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    let source_dir = prepare_ortools_source_dir()?;
    let source_dir = if link_static {
        prepare_patchable_source_dir_for_static(&source_dir, &out_dir)?
    } else {
        source_dir
    };

    if link_static {
        patch_downloaded_ortools_deps_for_static(&source_dir)?;
    }

    let install_dir = out_dir.join("or_tools_install");

    let mut cfg = cmake::Config::new(&source_dir);
    // Building OR-Tools in Debug mode is extremely slow and produces huge artifacts.
    // This is an internal dependency, so a Release build is acceptable.
    cfg.profile("Release");
    cfg.define("BUILD_CXX", "ON");
    cfg.define("BUILD_PYTHON", "OFF");
    cfg.define("BUILD_JAVA", "OFF");
    cfg.define("BUILD_DOTNET", "OFF");
    cfg.define("BUILD_DOC", "OFF");
    cfg.define("BUILD_TESTING", "OFF");
    cfg.define("BUILD_CXX_SAMPLES", "OFF");
    cfg.define("BUILD_CXX_EXAMPLES", "OFF");
    cfg.define("BUILD_CXX_DOC", "OFF");

    cfg.define("BUILD_SAMPLES", "OFF");
    cfg.define("BUILD_EXAMPLES", "OFF");

    // Keep the build self-contained: build all dependencies from source.
    // This is important for static builds and also avoids relying on system protobuf.
    cfg.define("BUILD_DEPS", "ON");

    // Lean defaults: disable optional components unless explicitly enabled.
    // OR-Tools' upstream CMake currently assumes MathOpt proto targets exist
    // (e.g. gurobi integration links against them). This means MathOpt is not
    // fully optional for a source build.
    let build_mathiopt = true;
    let build_flatzinc = std::env::var("CARGO_FEATURE_FLATZINC").is_ok();
    cfg.define("BUILD_MATH_OPT", if build_mathiopt { "ON" } else { "OFF" });
    cfg.define("BUILD_FLATZINC", if build_flatzinc { "ON" } else { "OFF" });

    // Optional third-party solvers: default OFF for CP-SAT-focused usage.
    // Note: Some solvers are "OFF not supported" upstream (e.g. GLOP/BOP), so we
    // avoid trying to disable those here.
    let use_coinor = std::env::var("CARGO_FEATURE_SOLVER_COINOR").is_ok();
    let use_highs = std::env::var("CARGO_FEATURE_SOLVER_HIGHS").is_ok();
    let use_pdlp = std::env::var("CARGO_FEATURE_SOLVER_PDLP").is_ok();
    let use_scip = std::env::var("CARGO_FEATURE_SOLVER_SCIP").is_ok();
    let use_glpk = std::env::var("CARGO_FEATURE_SOLVER_GLPK").is_ok();

    cfg.define("USE_COINOR", if use_coinor { "ON" } else { "OFF" });
    cfg.define("USE_HIGHS", if use_highs { "ON" } else { "OFF" });
    cfg.define("USE_PDLP", if use_pdlp { "ON" } else { "OFF" });
    cfg.define("USE_SCIP", if use_scip { "ON" } else { "OFF" });
    cfg.define("USE_GLPK", if use_glpk { "ON" } else { "OFF" });
    cfg.define("USE_CPLEX", "OFF");
    // These are ON-by-default upstream and are dynamically loaded.
    // Keep them ON explicitly to avoid stale CMakeCache values from older builds
    // and because upstream configuration assumes these targets/symbols exist.
    cfg.define("USE_GUROBI", "ON");
    cfg.define("USE_XPRESS", "ON");

    if link_static {
        cfg.define("BUILD_SHARED_LIBS", "OFF");
        cfg.define("OR_TOOLS_SYS_FORCE_STATIC_DEPS", "ON");
    } else {
        cfg.define("BUILD_SHARED_LIBS", "ON");
        cfg.define("OR_TOOLS_SYS_FORCE_STATIC_DEPS", "OFF");
    }

    // OR-Tools defaults to C++17 on non-MSVC; keep this aligned for compatibility.
    cfg.define("CMAKE_CXX_STANDARD", "17");
    cfg.define("CMAKE_POSITION_INDEPENDENT_CODE", "ON");
    cfg.define("CMAKE_INSTALL_PREFIX", &install_dir);

    let _dst = cfg.build();

    Ok(install_dir.to_string_lossy().to_string())
}

fn prepare_patchable_source_dir_for_static(
    source_dir: &Path,
    out_dir: &Path,
) -> Result<PathBuf, Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let vendored_source_dir = workspace_root.join("vendor/or-tools").canonicalize()?;
    let source_dir = source_dir.canonicalize()?;

    if source_dir != vendored_source_dir {
        return Ok(source_dir);
    }

    let scratch_dir = out_dir.join("or_tools_source_scratch");
    let marker = scratch_dir.join(".or-tools-sys-version");
    let expected_header = scratch_dir.join("ortools/sat/cp_model.h");

    if scratch_dir.is_dir() {
        let marker_version = std::fs::read_to_string(&marker)
            .ok()
            .map(|s| s.trim().to_string());
        if expected_header.is_file()
            && marker_version
                .as_deref()
                .is_some_and(|v| v == ORTOOLS_VERSION)
        {
            return Ok(scratch_dir);
        }

        std::fs::remove_dir_all(&scratch_dir)?;
    }

    copy_dir_all(&source_dir, &scratch_dir)?;
    std::fs::write(&marker, ORTOOLS_VERSION)?;
    Ok(scratch_dir)
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), Box<dyn std::error::Error>> {
    std::fs::create_dir_all(dst)?;

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let file_name = entry.file_name();
        if file_name.to_string_lossy() == ".git" {
            continue;
        }

        let src_path = entry.path();
        let dst_path = dst.join(&file_name);
        let ty = entry.file_type()?;
        if ty.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else if ty.is_file() {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

fn patch_downloaded_ortools_deps_for_static(
    source_dir: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let workspace_root = workspace_root()?;
    let vendored_source_dir = workspace_root.join("vendor/or-tools").canonicalize()?;
    let source_dir = source_dir.canonicalize()?;

    if source_dir == vendored_source_dir {
        return Ok(());
    }

    let marker = source_dir.join(".or-tools-sys-version");
    let safe_to_patch = marker.is_file() || !source_dir.starts_with(&workspace_root);
    if !safe_to_patch {
        return Ok(());
    }

    let deps_cmake = source_dir.join("cmake/dependencies/CMakeLists.txt");
    if !deps_cmake.is_file() {
        return Ok(());
    }

    let original = std::fs::read_to_string(&deps_cmake)?;
    if original.contains("OR_TOOLS_SYS_FORCE_STATIC_DEPS") {
        return Ok(());
    }

    let mut out = String::with_capacity(original.len() + 256);
    for line in original.lines() {
        let trimmed = line.trim();

        out.push_str(line);
        out.push('\n');

        if trimmed == "set(FETCHCONTENT_UPDATES_DISCONNECTED ON)" {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            out.push_str(indent);
            out.push_str("if(OR_TOOLS_SYS_FORCE_STATIC_DEPS)\n");
            out.push_str(indent);
            out.push_str("  set(BUILD_SHARED_LIBS OFF)\n");
            out.push_str(indent);
            out.push_str("  set(protobuf_BUILD_SHARED_LIBS OFF)\n");
            out.push_str(indent);
            out.push_str("endif()\n");
        }

        if trimmed == "set(BUILD_SHARED_LIBS ON)" {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            out.truncate(out.len().saturating_sub(line.len() + 1));
            out.push_str(indent);
            out.push_str("if(NOT OR_TOOLS_SYS_FORCE_STATIC_DEPS)\n");
            out.push_str(indent);
            out.push_str("  set(BUILD_SHARED_LIBS ON)\n");
            out.push_str(indent);
            out.push_str("endif()\n");
            continue;
        }
        if trimmed == "set(protobuf_BUILD_SHARED_LIBS ON)" {
            let indent_len = line.len() - line.trim_start().len();
            let indent = &line[..indent_len];
            out.truncate(out.len().saturating_sub(line.len() + 1));
            out.push_str(indent);
            out.push_str("if(NOT OR_TOOLS_SYS_FORCE_STATIC_DEPS)\n");
            out.push_str(indent);
            out.push_str("  set(protobuf_BUILD_SHARED_LIBS ON)\n");
            out.push_str(indent);
            out.push_str("endif()\n");
        }
    }

    std::fs::write(&deps_cmake, out)?;
    println!(
        "cargo:warning=or-tools-sys: patched downloaded OR-Tools cmake/dependencies/CMakeLists.txt to avoid forcing shared libs"
    );
    Ok(())
}

fn download_ortools_prebuilt() -> Result<String, Box<dyn std::error::Error>> {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR")?);

    let ortools_version = std::env::var("OR_TOOLS_SYS_PREBUILT_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| ORTOOLS_VERSION.to_string());
    let ortools_build = std::env::var("OR_TOOLS_SYS_PREBUILT_BUILD")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| ORTOOLS_PREBUILT_BUILD.to_string());

    let target = std::env::var("TARGET")?;
    let os = std::env::var("CARGO_CFG_TARGET_OS")?;
    let arch = std::env::var("CARGO_CFG_TARGET_ARCH")?;

    let asset = match (os.as_str(), arch.as_str()) {
        ("linux", "x86_64") => {
            format!("or-tools_amd64_ubuntu-24.04_cpp_v{ortools_version}.{ortools_build}.tar.gz")
        }
        ("macos", "aarch64") => {
            format!("or-tools_arm64_macOS-26.2_cpp_v{ortools_version}.{ortools_build}.tar.gz")
        }
        _ => {
            return Err(
                format!("or-tools-sys vendor-prebuilt does not support target {target}").into(),
            );
        }
    };

    let url =
        format!("https://github.com/google/or-tools/releases/download/v{ortools_version}/{asset}");

    let download_dir = if let Some(cache_dir) = ortools_sys_cache_dir() {
        cache_dir
            .join("or_tools_vendor_prebuilt")
            .join(&target)
            .join(format!("v{ortools_version}.{ortools_build}"))
    } else {
        out_dir.join("or_tools_vendor_prebuilt")
    };
    let tarball = download_dir.join(&asset);
    let extract_dir = download_dir.join("_extract");
    let prefix_marker = download_dir.join("PREFIX_DIR");

    std::fs::create_dir_all(&download_dir)?;

    if let Ok(prefix) = std::fs::read_to_string(&prefix_marker) {
        let prefix = prefix.trim();
        if !prefix.is_empty() && Path::new(prefix).is_dir() {
            return Ok(prefix.to_string());
        }
    }

    if !tarball.is_file() {
        download(&url, &tarball)?;
    }

    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;
    extract_tgz(&tarball, &extract_dir)?;

    let prefix = find_first_dir(&extract_dir)?;

    let include = prefix.join("include/ortools/sat/cp_model.h");
    if !include.is_file() {
        return Err("vendored OR-Tools extraction missing include/ortools/sat/cp_model.h".into());
    }

    let lib_dir = if prefix.join("lib").is_dir() {
        prefix.join("lib")
    } else {
        prefix.join("lib64")
    };

    if os == "linux" {
        if !glob_exists(&lib_dir, "libortools.so")? && !glob_exists(&lib_dir, "libortools.a")? {
            return Err("vendored OR-Tools extraction missing libortools".into());
        }
    } else if !glob_exists(&lib_dir, "libortools.dylib")? && !glob_exists(&lib_dir, "libortools.a")?
    {
        return Err("vendored OR-Tools extraction missing libortools".into());
    }

    let prefix_str = prefix.to_string_lossy().to_string();
    std::fs::write(prefix_marker, &prefix_str)?;
    Ok(prefix_str)
}

fn download(url: &str, out: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let tmp = out.with_extension("tmp");
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }

    let status = std::process::Command::new("curl")
        .args([
            "-L",
            "--fail",
            "--retry",
            "10",
            "--retry-connrefused",
            "--retry-all-errors",
            "--retry-delay",
            "2",
            "-o",
        ])
        .arg(&tmp)
        .arg(url)
        .status();

    match status {
        Ok(s) if s.success() => {}
        Ok(_) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(format!("failed to download {url}").into());
        }
        Err(e) => {
            let _ = std::fs::remove_file(&tmp);
            return Err(e.into());
        }
    }

    std::fs::rename(tmp, out)?;
    Ok(())
}

fn extract_tgz(tarball: &Path, extract_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let status = std::process::Command::new("tar")
        .args(["-xzf"])
        .arg(tarball)
        .args(["-C"])
        .arg(extract_dir)
        .status()?;
    if !status.success() {
        return Err("failed to extract OR-Tools tarball".into());
    }
    Ok(())
}

fn find_first_dir(dir: &Path) -> Result<PathBuf, Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            return Ok(path);
        }
    }
    Err("failed to locate extracted OR-Tools directory".into())
}

fn prepare_ortools_source_dir() -> Result<PathBuf, Box<dyn std::error::Error>> {
    if let Ok(p) = std::env::var("OR_TOOLS_SYS_SOURCE_DIR") {
        Ok(normalize_windows_path(PathBuf::from(p).canonicalize()?))
    } else if cfg!(windows) {
        let source_dir = if let Ok(tmp) = std::env::var("RUNNER_TEMP") {
            PathBuf::from(tmp).join("or_tools_source_dir")
        } else {
            std::env::temp_dir().join("or_tools_source_dir")
        };
        ensure_ortools_source_present(&source_dir)?;
        Ok(normalize_windows_path(source_dir.canonicalize()?))
    } else {
        let source_dir = workspace_root()?.join("vendor/or-tools");

        // On non-Windows we rely on the repository's vendored OR-Tools checkout.
        // Do not try to delete/replace it at build time.
        let expected_header = source_dir.join("ortools/sat/cp_model.h");
        if !expected_header.is_file() {
            return Err(
                "vendored OR-Tools source missing ortools/sat/cp_model.h (did you forget to fetch submodules?)"
                    .into(),
            );
        }

        Ok(source_dir.canonicalize()?)
    }
}

fn normalize_windows_path(path: PathBuf) -> PathBuf {
    #[cfg(windows)]
    {
        let s = path.to_string_lossy();
        if let Some(stripped) = s.strip_prefix(r"\\?\UNC\") {
            return PathBuf::from(format!(r"\\{stripped}"));
        }
        if let Some(stripped) = s.strip_prefix(r"\\?\") {
            return PathBuf::from(stripped);
        }
    }
    path
}

fn ensure_ortools_source_present(source_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let marker = source_dir.join(".or-tools-sys-version");

    let expected_header = source_dir.join("ortools/sat/cp_model.h");
    let expected_version = ORTOOLS_VERSION;

    if source_dir.is_dir() {
        let marker_version = std::fs::read_to_string(&marker)
            .ok()
            .map(|s| s.trim().to_string());

        let version_matches = marker_version
            .as_deref()
            .is_some_and(|v| v == expected_version);

        if expected_header.is_file() && version_matches {
            return Ok(());
        }

        std::fs::remove_dir_all(source_dir)?;
    }

    let vendor_dir = if let Some(cache_dir) = ortools_sys_cache_dir() {
        cache_dir.join("or-tools-sys")
    } else if cfg!(windows) {
        if let Ok(tmp) = std::env::var("RUNNER_TEMP") {
            PathBuf::from(tmp).join("or-tools-sys")
        } else {
            source_dir
                .parent()
                .ok_or("failed to locate vendor directory")?
                .to_path_buf()
        }
    } else {
        source_dir
            .parent()
            .ok_or("failed to locate vendor directory")?
            .to_path_buf()
    };
    std::fs::create_dir_all(&vendor_dir)?;

    let url =
        format!("https://github.com/google/or-tools/archive/refs/tags/v{expected_version}.tar.gz");
    let asset = format!("or-tools-src-v{expected_version}.tar.gz");

    let download_dir = vendor_dir.join("or_tools_source");
    let tarball = download_dir.join(&asset);
    let extract_dir = download_dir.join("_extract");

    std::fs::create_dir_all(&download_dir)?;

    if !tarball.is_file() {
        download(&url, &tarball)?;
    }

    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir)?;
    }
    std::fs::create_dir_all(&extract_dir)?;
    extract_tgz(&tarball, &extract_dir)?;

    let extracted_root = find_first_dir(&extract_dir)?;

    if source_dir.exists() {
        std::fs::remove_dir_all(source_dir)?;
    }

    std::fs::rename(&extracted_root, source_dir)?;

    if !expected_header.is_file() {
        return Err("downloaded OR-Tools source missing ortools/sat/cp_model.h".into());
    }

    std::fs::write(marker, expected_version)?;
    Ok(())
}

fn glob_exists(dir: &Path, prefix: &str) -> Result<bool, Box<dyn std::error::Error>> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.starts_with(prefix) {
            return Ok(true);
        }
    }
    Ok(false)
}
