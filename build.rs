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

/// Download URL for Windows release
fn filament_download_url() -> String {
    format!(
        "https://github.com/google/filament/releases/download/v{}/filament-v{}-windows.tgz",
        FILAMENT_VERSION, FILAMENT_VERSION
    )
}

/// Download a file from URL to destination
fn download_file(url: &str, dest: &Path) -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:warning=Downloading Filament v{} (~700MB, this may take a while)...", FILAMENT_VERSION);
    
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

/// Get Filament directory, downloading if necessary
/// The Filament archive extracts files directly to the destination directory,
/// not to a subdirectory. So include/ and lib/ will be at OUT_DIR/include, etc.
fn get_filament_dir() -> PathBuf {
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let archive_path = out_dir.join(format!("filament-v{}-windows.tgz", FILAMENT_VERSION));
    
    // Check if already extracted (files extract directly to out_dir)
    if out_dir.join("include").exists() && out_dir.join("lib").exists() {
        println!("cargo:warning=Using cached Filament at {:?}", out_dir);
        return out_dir;
    }
    
    // Download if archive doesn't exist
    if !archive_path.exists() {
        download_file(&filament_download_url(), &archive_path)
            .expect("Failed to download Filament");
    }
    
    // Extract archive - files go directly into out_dir
    extract_tgz(&archive_path, &out_dir)
        .expect("Failed to extract Filament");
    
    out_dir
}

/// Create the C++ bindings wrapper
fn create_bindings_cpp(_filament_dir: &Path, out_dir: &Path) -> PathBuf {
    let bindings_cpp = out_dir.join("bindings.cpp");
    
    // This is a minimal wrapper that exposes Filament's C++ API through C-style functions
    // We only include what we need for the hello world demo
    let cpp_content = r#"
// Minimal Filament C++ bindings wrapper
// This file exposes Filament C++ classes through extern "C" functions for FFI

#include <cstdio>
#include <filament/Engine.h>
#include <filament/Renderer.h>
#include <filament/Scene.h>
#include <filament/View.h>
#include <filament/Viewport.h>
#include <filament/Camera.h>
#include <filament/SwapChain.h>
#include <filament/Material.h>
#include <filament/MaterialInstance.h>
#include <filament/LightManager.h>
#include <filament/Box.h>
#include <filament/VertexBuffer.h>
#include <filament/IndexBuffer.h>
#include <filament/RenderableManager.h>
#include <filament/TransformManager.h>
#include <gltfio/AssetLoader.h>
#include <gltfio/FilamentAsset.h>
#include <gltfio/MaterialProvider.h>
#include <gltfio/ResourceLoader.h>
#include <gltfio/TextureProvider.h>
#include <utils/EntityManager.h>
#include <backend/DriverEnums.h>

using namespace filament;
using namespace utils;
using namespace filament::gltfio;

extern "C" {

// ============================================================================
// Engine
// ============================================================================

Engine* filament_engine_create(backend::Backend backend) {
    return Engine::create(backend);
}

void filament_engine_destroy(Engine** engine) {
    Engine::destroy(engine);
}

SwapChain* filament_engine_create_swap_chain(Engine* engine, void* native_window, uint64_t flags) {
    return engine->createSwapChain(native_window, flags);
}

void filament_engine_destroy_swap_chain(Engine* engine, SwapChain* swap_chain) {
    engine->destroy(swap_chain);
}

Renderer* filament_engine_create_renderer(Engine* engine) {
    printf("[C++] filament_engine_create_renderer: engine=%p\n", engine);
    fflush(stdout);
    if (!engine) return nullptr;
    auto* result = engine->createRenderer();
    printf("[C++] filament_engine_create_renderer: result=%p\n", result);
    fflush(stdout);
    return result;
}

void filament_engine_destroy_renderer(Engine* engine, Renderer* renderer) {
    engine->destroy(renderer);
}

Scene* filament_engine_create_scene(Engine* engine) {
    printf("[C++] filament_engine_create_scene: engine=%p\n", engine);
    fflush(stdout);
    if (!engine) return nullptr;
    auto* result = engine->createScene();
    printf("[C++] filament_engine_create_scene: result=%p\n", result);
    fflush(stdout);
    return result;
}

void filament_engine_destroy_scene(Engine* engine, Scene* scene) {
    engine->destroy(scene);
}

View* filament_engine_create_view(Engine* engine) {
    return engine->createView();
}

void filament_engine_destroy_view(Engine* engine, View* view) {
    engine->destroy(view);
}

Camera* filament_engine_create_camera(Engine* engine, int32_t entity_id) {
    Entity entity = Entity::import(entity_id);
    return engine->createCamera(entity);
}

void filament_engine_destroy_camera(Engine* engine, Camera* camera) {
    engine->destroyCameraComponent(camera->getEntity());
}

EntityManager* filament_engine_get_entity_manager(Engine* engine) {
    return &engine->getEntityManager();
}

TransformManager* filament_engine_get_transform_manager(Engine* engine) {
    return &engine->getTransformManager();
}

RenderableManager* filament_engine_get_renderable_manager(Engine* engine) {
    return &engine->getRenderableManager();
}

void filament_engine_flush_and_wait(Engine* engine) {
    engine->flushAndWait();
}

// ============================================================================
// Renderer
// ============================================================================

bool filament_renderer_begin_frame(Renderer* renderer, SwapChain* swap_chain) {
    return renderer->beginFrame(swap_chain);
}

void filament_renderer_end_frame(Renderer* renderer) {
    renderer->endFrame();
}

void filament_renderer_render(Renderer* renderer, View* view) {
    renderer->render(view);
}

void filament_renderer_set_clear_options(Renderer* renderer, float r, float g, float b, float a, bool clear, bool discard) {
    printf("[C++] filament_renderer_set_clear_options: renderer=%p\n", renderer);
    fflush(stdout);
    Renderer::ClearOptions options;
    options.clearColor = {r, g, b, a};
    options.clear = clear;
    options.discard = discard;
    renderer->setClearOptions(options);
    printf("[C++] filament_renderer_set_clear_options: done\n");
    fflush(stdout);
}

// ============================================================================
// View
// ============================================================================

void filament_view_set_scene(View* view, Scene* scene) {
    view->setScene(scene);
}

void filament_view_set_camera(View* view, Camera* camera) {
    view->setCamera(camera);
}

void filament_view_set_viewport(View* view, int32_t left, int32_t bottom, uint32_t width, uint32_t height) {
    view->setViewport({left, bottom, width, height});
}

void filament_view_set_post_processing_enabled(View* view, bool enabled) {
    view->setPostProcessingEnabled(enabled);
}

// ============================================================================
// Scene
// ============================================================================

void filament_scene_add_entity(Scene* scene, int32_t entity_id) {
    Entity entity = Entity::import(entity_id);
    scene->addEntity(entity);
}

void filament_scene_remove_entity(Scene* scene, int32_t entity_id) {
    Entity entity = Entity::import(entity_id);
    scene->remove(entity);
}

// ============================================================================
// Camera
// ============================================================================

void filament_camera_set_projection_ortho(Camera* camera, double left, double right, double bottom, double top, double near, double far) {
    camera->setProjection(Camera::Projection::ORTHO, left, right, bottom, top, near, far);
}

void filament_camera_set_projection_perspective(Camera* camera, double fov_degrees, double aspect, double near, double far) {
    camera->setProjection(fov_degrees, aspect, near, far);
}

void filament_camera_look_at(Camera* camera, float eye_x, float eye_y, float eye_z, float center_x, float center_y, float center_z, float up_x, float up_y, float up_z) {
    camera->lookAt({eye_x, eye_y, eye_z}, {center_x, center_y, center_z}, {up_x, up_y, up_z});
}

// ============================================================================
// Entity Manager
// ============================================================================

int32_t filament_entity_manager_create(EntityManager* em) {
    printf("[C++] filament_entity_manager_create: em=%p\n", em);
    fflush(stdout);
    Entity entity = em->create();
    int32_t result = Entity::smuggle(entity);
    printf("[C++] filament_entity_manager_create: entity_id=%d\n", result);
    fflush(stdout);
    return result;
}

void filament_entity_manager_destroy(EntityManager* em, int32_t entity_id) {
    Entity entity = Entity::import(entity_id);
    em->destroy(entity);
}

// No longer needed since we use smuggle/import now

// ============================================================================
// Material
// ============================================================================

typedef struct {
    Material::Builder* builder;
} MaterialBuilderWrapper;

MaterialBuilderWrapper* filament_material_builder_create() {
    auto* wrapper = new MaterialBuilderWrapper();
    wrapper->builder = new Material::Builder();
    return wrapper;
}

void filament_material_builder_destroy(MaterialBuilderWrapper* wrapper) {
    delete wrapper->builder;
    delete wrapper;
}

void filament_material_builder_package(MaterialBuilderWrapper* wrapper, const void* data, size_t size) {
    wrapper->builder->package(data, size);
}

Material* filament_material_builder_build(MaterialBuilderWrapper* wrapper, Engine* engine) {
    return wrapper->builder->build(*engine);
}

MaterialInstance* filament_material_get_default_instance(Material* material) {
    return material->getDefaultInstance();
}

MaterialInstance* filament_material_create_instance(Material* material) {
    return material->createInstance();
}

// ============================================================================
// Vertex Buffer
// ============================================================================

typedef struct {
    VertexBuffer::Builder* builder;
} VertexBufferBuilderWrapper;

VertexBufferBuilderWrapper* filament_vertex_buffer_builder_create() {
    auto* wrapper = new VertexBufferBuilderWrapper();
    wrapper->builder = new VertexBuffer::Builder();
    return wrapper;
}

void filament_vertex_buffer_builder_destroy(VertexBufferBuilderWrapper* wrapper) {
    delete wrapper->builder;
    delete wrapper;
}

void filament_vertex_buffer_builder_vertex_count(VertexBufferBuilderWrapper* wrapper, uint32_t count) {
    wrapper->builder->vertexCount(count);
}

void filament_vertex_buffer_builder_buffer_count(VertexBufferBuilderWrapper* wrapper, uint8_t count) {
    wrapper->builder->bufferCount(count);
}

void filament_vertex_buffer_builder_attribute(
    VertexBufferBuilderWrapper* wrapper,
    VertexAttribute attribute,
    uint8_t buffer_index,
    backend::ElementType element_type,
    uint32_t byte_offset,
    uint8_t byte_stride
) {
    wrapper->builder->attribute(attribute, buffer_index, element_type, byte_offset, byte_stride);
}

void filament_vertex_buffer_builder_normalized(VertexBufferBuilderWrapper* wrapper, VertexAttribute attribute, bool normalized) {
    wrapper->builder->normalized(attribute, normalized);
}

VertexBuffer* filament_vertex_buffer_builder_build(VertexBufferBuilderWrapper* wrapper, Engine* engine) {
    return wrapper->builder->build(*engine);
}

void filament_vertex_buffer_set_buffer_at(VertexBuffer* vb, Engine* engine, uint8_t buffer_index, const void* data, size_t size, uint32_t dest_offset) {
    // Create a copy of the data since Filament takes ownership
    void* buffer_copy = malloc(size);
    memcpy(buffer_copy, data, size);
    
    backend::BufferDescriptor desc(buffer_copy, size, [](void* buffer, size_t, void*) {
        free(buffer);
    });
    vb->setBufferAt(*engine, buffer_index, std::move(desc), dest_offset);
}

// ============================================================================
// Index Buffer
// ============================================================================

typedef struct {
    IndexBuffer::Builder* builder;
} IndexBufferBuilderWrapper;

IndexBufferBuilderWrapper* filament_index_buffer_builder_create() {
    auto* wrapper = new IndexBufferBuilderWrapper();
    wrapper->builder = new IndexBuffer::Builder();
    return wrapper;
}

void filament_index_buffer_builder_destroy(IndexBufferBuilderWrapper* wrapper) {
    delete wrapper->builder;
    delete wrapper;
}

void filament_index_buffer_builder_index_count(IndexBufferBuilderWrapper* wrapper, uint32_t count) {
    wrapper->builder->indexCount(count);
}

void filament_index_buffer_builder_buffer_type(IndexBufferBuilderWrapper* wrapper, IndexBuffer::IndexType type) {
    wrapper->builder->bufferType(type);
}

IndexBuffer* filament_index_buffer_builder_build(IndexBufferBuilderWrapper* wrapper, Engine* engine) {
    return wrapper->builder->build(*engine);
}

void filament_index_buffer_set_buffer(IndexBuffer* ib, Engine* engine, const void* data, size_t size, uint32_t dest_offset) {
    // Create a copy of the data since Filament takes ownership
    void* buffer_copy = malloc(size);
    memcpy(buffer_copy, data, size);
    
    backend::BufferDescriptor desc(buffer_copy, size, [](void* buffer, size_t, void*) {
        free(buffer);
    });
    ib->setBuffer(*engine, std::move(desc), dest_offset);
}

// ============================================================================
// Renderable Manager
// ============================================================================

typedef struct {
    RenderableManager::Builder* builder;
} RenderableBuilderWrapper;

RenderableBuilderWrapper* filament_renderable_builder_create(size_t count) {
    auto* wrapper = new RenderableBuilderWrapper();
    wrapper->builder = new RenderableManager::Builder(count);
    return wrapper;
}

void filament_renderable_builder_destroy(RenderableBuilderWrapper* wrapper) {
    delete wrapper->builder;
    delete wrapper;
}

void filament_renderable_builder_bounding_box(RenderableBuilderWrapper* wrapper, float cx, float cy, float cz, float hx, float hy, float hz) {
    wrapper->builder->boundingBox({{cx - hx, cy - hy, cz - hz}, {cx + hx, cy + hy, cz + hz}});
}

void filament_renderable_builder_material(RenderableBuilderWrapper* wrapper, size_t index, MaterialInstance* mi) {
    wrapper->builder->material(index, mi);
}

void filament_renderable_builder_geometry(
    RenderableBuilderWrapper* wrapper,
    size_t index,
    RenderableManager::PrimitiveType type,
    VertexBuffer* vb,
    IndexBuffer* ib
) {
    wrapper->builder->geometry(index, type, vb, ib);
}

void filament_renderable_builder_geometry_range(
    RenderableBuilderWrapper* wrapper,
    size_t index,
    RenderableManager::PrimitiveType type,
    VertexBuffer* vb,
    IndexBuffer* ib,
    size_t offset,
    size_t count
) {
    wrapper->builder->geometry(index, type, vb, ib, offset, count);
}

void filament_renderable_builder_culling(RenderableBuilderWrapper* wrapper, bool enabled) {
    wrapper->builder->culling(enabled);
}

void filament_renderable_builder_build(RenderableBuilderWrapper* wrapper, Engine* engine, int32_t entity_id) {
    Entity entity = Entity::import(entity_id);
    wrapper->builder->build(*engine, entity);
}

// ============================================================================
// Lights
// ============================================================================

int32_t filament_light_create_directional(
    Engine* engine,
    EntityManager* em,
    float color_r,
    float color_g,
    float color_b,
    float intensity,
    float dir_x,
    float dir_y,
    float dir_z
) {
    Entity entity = em->create();
    LightManager::Builder(LightManager::Type::DIRECTIONAL)
        .color({color_r, color_g, color_b})
        .intensity(intensity)
        .direction({dir_x, dir_y, dir_z})
        .castShadows(true)
        .build(*engine, entity);
    return Entity::smuggle(entity);
}

// ============================================================================
// gltfio
// ============================================================================

MaterialProvider* filament_gltfio_create_jit_shader_provider(Engine* engine, bool optimize) {
    return createJitShaderProvider(engine, optimize);
}

void filament_gltfio_material_provider_destroy_materials(MaterialProvider* provider) {
    if (provider) {
        provider->destroyMaterials();
    }
}

void filament_gltfio_destroy_material_provider(MaterialProvider* provider) {
    delete provider;
}

AssetLoader* filament_gltfio_asset_loader_create(
    Engine* engine,
    MaterialProvider* materials,
    EntityManager* entities
) {
    AssetConfiguration config{};
    config.engine = engine;
    config.materials = materials;
    config.entities = entities;
    return AssetLoader::create(config);
}

void filament_gltfio_asset_loader_destroy(AssetLoader* loader) {
    AssetLoader::destroy(&loader);
}

FilamentAsset* filament_gltfio_asset_loader_create_asset_from_json(
    AssetLoader* loader,
    const uint8_t* data,
    uint32_t size
) {
    return loader->createAsset(data, size);
}

void filament_gltfio_asset_loader_destroy_asset(AssetLoader* loader, FilamentAsset* asset) {
    loader->destroyAsset(asset);
}

ResourceLoader* filament_gltfio_resource_loader_create(
    Engine* engine,
    const char* gltf_path,
    bool normalize_skinning_weights
) {
    ResourceConfiguration config{engine, gltf_path, normalize_skinning_weights};
    return new ResourceLoader(config);
}

void filament_gltfio_resource_loader_destroy(ResourceLoader* loader) {
    delete loader;
}

bool filament_gltfio_resource_loader_load_resources(ResourceLoader* loader, FilamentAsset* asset) {
    return loader->loadResources(asset);
}

void filament_gltfio_resource_loader_add_texture_provider(
    ResourceLoader* loader,
    const char* mime_type,
    TextureProvider* provider
) {
    loader->addTextureProvider(mime_type, provider);
}

TextureProvider* filament_gltfio_create_stb_texture_provider(Engine* engine) {
    return createStbProvider(engine);
}

void filament_gltfio_destroy_texture_provider(TextureProvider* provider) {
    delete provider;
}

void filament_gltfio_asset_add_entities_to_scene(FilamentAsset* asset, Scene* scene) {
    auto entities = asset->getEntities();
    auto count = asset->getEntityCount();
    scene->addEntities(entities, count);
}

void filament_gltfio_asset_release_source_data(FilamentAsset* asset) {
    asset->releaseSourceData();
}

void filament_gltfio_asset_get_bounding_box(
    FilamentAsset* asset,
    float* center_xyz,
    float* extent_xyz
) {
    filament::Aabb box = asset->getBoundingBox();
    auto c = box.center();
    auto e = box.extent();
    center_xyz[0] = c.x;
    center_xyz[1] = c.y;
    center_xyz[2] = c.z;
    extent_xyz[0] = e.x;
    extent_xyz[1] = e.y;
    extent_xyz[2] = e.z;
}

} // extern "C"
"#;

    let mut file = File::create(&bindings_cpp).expect("Failed to create bindings.cpp");
    file.write_all(cpp_content.as_bytes()).expect("Failed to write bindings.cpp");
    
    bindings_cpp
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

/// Generate Rust FFI bindings manually
/// This is more reliable than bindgen for our use case since Filament's headers
/// contain complex C++ templates that bindgen struggles with
fn generate_rust_bindings() -> String {
    r#"// Auto-generated Filament FFI bindings
// These bindings are manually crafted to match our C++ wrapper functions.
// We avoid bindgen because Filament's headers use complex C++ templates.

use std::ffi::{c_char, c_void};

// Opaque pointer types for Filament objects
pub type Engine = c_void;
pub type SwapChain = c_void;
pub type Renderer = c_void;
pub type Scene = c_void;
pub type View = c_void;
pub type Camera = c_void;
pub type EntityManager = c_void;
pub type TransformManager = c_void;
pub type RenderableManager = c_void;
pub type Material = c_void;
pub type MaterialInstance = c_void;
pub type VertexBuffer = c_void;
pub type IndexBuffer = c_void;
pub type MaterialProvider = c_void;
pub type AssetLoader = c_void;
pub type ResourceLoader = c_void;
pub type TextureProvider = c_void;
pub type FilamentAsset = c_void;

// Builder wrapper types (opaque)
pub type MaterialBuilderWrapper = c_void;
pub type VertexBufferBuilderWrapper = c_void;
pub type IndexBufferBuilderWrapper = c_void;
pub type RenderableBuilderWrapper = c_void;

extern "C" {
    // ========================================================================
    // Engine
    // ========================================================================
    
    pub fn filament_engine_create(backend: u8) -> *mut Engine;
    pub fn filament_engine_destroy(engine: *mut *mut Engine);
    
    pub fn filament_engine_create_swap_chain(
        engine: *mut Engine,
        native_window: *mut c_void,
        flags: u64,
    ) -> *mut SwapChain;
    pub fn filament_engine_destroy_swap_chain(engine: *mut Engine, swap_chain: *mut SwapChain);
    
    pub fn filament_engine_create_renderer(engine: *mut Engine) -> *mut Renderer;
    pub fn filament_engine_destroy_renderer(engine: *mut Engine, renderer: *mut Renderer);
    
    pub fn filament_engine_create_scene(engine: *mut Engine) -> *mut Scene;
    pub fn filament_engine_destroy_scene(engine: *mut Engine, scene: *mut Scene);
    
    pub fn filament_engine_create_view(engine: *mut Engine) -> *mut View;
    pub fn filament_engine_destroy_view(engine: *mut Engine, view: *mut View);
    
    pub fn filament_engine_create_camera(engine: *mut Engine, entity: i32) -> *mut Camera;
    pub fn filament_engine_destroy_camera(engine: *mut Engine, camera: *mut Camera);
    
    pub fn filament_engine_get_entity_manager(engine: *mut Engine) -> *mut EntityManager;
    pub fn filament_engine_get_transform_manager(engine: *mut Engine) -> *mut TransformManager;
    pub fn filament_engine_get_renderable_manager(engine: *mut Engine) -> *mut RenderableManager;
    
    pub fn filament_engine_flush_and_wait(engine: *mut Engine);
    
    // ========================================================================
    // Renderer
    // ========================================================================
    
    pub fn filament_renderer_begin_frame(
        renderer: *mut Renderer,
        swap_chain: *mut SwapChain,
    ) -> bool;
    pub fn filament_renderer_end_frame(renderer: *mut Renderer);
    pub fn filament_renderer_render(renderer: *mut Renderer, view: *mut View);
    pub fn filament_renderer_set_clear_options(
        renderer: *mut Renderer,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        clear: bool,
        discard: bool,
    );
    
    // ========================================================================
    // View
    // ========================================================================
    
    pub fn filament_view_set_scene(view: *mut View, scene: *mut Scene);
    pub fn filament_view_set_camera(view: *mut View, camera: *mut Camera);
    pub fn filament_view_set_viewport(
        view: *mut View,
        left: i32,
        bottom: i32,
        width: u32,
        height: u32,
    );
    pub fn filament_view_set_post_processing_enabled(view: *mut View, enabled: bool);
    
    // ========================================================================
    // Scene
    // ========================================================================
    
    pub fn filament_scene_add_entity(scene: *mut Scene, entity: i32);
    pub fn filament_scene_remove_entity(scene: *mut Scene, entity: i32);
    
    // ========================================================================
    // Camera
    // ========================================================================
    
    pub fn filament_camera_set_projection_ortho(
        camera: *mut Camera,
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    );
    pub fn filament_camera_set_projection_perspective(
        camera: *mut Camera,
        fov_degrees: f64,
        aspect: f64,
        near: f64,
        far: f64,
    );
    pub fn filament_camera_look_at(
        camera: *mut Camera,
        eye_x: f32,
        eye_y: f32,
        eye_z: f32,
        center_x: f32,
        center_y: f32,
        center_z: f32,
        up_x: f32,
        up_y: f32,
        up_z: f32,
    );
    
    // ========================================================================
    // Entity Manager
    // ========================================================================
    
    pub fn filament_entity_manager_create(em: *mut EntityManager) -> i32;
    pub fn filament_entity_manager_destroy(em: *mut EntityManager, entity: i32);
    
    // ========================================================================
    // Material
    // ========================================================================
    
    pub fn filament_material_builder_create() -> *mut MaterialBuilderWrapper;
    pub fn filament_material_builder_destroy(wrapper: *mut MaterialBuilderWrapper);
    pub fn filament_material_builder_package(
        wrapper: *mut MaterialBuilderWrapper,
        data: *const c_void,
        size: usize,
    );
    pub fn filament_material_builder_build(
        wrapper: *mut MaterialBuilderWrapper,
        engine: *mut Engine,
    ) -> *mut Material;
    
    pub fn filament_material_get_default_instance(
        material: *mut Material,
    ) -> *mut MaterialInstance;
    pub fn filament_material_create_instance(material: *mut Material) -> *mut MaterialInstance;
    
    // ========================================================================
    // Vertex Buffer
    // ========================================================================
    
    pub fn filament_vertex_buffer_builder_create() -> *mut VertexBufferBuilderWrapper;
    pub fn filament_vertex_buffer_builder_destroy(wrapper: *mut VertexBufferBuilderWrapper);
    pub fn filament_vertex_buffer_builder_vertex_count(
        wrapper: *mut VertexBufferBuilderWrapper,
        count: u32,
    );
    pub fn filament_vertex_buffer_builder_buffer_count(
        wrapper: *mut VertexBufferBuilderWrapper,
        count: u8,
    );
    pub fn filament_vertex_buffer_builder_attribute(
        wrapper: *mut VertexBufferBuilderWrapper,
        attribute: u8,
        buffer_index: u8,
        element_type: u8,
        byte_offset: u32,
        byte_stride: u8,
    );
    pub fn filament_vertex_buffer_builder_normalized(
        wrapper: *mut VertexBufferBuilderWrapper,
        attribute: u8,
        normalized: bool,
    );
    pub fn filament_vertex_buffer_builder_build(
        wrapper: *mut VertexBufferBuilderWrapper,
        engine: *mut Engine,
    ) -> *mut VertexBuffer;
    
    pub fn filament_vertex_buffer_set_buffer_at(
        vb: *mut VertexBuffer,
        engine: *mut Engine,
        buffer_index: u8,
        data: *const c_void,
        size: usize,
        dest_offset: u32,
    );
    
    // ========================================================================
    // Index Buffer
    // ========================================================================
    
    pub fn filament_index_buffer_builder_create() -> *mut IndexBufferBuilderWrapper;
    pub fn filament_index_buffer_builder_destroy(wrapper: *mut IndexBufferBuilderWrapper);
    pub fn filament_index_buffer_builder_index_count(
        wrapper: *mut IndexBufferBuilderWrapper,
        count: u32,
    );
    pub fn filament_index_buffer_builder_buffer_type(
        wrapper: *mut IndexBufferBuilderWrapper,
        index_type: u8,
    );
    pub fn filament_index_buffer_builder_build(
        wrapper: *mut IndexBufferBuilderWrapper,
        engine: *mut Engine,
    ) -> *mut IndexBuffer;
    
    pub fn filament_index_buffer_set_buffer(
        ib: *mut IndexBuffer,
        engine: *mut Engine,
        data: *const c_void,
        size: usize,
        dest_offset: u32,
    );
    
    // ========================================================================
    // Renderable Manager
    // ========================================================================
    
    pub fn filament_renderable_builder_create(count: usize) -> *mut RenderableBuilderWrapper;
    pub fn filament_renderable_builder_destroy(wrapper: *mut RenderableBuilderWrapper);
    pub fn filament_renderable_builder_bounding_box(
        wrapper: *mut RenderableBuilderWrapper,
        cx: f32,
        cy: f32,
        cz: f32,
        hx: f32,
        hy: f32,
        hz: f32,
    );
    pub fn filament_renderable_builder_material(
        wrapper: *mut RenderableBuilderWrapper,
        index: usize,
        material_instance: *mut MaterialInstance,
    );
    pub fn filament_renderable_builder_geometry(
        wrapper: *mut RenderableBuilderWrapper,
        index: usize,
        primitive_type: u8,
        vertex_buffer: *mut VertexBuffer,
        index_buffer: *mut IndexBuffer,
    );
    pub fn filament_renderable_builder_culling(
        wrapper: *mut RenderableBuilderWrapper,
        enabled: bool,
    );
    pub fn filament_renderable_builder_build(
        wrapper: *mut RenderableBuilderWrapper,
        engine: *mut Engine,
        entity: i32,
    );

    // ========================================================================
    // Lights
    // ========================================================================
    pub fn filament_light_create_directional(
        engine: *mut Engine,
        entity_manager: *mut EntityManager,
        color_r: f32,
        color_g: f32,
        color_b: f32,
        intensity: f32,
        dir_x: f32,
        dir_y: f32,
        dir_z: f32,
    ) -> i32;

    // ========================================================================
    // gltfio
    // ========================================================================
    pub fn filament_gltfio_create_jit_shader_provider(
        engine: *mut Engine,
        optimize: bool,
    ) -> *mut MaterialProvider;
    pub fn filament_gltfio_material_provider_destroy_materials(provider: *mut MaterialProvider);
    pub fn filament_gltfio_destroy_material_provider(provider: *mut MaterialProvider);

    pub fn filament_gltfio_asset_loader_create(
        engine: *mut Engine,
        materials: *mut MaterialProvider,
        entities: *mut EntityManager,
    ) -> *mut AssetLoader;
    pub fn filament_gltfio_asset_loader_destroy(loader: *mut AssetLoader);
    pub fn filament_gltfio_asset_loader_create_asset_from_json(
        loader: *mut AssetLoader,
        data: *const u8,
        size: u32,
    ) -> *mut FilamentAsset;
    pub fn filament_gltfio_asset_loader_destroy_asset(
        loader: *mut AssetLoader,
        asset: *mut FilamentAsset,
    );

    pub fn filament_gltfio_resource_loader_create(
        engine: *mut Engine,
        gltf_path: *const c_char,
        normalize_skinning_weights: bool,
    ) -> *mut ResourceLoader;
    pub fn filament_gltfio_resource_loader_destroy(loader: *mut ResourceLoader);
    pub fn filament_gltfio_resource_loader_load_resources(
        loader: *mut ResourceLoader,
        asset: *mut FilamentAsset,
    ) -> bool;
    pub fn filament_gltfio_resource_loader_add_texture_provider(
        loader: *mut ResourceLoader,
        mime_type: *const c_char,
        provider: *mut TextureProvider,
    );

    pub fn filament_gltfio_create_stb_texture_provider(
        engine: *mut Engine,
    ) -> *mut TextureProvider;
    pub fn filament_gltfio_destroy_texture_provider(provider: *mut TextureProvider);

    pub fn filament_gltfio_asset_add_entities_to_scene(
        asset: *mut FilamentAsset,
        scene: *mut Scene,
    );
    pub fn filament_gltfio_asset_release_source_data(asset: *mut FilamentAsset);
    pub fn filament_gltfio_asset_get_bounding_box(
        asset: *mut FilamentAsset,
        center_xyz: *mut f32,
        extent_xyz: *mut f32,
    );
}
"#.to_string()
}

fn main() {
    // Only build on Windows for now
    #[cfg(not(target_os = "windows"))]
    compile_error!("This build script currently only supports Windows");
    
    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    
    // Step 1: Get Filament (download if needed)
    let filament_dir = get_filament_dir();
    let include_dir = filament_dir.join("include");
    // lib/x86_64/md = dynamic CRT (MD), lib/x86_64/mt = static CRT (MT)
    // We use MD since that's the default for Rust debug builds
    let lib_dir = filament_dir.join("lib").join("x86_64").join("md");
    
    // Verify directories exist
    if !include_dir.exists() {
        panic!("Filament include directory not found at {:?}", include_dir);
    }
    if !lib_dir.exists() {
        panic!("Filament lib directory not found at {:?}", lib_dir);
    }
    
    println!("cargo:warning=Using Filament from {:?}", filament_dir);
    
    // Step 2: Create C++ bindings wrapper
    let bindings_cpp = create_bindings_cpp(&filament_dir, &out_dir);
    
    // Step 3: Compile C++ bindings wrapper
    println!("cargo:warning=Compiling C++ bindings...");
    cc::Build::new()
        .cpp(true)
        .file(&bindings_cpp)
        .include(&include_dir)
        .flag("/std:c++20") // Filament uses designated initializers which require C++20
        .flag("/EHsc")     // Exception handling
        .flag("/MD")       // Dynamic CRT - must match Filament's "md" libraries
        .warnings(false)
        .compile("filament_bindings");
    
    // Step 4: Generate Rust bindings manually
    // We skip bindgen because Filament's C++ headers contain complex templates
    // that bindgen can't handle. Since our API is fixed and well-defined,
    // we write the FFI bindings manually.
    println!("cargo:warning=Writing Rust bindings...");
    let bindings_rs = out_dir.join("bindings.rs");
    fs::write(&bindings_rs, generate_rust_bindings())
        .expect("Couldn't write bindings!");

    // Step 4.5: Compile material with matching Filament version
    let material_out = compile_material(&filament_dir, &out_dir);
    println!(
        "cargo:warning=Material compiled at {}",
        material_out.display()
    );
    
    // Step 5: Link Filament libraries
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    
    // Core Filament libraries (order matters for static linking)
    // The libraries listed here are the ones required for a basic rendering setup
    let filament_libs = [
        "filament",
        "backend",
        "bluegl",        // OpenGL backend
        "bluevk",        // Vulkan backend
        "filabridge",
        "filaflat",
        "filamat",       // Material system (includes MaterialParser)
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
        "shaders",       // Shader management
        "uberzlib",      // Compression
        "uberarchive",   // Archive support
        "zstd",          // Compression
        "matdbg",        // Material debug
    ];
    
    for lib in &filament_libs {
        println!("cargo:rustc-link-lib=static={}", lib);
    }
    
    // System libraries required on Windows
    println!("cargo:rustc-link-lib=gdi32");
    println!("cargo:rustc-link-lib=user32");
    println!("cargo:rustc-link-lib=opengl32");
    println!("cargo:rustc-link-lib=shlwapi");
    println!("cargo:rustc-link-lib=advapi32");
    println!("cargo:rustc-link-lib=shell32");
    
    // Rebuild if bindings.cpp changes
    println!("cargo:rerun-if-changed=build.rs");
    println!(
        "cargo:rerun-if-changed={}",
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("assets")
            .join("bakedColor.mat")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap())
            .join("assets")
            .join("gltf")
            .join("DamagedHelmet.gltf")
            .display()
    );
}
