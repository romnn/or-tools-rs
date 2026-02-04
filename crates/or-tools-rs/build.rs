use std::io::{Read, Write};

const DEFAULT_ORTOOLS_VERSION: &str = "9.15";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=OR_TOOLS_PROTO_VERSION");
    println!("cargo:rerun-if-env-changed=DEP_OR_TOOLS_ORTOOLS_VERSION");
    println!("cargo:rerun-if-env-changed=DOCS_RS");
    println!("cargo:rerun-if-env-changed=CARGO_WORKSPACE_DIR");

    let out_dir = std::path::PathBuf::from(std::env::var("OUT_DIR")?);
    let ortools_version = ortools_version();

    let (proto_files, proto_include_dirs) = if std::env::var("DOCS_RS").is_ok() {
        let vendor_sat = workspace_dir()?.join("vendor/or-tools/ortools/sat");
        let cp = vendor_sat.join("cp_model.proto");
        let sat = vendor_sat.join("sat_parameters.proto");

        println!("cargo:rerun-if-changed={}", cp.display());
        println!("cargo:rerun-if-changed={}", sat.display());

        (vec![cp, sat], vec![vendor_sat])
    } else {
        let proto_dir = out_dir.join("ortools_protos");
        let marker = proto_dir.join(".ortools-proto-version");
        let cp_path = proto_dir.join("cp_model.proto");
        let sat_path = proto_dir.join("sat_parameters.proto");

        let version_matches = std::fs::read_to_string(&marker)
            .ok()
            .map(|s| s.trim().to_string())
            .is_some_and(|v| v == ortools_version);

        if proto_dir.is_dir() && !(version_matches && cp_path.is_file() && sat_path.is_file()) {
            let _ = std::fs::remove_dir_all(&proto_dir);
        }

        if !proto_dir.is_dir() {
            std::fs::create_dir_all(&proto_dir)?;

            let base = format!(
                "https://raw.githubusercontent.com/google/or-tools/v{ortools_version}/ortools/sat"
            );
            download(&format!("{base}/cp_model.proto"), &cp_path)?;
            download(&format!("{base}/sat_parameters.proto"), &sat_path)?;
            std::fs::write(&marker, &ortools_version)?;
        }

        (vec![cp_path, sat_path], vec![proto_dir])
    };

    let mut config = prost_build::Config::new();
    config.out_dir(&out_dir);
    config.compile_protos(&proto_files, &proto_include_dirs)?;

    if std::env::var("DOCS_RS").is_err() {
        println!("cargo:rerun-if-env-changed=ORTOOLS_PREFIX");
        println!("cargo:rerun-if-env-changed=ORTOOL_PREFIX");

        if let (Ok(lib_dir), Ok(link_static)) = (
            std::env::var("DEP_OR_TOOLS_ORTOOLS_LIB_DIR"),
            std::env::var("DEP_OR_TOOLS_ORTOOLS_LINK_STATIC"),
        ) && !lib_dir.is_empty()
            && link_static != "true"
        {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
            return Ok(());
        }

        if let Ok(prefix) =
            std::env::var("ORTOOLS_PREFIX").or_else(|_| std::env::var("ORTOOL_PREFIX"))
        {
            let lib_dir = format!("{prefix}/lib");
            let lib64_dir = format!("{prefix}/lib64");
            let lib_dir = if std::path::Path::new(&lib_dir).exists() {
                lib_dir
            } else {
                lib64_dir
            };
            println!("cargo:rustc-link-arg=-Wl,-rpath,{lib_dir}");
        }
    }

    Ok(())
}

fn ortools_version() -> String {
    std::env::var("OR_TOOLS_PROTO_VERSION")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .or_else(|| {
            std::env::var("DEP_OR_TOOLS_ORTOOLS_VERSION")
                .ok()
                .filter(|s| !s.trim().is_empty())
        })
        .unwrap_or_else(|| DEFAULT_ORTOOLS_VERSION.to_string())
}

fn workspace_dir() -> Result<std::path::PathBuf, Box<dyn std::error::Error>> {
    let dir = std::env::var("CARGO_WORKSPACE_DIR")
        .ok()
        .filter(|s| !s.trim().is_empty())
        .ok_or("CARGO_WORKSPACE_DIR is not set (expected from .cargo/config.toml)")?;
    Ok(std::path::PathBuf::from(dir))
}

fn download(url: &str, out: &std::path::Path) -> Result<(), Box<dyn std::error::Error>> {
    let tmp = out.with_extension("tmp");
    if tmp.exists() {
        let _ = std::fs::remove_file(&tmp);
    }

    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    let mut last_err: Option<String> = None;
    for attempt in 1..=5 {
        match ureq::get(url)
            .header("User-Agent", "or-tools-rs-build")
            .call()
        {
            Ok(res) => {
                let mut reader = res.into_body().into_reader();
                let mut buf = Vec::new();
                reader.read_to_end(&mut buf)?;

                let mut f = std::fs::File::create(&tmp)?;
                f.write_all(&buf)?;
                drop(f);

                std::fs::rename(&tmp, out).map_err(|e| {
                    format!(
                        "failed to move downloaded proto {} -> {}: {e}",
                        tmp.display(),
                        out.display()
                    )
                })?;
                return Ok(());
            }
            Err(e) => {
                last_err = Some(format!(
                    "failed to download {url} (attempt {attempt}/5): {e}"
                ));
                std::thread::sleep(std::time::Duration::from_millis(250 * attempt));
            }
        }
    }

    let _ = std::fs::remove_file(&tmp);
    Err(last_err
        .unwrap_or_else(|| format!("failed to download {url}"))
        .into())
}
