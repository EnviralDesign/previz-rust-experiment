//! Build script for Previz
//!
//! This script:
//! 1. Downloads the latest Filament Windows release if not present
//! 2. Extracts the prebuilt libraries and headers
//! 3. Compiles a C++ wrapper for FFI
//! 4. Writes manually-crafted Rust FFI bindings (bypassing bindgen due to C++ complexity)
//! 5. Links everything together

use std::{
    env,
    fs::{self, File},
    io::{BufReader, Write},
    path::{Path, PathBuf},
    process::Command,
};

use flate2::read::GzDecoder;
use tar::Archive;

/// Filament version to use
const FILAMENT_VERSION: &str = "1.69.0";

const FILAMENT_LIBS: &[&str] = &[
    "filament",
    "backend",
    "bluegl", // OpenGL backend
    "bluevk", // Vulkan backend
    "filabridge",
    "filaflat",
    "filamat", // Material system (includes MaterialParser)
    "gltfio",
    "gltfio_core",
    "geometry",
    "ibl",
    "ibl-lite",
    "image",
    "ktxreader",
    "stb",
    "dracodec",
    "meshoptimizer",
    "utils",
    "smol-v",
    "shaders",     // Shader management
    "uberzlib",    // Compression
    "uberarchive", // Archive support
    "zstd",        // Compression
    "matdbg",      // Material debug
];

const WINDOWS_SYSTEM_LIBS: &[&str] = &["gdi32", "user32", "opengl32", "shlwapi", "advapi32", "shell32"];

struct BuildPaths {
    out_dir: PathBuf,
    manifest_dir: PathBuf,
    filament_src_dir: PathBuf,
    filament_dir: PathBuf,
    include_dir: PathBuf,
    lib_dir: PathBuf,
    filagui_include_dir: PathBuf,
    imgui_dir: PathBuf,
}

/// Download URL for Windows release
fn filament_download_url() -> String {
    format!(
        "https://github.com/google/filament/releases/download/v{}/filament-v{}-windows.tgz",
        FILAMENT_VERSION, FILAMENT_VERSION
    )
}

/// Download a file from URL to destination
fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!(
        "cargo:warning=Downloading Filament v{} (~700MB, this may take a while)...",
        FILAMENT_VERSION
    );

    let response = ureq::get(url).call()?;
    let mut file = File::create(dest)?;
    let mut reader = response.into_reader();
    std::io::copy(&mut reader, &mut file)?;

    println!("cargo:warning=Download complete!");
    Ok(())
}

/// Extract a .tgz file to destination directory
fn extract_tgz(archive_path: &Path, dest_dir: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Extracting Filament archive...");

    let file = File::open(archive_path)?;
    let reader = BufReader::new(file);
    let decoder = GzDecoder::new(reader);
    let mut archive = Archive::new(decoder);

    archive.unpack(dest_dir)?;

    println!("cargo:warning=Extraction complete!");
    Ok(())
}

/// Get Filament directory, downloading if necessary.
/// The Filament archive extracts files directly to the destination directory,
/// not to a subdirectory. So include/ and lib/ will be at OUT_DIR/include, etc.
fn get_filament_dir(out_dir: &Path) -> PathBuf {
    let archive_path = out_dir.join(format!("filament-v{}-windows.tgz", FILAMENT_VERSION));

    // Check if already extracted (files extract directly to out_dir)
    if out_dir.join("include").exists() && out_dir.join("lib").exists() {
        println!("cargo:warning=Using cached Filament at {:?}", out_dir);
        return out_dir.to_path_buf();
    }

    // Download if archive doesn't exist
    if !archive_path.exists() {
        download_file(&filament_download_url(), &archive_path)
            .expect("Failed to download Filament");
    }

    // Extract archive - files go directly into out_dir
    extract_tgz(&archive_path, out_dir).expect("Failed to extract Filament");

    out_dir.to_path_buf()
}

fn resolve_paths() -> BuildPaths {
    let out_dir = PathBuf::from(env::var("OUT_DIR").expect("OUT_DIR not set"));
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set"));
    let filament_src_dir = manifest_dir.join("third_party").join("filament");
    if !filament_src_dir.exists() {
        panic!(
            "Filament source repo not found at {:?}. Clone it into third_party/filament.",
            filament_src_dir
        );
    }

    let filament_dir = get_filament_dir(&out_dir);
    let include_dir = filament_dir.join("include");
    // lib/x86_64/md = dynamic CRT (MD), lib/x86_64/mt = static CRT (MT)
    // We use MD since that's the default for Rust debug builds
    let lib_dir = filament_dir.join("lib").join("x86_64").join("md");

    if !include_dir.exists() {
        panic!("Filament include directory not found at {:?}", include_dir);
    }
    if !lib_dir.exists() {
        panic!("Filament lib directory not found at {:?}", lib_dir);
    }

    let filagui_include_dir = filament_src_dir
        .join("libs")
        .join("filagui")
        .join("include");
    let imgui_dir = filament_src_dir.join("third_party").join("imgui");

    BuildPaths {
        out_dir,
        manifest_dir,
        filament_src_dir,
        filament_dir,
        include_dir,
        lib_dir,
        filagui_include_dir,
        imgui_dir,
    }
}

/// Create the C++ bindings wrapper
fn create_bindings_cpp(out_dir: &Path) -> PathBuf {
    let bindings_cpp = out_dir.join("bindings.cpp");
    let cpp_content = include_str!("build_support/bindings.cpp");

    let mut file = File::create(&bindings_cpp).expect("Failed to create bindings.cpp");
    file.write_all(cpp_content.as_bytes())
        .expect("Failed to write bindings.cpp");

    bindings_cpp
}

fn compile_filament_bindings(paths: &BuildPaths, bindings_cpp: &Path) {
    println!("cargo:warning=Compiling C++ bindings...");
    cc::Build::new()
        .cpp(true)
        .file(bindings_cpp)
        .include(&paths.include_dir)
        .include(&paths.filagui_include_dir)
        .include(&paths.imgui_dir)
        .flag("/std:c++20") // Filament uses designated initializers which require C++20
        .flag("/EHsc") // Exception handling
        .flag("/MD") // Dynamic CRT - must match Filament's "md" libraries
        .warnings(false)
        .compile("filament_bindings");
}

/// Generate Rust FFI bindings manually
/// This is more reliable than bindgen for our use case since Filament's headers
/// contain complex C++ templates that bindgen struggles with
fn generate_rust_bindings() -> String {
    include_str!("build_support/bindings.rs").to_string()
}

fn write_generated_bindings(out_dir: &Path) {
    println!("cargo:warning=Writing Rust bindings...");
    let bindings_rs = out_dir.join("bindings.rs");
    fs::write(&bindings_rs, generate_rust_bindings()).expect("Couldn't write bindings!");
}

/// Compile the baked color material using Filament's matc tool
fn compile_material(filament_dir: &Path, out_dir: &Path) -> PathBuf {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let material_src = manifest_dir.join("assets").join("bakedColor.mat");
    let material_out = out_dir.join("bakedColor.filamat");
    let matc_path = filament_dir.join("bin").join("matc.exe");

    if !material_src.exists() {
        panic!("Material source not found at {:?}", material_src);
    }
    if !matc_path.exists() {
        panic!("matc tool not found at {:?}", matc_path);
    }

    println!(
        "cargo:warning=Compiling material {} -> {}",
        material_src.display(),
        material_out.display()
    );

    let status = Command::new(&matc_path)
        .args(["-a", "opengl", "-p", "desktop", "-o"])
        .arg(&material_out)
        .arg(&material_src)
        .status()
        .expect("Failed to run matc");

    if !status.success() {
        panic!("matc failed with status {:?}", status.code());
    }

    material_out
}

/// Compile filagui materials and generate resources header/source.
fn compile_filagui_resources(
    filament_dir: &Path,
    filament_src_dir: &Path,
    out_dir: &Path,
) -> (PathBuf, PathBuf) {
    let filagui_dir = filament_src_dir.join("libs").join("filagui");
    let material_src = filagui_dir.join("src").join("materials").join("uiBlit.mat");
    let matc_path = filament_dir.join("bin").join("matc.exe");
    let resgen_path = filament_dir.join("bin").join("resgen.exe");

    if !material_src.exists() {
        panic!("filagui material source not found at {:?}", material_src);
    }
    if !matc_path.exists() {
        panic!("matc tool not found at {:?}", matc_path);
    }
    if !resgen_path.exists() {
        panic!("resgen tool not found at {:?}", resgen_path);
    }

    let generation_root = out_dir.join("filagui_generated");
    let material_dir = generation_root.join("generated").join("material");
    let resource_dir = generation_root.join("generated").join("resources");
    fs::create_dir_all(&material_dir).expect("Failed to create filagui material dir");
    fs::create_dir_all(&resource_dir).expect("Failed to create filagui resource dir");

    let material_out = material_dir.join("uiBlit.filamat");
    println!(
        "cargo:warning=Compiling filagui material {} -> {}",
        material_src.display(),
        material_out.display()
    );
    let status = Command::new(&matc_path)
        .args(["-a", "opengl", "-p", "desktop", "-o"])
        .arg(&material_out)
        .arg(&material_src)
        .status()
        .expect("Failed to run matc for filagui");
    if !status.success() {
        panic!("matc failed for filagui with status {:?}", status.code());
    }

    println!(
        "cargo:warning=Generating filagui resources in {}",
        resource_dir.display()
    );
    let status = Command::new(&resgen_path)
        .args(["-p", "filagui_resources", "-x"])
        .arg(&resource_dir)
        .arg("-c")
        .arg(&material_out)
        .status()
        .expect("Failed to run resgen for filagui");
    if !status.success() {
        panic!("resgen failed for filagui with status {:?}", status.code());
    }

    (generation_root, resource_dir)
}

fn compile_filagui_bindings(paths: &BuildPaths, filagui_generation_root: &Path, filagui_resource_dir: &Path) {
    let filagui_resources_c = filagui_resource_dir.join("filagui_resources.c");
    let filagui_src_dir = paths.filament_src_dir.join("libs").join("filagui").join("src");

    println!("cargo:warning=Compiling filagui...");
    cc::Build::new()
        .cpp(true)
        .file(filagui_src_dir.join("ImGuiHelper.cpp"))
        .file(filagui_src_dir.join("ImGuiExtensions.cpp"))
        .file(paths.imgui_dir.join("imgui.cpp"))
        .file(paths.imgui_dir.join("imgui_draw.cpp"))
        .file(paths.imgui_dir.join("imgui_tables.cpp"))
        .file(paths.imgui_dir.join("imgui_widgets.cpp"))
        .file(filagui_resources_c)
        .include(&paths.include_dir)
        .include(&paths.filagui_include_dir)
        .include(&paths.imgui_dir)
        .include(filagui_generation_root)
        .flag("/std:c++20")
        .flag("/EHsc")
        .flag("/MD")
        .warnings(false)
        .compile("filagui_bindings");
}

fn emit_link_directives(paths: &BuildPaths) {
    println!("cargo:rustc-link-search=native={}", paths.lib_dir.display());
    println!(
        "cargo:rustc-env=FILAMENT_BIN_DIR={}",
        paths.filament_dir.join("bin").display()
    );

    for lib in FILAMENT_LIBS {
        println!("cargo:rustc-link-lib=static={}", lib);
    }
    println!("cargo:rustc-link-lib=static=filagui_bindings");

    for lib in WINDOWS_SYSTEM_LIBS {
        println!("cargo:rustc-link-lib={}", lib);
    }
}

fn emit_rerun_if_changed(paths: &BuildPaths) {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=build_support/bindings.cpp");
    println!("cargo:rerun-if-changed=build_support/bindings.rs");
    println!(
        "cargo:rerun-if-changed={}",
        paths.manifest_dir.join("assets").join("bakedColor.mat").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .manifest_dir
            .join("assets")
            .join("gltf")
            .join("DamagedHelmet.gltf")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("ImGuiHelper.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("ImGuiExtensions.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("materials")
            .join("uiBlit.mat")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_draw.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_tables.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        paths
            .filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_widgets.cpp")
            .display()
    );
}

fn main() {
    // Only build on Windows for now
    #[cfg(not(target_os = "windows"))]
    compile_error!("This build script currently only supports Windows");

    let paths = resolve_paths();
    println!("cargo:warning=Using Filament from {:?}", paths.filament_dir);

    let bindings_cpp = create_bindings_cpp(&paths.out_dir);
    compile_filament_bindings(&paths, &bindings_cpp);
    write_generated_bindings(&paths.out_dir);

    let material_out = compile_material(&paths.filament_dir, &paths.out_dir);
    println!(
        "cargo:warning=Material compiled at {}",
        material_out.display()
    );

    let (filagui_generation_root, filagui_resource_dir) =
        compile_filagui_resources(&paths.filament_dir, &paths.filament_src_dir, &paths.out_dir);
    compile_filagui_bindings(&paths, &filagui_generation_root, &filagui_resource_dir);

    emit_link_directives(&paths);
    emit_rerun_if_changed(&paths);
}
