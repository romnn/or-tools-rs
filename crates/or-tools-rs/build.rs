extern crate prost_build;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    prost_build::compile_protos(
        &["src/cp_model.proto", "src/sat_parameters.proto"],
        &["src/"],
    )?;

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
