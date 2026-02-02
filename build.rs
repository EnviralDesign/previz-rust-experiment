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
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let filament_src_dir = manifest_dir.join("third_party").join("filament");
    if !filament_src_dir.exists() {
        panic!(
            "Filament source repo not found at {:?}. Clone it into third_party/filament.",
            filament_src_dir
        );
    }
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
#include <filament/IndirectLight.h>
#include <filament/Skybox.h>
#include <filament/Texture.h>
#include <filament/LightManager.h>
#include <filament/TransformManager.h>
#include <filament/Box.h>
#include <math/mat4.h>
#include <filagui/ImGuiHelper.h>
#include <imgui.h>
#include <filament/VertexBuffer.h>
#include <filament/IndexBuffer.h>
#include <filament/RenderableManager.h>
#include <cstring>
#include <cmath>
#include <filament/TransformManager.h>
#include <gltfio/AssetLoader.h>
#include <gltfio/FilamentAsset.h>
#include <gltfio/FilamentInstance.h>
#include <gltfio/MaterialProvider.h>
#include <gltfio/ResourceLoader.h>
#include <gltfio/TextureProvider.h>
#include <utils/EntityManager.h>
#include <backend/DriverEnums.h>
#include <image/Ktx1Bundle.h>
#include <ktxreader/Ktx1Reader.h>
#include <fstream>
#include <vector>

using namespace filament;
using namespace utils;
using namespace filament::gltfio;

static bool read_file_bytes(const char* path, std::vector<uint8_t>& out) {
    if (!path || !path[0]) {
        return false;
    }
    std::ifstream file(path, std::ios::binary | std::ios::ate);
    if (!file) {
        return false;
    }
    const auto size = file.tellg();
    if (size <= 0) {
        return false;
    }
    out.resize(static_cast<size_t>(size));
    file.seekg(0, std::ios::beg);
    if (!file.read(reinterpret_cast<char*>(out.data()), size)) {
        out.clear();
        return false;
    }
    return true;
}

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

void filament_scene_set_indirect_light(Scene* scene, IndirectLight* light) {
    scene->setIndirectLight(light);
}

void filament_scene_set_skybox(Scene* scene, Skybox* skybox) {
    scene->setSkybox(skybox);
}

// ============================================================================
// Environment
// ============================================================================

IndirectLight* filament_create_indirect_light_from_ktx(
    Engine* engine,
    const char* ktx_path,
    float intensity,
    Texture** out_texture
) {
    if (!engine || !ktx_path || !out_texture) {
        return nullptr;
    }
    std::vector<uint8_t> bytes;
    if (!read_file_bytes(ktx_path, bytes)) {
        return nullptr;
    }
    auto* bundle = new image::Ktx1Bundle(bytes.data(), (uint32_t)bytes.size());
    filament::math::float3 sh[9];
    bool has_sh = bundle->getSphericalHarmonics(sh);
    Texture* texture = ktxreader::Ktx1Reader::createTexture(engine, bundle, false);
    if (!texture) {
        return nullptr;
    }
    IndirectLight::Builder builder;
    builder.reflections(texture).intensity(intensity);
    if (has_sh) {
        builder.irradiance(3, sh);
    }
    IndirectLight* light = builder.build(*engine);
    *out_texture = texture;
    return light;
}

Skybox* filament_create_skybox_from_ktx(
    Engine* engine,
    const char* ktx_path,
    Texture** out_texture
) {
    if (!engine || !ktx_path || !out_texture) {
        return nullptr;
    }
    std::vector<uint8_t> bytes;
    if (!read_file_bytes(ktx_path, bytes)) {
        return nullptr;
    }
    auto* bundle = new image::Ktx1Bundle(bytes.data(), (uint32_t)bytes.size());
    Texture* texture = ktxreader::Ktx1Reader::createTexture(engine, bundle, true);
    if (!texture) {
        return nullptr;
    }
    Skybox* skybox = Skybox::Builder().environment(texture).build(*engine);
    *out_texture = texture;
    return skybox;
}

void filament_indirect_light_set_intensity(IndirectLight* light, float intensity) {
    if (light) {
        light->setIntensity(intensity);
    }
}

void filament_engine_destroy_indirect_light(Engine* engine, IndirectLight* light) {
    if (engine && light) {
        engine->destroy(light);
    }
}

void filament_engine_destroy_skybox(Engine* engine, Skybox* skybox) {
    if (engine && skybox) {
        engine->destroy(skybox);
    }
}

void filament_engine_destroy_texture(Engine* engine, Texture* texture) {
    if (engine && texture) {
        engine->destroy(texture);
    }
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

const char* filament_material_instance_get_name(MaterialInstance* instance) {
    if (!instance) {
        return nullptr;
    }
    return instance->getName();
}

bool filament_material_instance_has_parameter(MaterialInstance* instance, const char* name) {
    if (!instance || !name) {
        return false;
    }
    Material const* material = instance->getMaterial();
    return material ? material->hasParameter(name) : false;
}

void filament_material_instance_set_float(MaterialInstance* instance, const char* name, float value) {
    if (!instance || !name) {
        return;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return;
    }
    instance->setParameter(name, value);
}

void filament_material_instance_set_float3(
    MaterialInstance* instance,
    const char* name,
    float x,
    float y,
    float z
) {
    if (!instance || !name) {
        return;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return;
    }
    instance->setParameter(name, filament::math::float3{x, y, z});
}

void filament_material_instance_set_float4(
    MaterialInstance* instance,
    const char* name,
    float x,
    float y,
    float z,
    float w
) {
    if (!instance || !name) {
        return;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return;
    }
    instance->setParameter(name, filament::math::float4{x, y, z, w});
}

bool filament_material_instance_get_float(
    MaterialInstance* instance,
    const char* name,
    float* out_value
) {
    if (!instance || !name || !out_value) {
        return false;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return false;
    }
    *out_value = instance->getParameter<float>(name);
    return true;
}

bool filament_material_instance_get_float3(
    MaterialInstance* instance,
    const char* name,
    float* out_value
) {
    if (!instance || !name || !out_value) {
        return false;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return false;
    }
    filament::math::float3 value = instance->getParameter<filament::math::float3>(name);
    out_value[0] = value.x;
    out_value[1] = value.y;
    out_value[2] = value.z;
    return true;
}

bool filament_material_instance_get_float4(
    MaterialInstance* instance,
    const char* name,
    float* out_value
) {
    if (!instance || !name || !out_value) {
        return false;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return false;
    }
    filament::math::float4 value = instance->getParameter<filament::math::float4>(name);
    out_value[0] = value.x;
    out_value[1] = value.y;
    out_value[2] = value.z;
    out_value[3] = value.w;
    return true;
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

void filament_light_set_directional(
    Engine* engine,
    int32_t entity_id,
    float color_r,
    float color_g,
    float color_b,
    float intensity,
    float dir_x,
    float dir_y,
    float dir_z
) {
    Entity entity = Entity::import(entity_id);
    auto& lm = engine->getLightManager();
    if (!lm.hasComponent(entity)) {
        return;
    }
    auto instance = lm.getInstance(entity);
    lm.setColor(instance, {color_r, color_g, color_b});
    lm.setIntensity(instance, intensity);
    lm.setDirection(instance, {dir_x, dir_y, dir_z});
}

// ============================================================================
// Transforms
// ============================================================================

void filament_transform_manager_set_transform(
    TransformManager* tm,
    int32_t entity_id,
    const float* matrix4x4
) {
    if (!tm || !matrix4x4) {
        return;
    }
    Entity entity = Entity::import(entity_id);
    if (!tm->hasComponent(entity)) {
        return;
    }
    auto instance = tm->getInstance(entity);
    filament::math::mat4f matrix;
    std::memcpy(&matrix, matrix4x4, sizeof(float) * 16);
    tm->setTransform(instance, matrix);
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

int32_t filament_gltfio_asset_get_root(FilamentAsset* asset) {
    Entity entity = asset->getRoot();
    return Entity::smuggle(entity);
}

FilamentInstance* filament_gltfio_asset_get_instance(FilamentAsset* asset) {
    if (!asset) {
        return nullptr;
    }
    return asset->getInstance();
}

int32_t filament_gltfio_instance_get_material_instance_count(FilamentInstance* instance) {
    if (!instance) {
        return 0;
    }
    return static_cast<int32_t>(instance->getMaterialInstanceCount());
}

MaterialInstance* filament_gltfio_instance_get_material_instance(
    FilamentInstance* instance,
    int32_t index
) {
    if (!instance) {
        return nullptr;
    }
    const size_t count = instance->getMaterialInstanceCount();
    if (index < 0 || static_cast<size_t>(index) >= count) {
        return nullptr;
    }
    return instance->getMaterialInstances()[index];
}

// ============================================================================
// filagui
// ============================================================================

filagui::ImGuiHelper* filagui_imgui_helper_create(
    Engine* engine,
    View* view,
    const char* font_path
) {
    utils::Path fontPath(font_path ? font_path : "");
    return new filagui::ImGuiHelper(engine, view, fontPath);
}

void filagui_imgui_helper_destroy(filagui::ImGuiHelper* helper) {
    delete helper;
}

void filagui_imgui_helper_set_display_size(
    filagui::ImGuiHelper* helper,
    int width,
    int height,
    float scale_x,
    float scale_y,
    bool flip_vertical
) {
    if (helper) {
        helper->setDisplaySize(width, height, scale_x, scale_y, flip_vertical);
    }
}

static inline void filagui_imgui_helper_set_context(filagui::ImGuiHelper* helper) {
    if (!helper) {
        return;
    }
    ImGui::SetCurrentContext(helper->getImGuiContext());
}

void filagui_imgui_helper_add_mouse_pos(
    filagui::ImGuiHelper* helper,
    float x,
    float y
) {
    if (!helper) {
        return;
    }
    filagui_imgui_helper_set_context(helper);
    ImGuiIO& io = ImGui::GetIO();
#if IMGUI_VERSION_NUM >= 18700
    io.AddMousePosEvent(x, y);
#else
    io.MousePos = ImVec2(x, y);
#endif
}

void filagui_imgui_helper_add_mouse_button(
    filagui::ImGuiHelper* helper,
    int button,
    bool down
) {
    if (!helper) {
        return;
    }
    filagui_imgui_helper_set_context(helper);
    ImGuiIO& io = ImGui::GetIO();
#if IMGUI_VERSION_NUM >= 18700
    io.AddMouseButtonEvent(button, down);
#else
    if (button >= 0 && button < 5) {
        io.MouseDown[button] = down;
    }
#endif
}

void filagui_imgui_helper_add_mouse_wheel(
    filagui::ImGuiHelper* helper,
    float wheel_x,
    float wheel_y
) {
    if (!helper) {
        return;
    }
    filagui_imgui_helper_set_context(helper);
    ImGuiIO& io = ImGui::GetIO();
#if IMGUI_VERSION_NUM >= 18700
    io.AddMouseWheelEvent(wheel_x, wheel_y);
#else
    io.MouseWheelH += wheel_x;
    io.MouseWheel += wheel_y;
#endif
}

void filagui_imgui_helper_add_key_event(
    filagui::ImGuiHelper* helper,
    int key,
    bool down
) {
    if (!helper) {
        return;
    }
    filagui_imgui_helper_set_context(helper);
    ImGuiIO& io = ImGui::GetIO();
#if IMGUI_VERSION_NUM >= 18700
    io.AddKeyEvent((ImGuiKey)key, down);
#else
    if (key >= 0 && key < IM_ARRAYSIZE(io.KeysDown)) {
        io.KeysDown[key] = down;
    }
#endif
}

void filagui_imgui_helper_add_input_character(
    filagui::ImGuiHelper* helper,
    unsigned int codepoint
) {
    if (!helper) {
        return;
    }
    filagui_imgui_helper_set_context(helper);
    ImGuiIO& io = ImGui::GetIO();
    io.AddInputCharacter(codepoint);
}

bool filagui_imgui_helper_want_capture_mouse(filagui::ImGuiHelper* helper) {
    if (!helper) {
        return false;
    }
    filagui_imgui_helper_set_context(helper);
    return ImGui::GetIO().WantCaptureMouse;
}

bool filagui_imgui_helper_want_capture_keyboard(filagui::ImGuiHelper* helper) {
    if (!helper) {
        return false;
    }
    filagui_imgui_helper_set_context(helper);
    return ImGui::GetIO().WantCaptureKeyboard;
}

void filagui_imgui_helper_render_text(
    filagui::ImGuiHelper* helper,
    float delta_seconds,
    const char* title,
    const char* body
) {
    if (!helper) {
        return;
    }
    helper->render(delta_seconds, [title, body](filament::Engine*, filament::View*) {
        ImGui::Begin(title ? title : "Overlay");
        if (body) {
            ImGui::TextUnformatted(body);
        }
        ImGui::End();
    });
}

void filagui_imgui_helper_render_controls(
    filagui::ImGuiHelper* helper,
    float delta_seconds
) {
    if (!helper) {
        return;
    }
    helper->render(delta_seconds, [](filament::Engine*, filament::View*) {
        static char name[128] = "";
        static float intensity = 0.5f;
        ImGuiIO& io = ImGui::GetIO();

        ImGui::SetNextWindowSize(ImVec2(520, 320), ImGuiCond_Always);
        ImGui::Begin("Controls");
        ImGui::InputText("Name", name, sizeof(name));
        ImGui::SliderFloat("Intensity", &intensity, 0.0f, 1.0f);
        ImGui::Text("Editable test field above.");
        ImGui::Separator();
        ImGui::Text("io.MousePos: %.1f, %.1f", io.MousePos.x, io.MousePos.y);
        ImGui::Text("io.MouseDown: L=%d R=%d M=%d",
                io.MouseDown[0] ? 1 : 0, io.MouseDown[1] ? 1 : 0, io.MouseDown[2] ? 1 : 0);
        ImGui::Text("io.WantCaptureMouse: %d", io.WantCaptureMouse ? 1 : 0);
        ImGui::Text("io.WantCaptureKeyboard: %d", io.WantCaptureKeyboard ? 1 : 0);
        ImGui::Text("io.DisplaySize: %.1f, %.1f", io.DisplaySize.x, io.DisplaySize.y);
        ImGui::Text("io.DisplayFramebufferScale: %.2f, %.2f",
                io.DisplayFramebufferScale.x, io.DisplayFramebufferScale.y);
        ImGui::End();
    });
}

void filagui_imgui_helper_render_overlay(
    filagui::ImGuiHelper* helper,
    float delta_seconds,
    const char* title,
    const char* body
) {
    if (!helper) {
        return;
    }
    helper->render(delta_seconds, [title, body](filament::Engine*, filament::View*) {
        ImGuiIO& io = ImGui::GetIO();

        ImGui::SetNextWindowPos(ImVec2(12, 12), ImGuiCond_FirstUseEver);
        ImGui::Begin(title ? title : "Assets");
        if (body) {
            ImGui::TextUnformatted(body);
        }
        ImGui::End();

        static char name[128] = "";
        static float intensity = 0.5f;
        ImGui::SetNextWindowPos(ImVec2(12, 220), ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(520, 240), ImGuiCond_FirstUseEver);
        ImGui::Begin("Controls");
        ImGui::InputText("Name", name, sizeof(name));
        ImGui::SliderFloat("Intensity", &intensity, 0.0f, 1.0f);
        ImGui::Text("Editable test field above.");
        ImGui::End();
    });
}

void filagui_imgui_helper_render_scene_ui(
    filagui::ImGuiHelper* helper,
    float delta_seconds,
    const char* assets_title,
    const char* assets_body,
    const char** object_names,
    int object_count,
    int* selected_index,
    int* selected_kind,
    bool* can_edit_transform,
    float* position_xyz,
    float* rotation_deg_xyz,
    float* scale_xyz,
    float* light_color_rgb,
    float* light_intensity,
    float* light_dir_xyz,
    const char** material_names,
    int material_count,
    int* selected_material_index,
    float* material_base_color_rgba,
    float* material_metallic,
    float* material_roughness,
    float* material_emissive_rgb,
    char* hdr_path,
    int hdr_path_capacity,
    char* ibl_path,
    int ibl_path_capacity,
    char* skybox_path,
    int skybox_path_capacity,
    float* environment_intensity,
    bool* environment_apply,
    bool* environment_generate
) {
    if (!helper) {
        return;
    }
    helper->render(delta_seconds, [=](filament::Engine*, filament::View*) {
        const ImGuiViewport* viewport = ImGui::GetMainViewport();
        ImVec2 work_pos = viewport->WorkPos;
        ImVec2 work_size = viewport->WorkSize;
        float left_width = work_size.x * 0.22f;
        float right_width = work_size.x * 0.30f;
        float gutter = 12.0f;

        ImGui::SetNextWindowPos(work_pos, ImGuiCond_FirstUseEver);
        ImGui::SetNextWindowSize(ImVec2(left_width, work_size.y), ImGuiCond_FirstUseEver);
        ImGui::Begin("Hierarchy");
        if (!object_names || object_count <= 0) {
            ImGui::TextUnformatted("No objects loaded.");
        } else {
            int current = selected_index ? *selected_index : -1;
            if (current < 0 || current >= object_count) {
                current = -1;
            }
            for (int i = 0; i < object_count; ++i) {
                const char* name = object_names[i] ? object_names[i] : "Object";
                bool selected = (i == current);
                if (ImGui::Selectable(name, selected)) {
                    if (selected_index) {
                        *selected_index = i;
                    }
                    current = i;
                }
            }
        }
        ImGui::End();

        ImGui::SetNextWindowPos(
            ImVec2(work_pos.x + work_size.x - right_width - gutter, work_pos.y),
            ImGuiCond_FirstUseEver
        );
        ImGui::SetNextWindowSize(ImVec2(right_width, work_size.y), ImGuiCond_FirstUseEver);
        int current = selected_index ? *selected_index : -1;
        const char* selected_name = "None";
        if (current >= 0 && current < object_count && object_names) {
            const char* name = object_names[current];
            if (name) {
                selected_name = name;
            }
        }
        ImGui::Begin("Inspector");
        ImGui::Text("Inspector - %s", selected_name);
        ImGui::Separator();

        bool show_transform = selected_kind && *selected_kind == 0;
        if (show_transform && ImGui::CollapsingHeader("Transform", ImGuiTreeNodeFlags_DefaultOpen)) {
            bool has_selection = selected_index && *selected_index >= 0;
            bool allow_transform = has_selection && (!can_edit_transform || *can_edit_transform);
            if (!allow_transform) {
                ImGui::BeginDisabled();
            }
            if (position_xyz) {
                ImGui::InputFloat3("Position", position_xyz, "%.3f");
            }
            if (rotation_deg_xyz) {
                ImGui::InputFloat3("Rotation (deg)", rotation_deg_xyz, "%.2f");
            }
            if (scale_xyz) {
                ImGui::InputFloat3("Scale", scale_xyz, "%.3f");
            }
            if (!allow_transform) {
                ImGui::EndDisabled();
            }
        }

        bool show_lighting = selected_kind && *selected_kind == 1;
        if (show_lighting && ImGui::CollapsingHeader("Lighting", ImGuiTreeNodeFlags_DefaultOpen)) {
            if (light_color_rgb) {
                ImGui::ColorEdit3("Color", light_color_rgb);
            }
            if (light_intensity) {
                ImGui::SliderFloat("Intensity", light_intensity, 0.0f, 200000.0f, "%.1f");
            }
            if (light_dir_xyz) {
                ImGui::InputFloat3("Direction", light_dir_xyz, "%.3f");
                ImGui::SameLine();
                if (ImGui::Button("Normalize")) {
                    float x = light_dir_xyz[0];
                    float y = light_dir_xyz[1];
                    float z = light_dir_xyz[2];
                    float len = std::sqrt(x * x + y * y + z * z);
                    if (len > 1e-6f) {
                        light_dir_xyz[0] = x / len;
                        light_dir_xyz[1] = y / len;
                        light_dir_xyz[2] = z / len;
                    }
                }
            }
        }

        bool show_materials = selected_kind && *selected_kind == 0;
        if (show_materials && ImGui::CollapsingHeader("Materials", ImGuiTreeNodeFlags_DefaultOpen)) {
            if (!material_names || material_count <= 0) {
                ImGui::TextUnformatted("No materials loaded.");
            } else {
                int current = selected_material_index ? *selected_material_index : -1;
                if (current < 0 || current >= material_count) {
                    current = -1;
                }
                for (int i = 0; i < material_count; ++i) {
                    const char* name = material_names[i] ? material_names[i] : "Material";
                    bool selected = (i == current);
                    if (ImGui::Selectable(name, selected)) {
                        if (selected_material_index) {
                            *selected_material_index = i;
                        }
                        current = i;
                    }
                }
            }
            ImGui::Separator();
            bool has_material = selected_material_index && *selected_material_index >= 0;
            if (!has_material) {
                ImGui::BeginDisabled();
            }
            if (material_base_color_rgba) {
                ImGui::ColorEdit4("Base Color", material_base_color_rgba);
            }
            if (material_metallic) {
                ImGui::SliderFloat("Metallic", material_metallic, 0.0f, 1.0f, "%.3f");
            }
            if (material_roughness) {
                ImGui::SliderFloat("Roughness", material_roughness, 0.0f, 1.0f, "%.3f");
            }
            if (material_emissive_rgb) {
                ImGui::ColorEdit3("Emissive", material_emissive_rgb);
            }
            if (!has_material) {
                ImGui::EndDisabled();
            }
        }

        bool show_environment = selected_kind && *selected_kind == 2;
        if (show_environment && ImGui::CollapsingHeader("Environment", ImGuiTreeNodeFlags_DefaultOpen)) {
            if (hdr_path && hdr_path_capacity > 0) {
                ImGui::InputText("Equirect HDR", hdr_path, (size_t)hdr_path_capacity);
            }
            if (ibl_path && ibl_path_capacity > 0) {
                ImGui::InputText("IBL KTX", ibl_path, (size_t)ibl_path_capacity);
            }
            if (skybox_path && skybox_path_capacity > 0) {
                ImGui::InputText("Skybox KTX", skybox_path, (size_t)skybox_path_capacity);
            }
            if (environment_intensity) {
                ImGui::SliderFloat("Intensity", environment_intensity, 0.0f, 200000.0f, "%.1f");
            }
            if (environment_generate) {
                *environment_generate = false;
                if (ImGui::Button("Generate KTX")) {
                    *environment_generate = true;
                }
            }
            ImGui::SameLine();
            if (environment_apply) {
                *environment_apply = false;
                if (ImGui::Button("Load Environment")) {
                    *environment_apply = true;
                }
            }
        }

        ImGui::End();
    });
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
pub type Texture = c_void;
pub type IndirectLight = c_void;
pub type Skybox = c_void;
pub type VertexBuffer = c_void;
pub type IndexBuffer = c_void;
pub type MaterialProvider = c_void;
pub type AssetLoader = c_void;
pub type ResourceLoader = c_void;
pub type TextureProvider = c_void;
pub type FilamentAsset = c_void;
pub type FilamentInstance = c_void;
pub type ImGuiHelper = c_void;

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
    pub fn filament_scene_set_indirect_light(scene: *mut Scene, light: *mut IndirectLight);
    pub fn filament_scene_set_skybox(scene: *mut Scene, skybox: *mut Skybox);

    // ========================================================================
    // Environment
    // ========================================================================
    
    pub fn filament_create_indirect_light_from_ktx(
        engine: *mut Engine,
        ktx_path: *const c_char,
        intensity: f32,
        out_texture: *mut *mut Texture,
    ) -> *mut IndirectLight;
    pub fn filament_create_skybox_from_ktx(
        engine: *mut Engine,
        ktx_path: *const c_char,
        out_texture: *mut *mut Texture,
    ) -> *mut Skybox;
    pub fn filament_indirect_light_set_intensity(light: *mut IndirectLight, intensity: f32);
    pub fn filament_engine_destroy_indirect_light(engine: *mut Engine, light: *mut IndirectLight);
    pub fn filament_engine_destroy_skybox(engine: *mut Engine, skybox: *mut Skybox);
    pub fn filament_engine_destroy_texture(engine: *mut Engine, texture: *mut Texture);
    
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
    pub fn filament_material_instance_get_name(instance: *mut MaterialInstance) -> *const c_char;
    pub fn filament_material_instance_has_parameter(
        instance: *mut MaterialInstance,
        name: *const c_char,
    ) -> bool;
    pub fn filament_material_instance_set_float(
        instance: *mut MaterialInstance,
        name: *const c_char,
        value: f32,
    );
    pub fn filament_material_instance_set_float3(
        instance: *mut MaterialInstance,
        name: *const c_char,
        x: f32,
        y: f32,
        z: f32,
    );
    pub fn filament_material_instance_set_float4(
        instance: *mut MaterialInstance,
        name: *const c_char,
        x: f32,
        y: f32,
        z: f32,
        w: f32,
    );
    pub fn filament_material_instance_get_float(
        instance: *mut MaterialInstance,
        name: *const c_char,
        out_value: *mut f32,
    ) -> bool;
    pub fn filament_material_instance_get_float3(
        instance: *mut MaterialInstance,
        name: *const c_char,
        out_value: *mut f32,
    ) -> bool;
    pub fn filament_material_instance_get_float4(
        instance: *mut MaterialInstance,
        name: *const c_char,
        out_value: *mut f32,
    ) -> bool;
    
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
    pub fn filament_light_set_directional(
        engine: *mut Engine,
        entity_id: i32,
        color_r: f32,
        color_g: f32,
        color_b: f32,
        intensity: f32,
        dir_x: f32,
        dir_y: f32,
        dir_z: f32,
    );

    // ========================================================================
    // Transforms
    // ========================================================================
    pub fn filament_transform_manager_set_transform(
        tm: *mut TransformManager,
        entity_id: i32,
        matrix4x4: *const f32,
    );

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
    pub fn filament_gltfio_asset_get_root(asset: *mut FilamentAsset) -> i32;
    pub fn filament_gltfio_asset_get_instance(asset: *mut FilamentAsset) -> *mut FilamentInstance;
    pub fn filament_gltfio_instance_get_material_instance_count(
        instance: *mut FilamentInstance,
    ) -> i32;
    pub fn filament_gltfio_instance_get_material_instance(
        instance: *mut FilamentInstance,
        index: i32,
    ) -> *mut MaterialInstance;

    // ========================================================================
    // filagui
    // ========================================================================

    pub fn filagui_imgui_helper_create(
        engine: *mut Engine,
        view: *mut View,
        font_path: *const c_char,
    ) -> *mut ImGuiHelper;
    pub fn filagui_imgui_helper_destroy(helper: *mut ImGuiHelper);
    pub fn filagui_imgui_helper_set_display_size(
        helper: *mut ImGuiHelper,
        width: i32,
        height: i32,
        scale_x: f32,
        scale_y: f32,
        flip_vertical: bool,
    );
    pub fn filagui_imgui_helper_add_mouse_pos(
        helper: *mut ImGuiHelper,
        x: f32,
        y: f32,
    );
    pub fn filagui_imgui_helper_add_mouse_button(
        helper: *mut ImGuiHelper,
        button: i32,
        down: bool,
    );
    pub fn filagui_imgui_helper_add_mouse_wheel(
        helper: *mut ImGuiHelper,
        wheel_x: f32,
        wheel_y: f32,
    );
    pub fn filagui_imgui_helper_add_key_event(
        helper: *mut ImGuiHelper,
        key: i32,
        down: bool,
    );
    pub fn filagui_imgui_helper_add_input_character(
        helper: *mut ImGuiHelper,
        codepoint: u32,
    );
    pub fn filagui_imgui_helper_want_capture_mouse(helper: *mut ImGuiHelper) -> bool;
    pub fn filagui_imgui_helper_want_capture_keyboard(helper: *mut ImGuiHelper) -> bool;
    pub fn filagui_imgui_helper_render_text(
        helper: *mut ImGuiHelper,
        delta_seconds: f32,
        title: *const c_char,
        body: *const c_char,
    );
    pub fn filagui_imgui_helper_render_controls(
        helper: *mut ImGuiHelper,
        delta_seconds: f32,
    );
    pub fn filagui_imgui_helper_render_overlay(
        helper: *mut ImGuiHelper,
        delta_seconds: f32,
        title: *const c_char,
        body: *const c_char,
    );
    pub fn filagui_imgui_helper_render_scene_ui(
        helper: *mut ImGuiHelper,
        delta_seconds: f32,
        assets_title: *const c_char,
        assets_body: *const c_char,
        object_names: *const *const c_char,
        object_count: i32,
        selected_index: *mut i32,
        selected_kind: *mut i32,
        can_edit_transform: *mut bool,
        position_xyz: *mut f32,
        rotation_deg_xyz: *mut f32,
        scale_xyz: *mut f32,
        light_color_rgb: *mut f32,
        light_intensity: *mut f32,
        light_dir_xyz: *mut f32,
        material_names: *const *const c_char,
        material_count: i32,
        selected_material_index: *mut i32,
        material_base_color_rgba: *mut f32,
        material_metallic: *mut f32,
        material_roughness: *mut f32,
        material_emissive_rgb: *mut f32,
        hdr_path: *mut c_char,
        hdr_path_capacity: i32,
        ibl_path: *mut c_char,
        ibl_path_capacity: i32,
        skybox_path: *mut c_char,
        skybox_path_capacity: i32,
        environment_intensity: *mut f32,
        environment_apply: *mut bool,
        environment_generate: *mut bool,
    );
}
"#.to_string()
}

fn main() {
    // Only build on Windows for now
    #[cfg(not(target_os = "windows"))]
    compile_error!("This build script currently only supports Windows");

    let out_dir = PathBuf::from(env::var("OUT_DIR").unwrap());
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let filament_src_dir = manifest_dir.join("third_party").join("filament");
    if !filament_src_dir.exists() {
        panic!(
            "Filament source repo not found at {:?}. Clone it into third_party/filament.",
            filament_src_dir
        );
    }
    
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
    let filagui_include_dir = filament_src_dir.join("libs").join("filagui").join("include");
    let imgui_dir = filament_src_dir.join("third_party").join("imgui");
    cc::Build::new()
        .cpp(true)
        .file(&bindings_cpp)
        .include(&include_dir)
        .include(&filagui_include_dir)
        .include(&imgui_dir)
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

    // Step 4.6: Build filagui resources and static lib
    let (filagui_generation_root, filagui_resource_dir) =
        compile_filagui_resources(&filament_dir, &filament_src_dir, &out_dir);
    let filagui_resources_c = filagui_resource_dir.join("filagui_resources.c");
    let filagui_src_dir = filament_src_dir.join("libs").join("filagui").join("src");
    let imgui_cpp_dir = imgui_dir.clone();
    println!("cargo:warning=Compiling filagui...");
    cc::Build::new()
        .cpp(true)
        .file(filagui_src_dir.join("ImGuiHelper.cpp"))
        .file(filagui_src_dir.join("ImGuiExtensions.cpp"))
        .file(imgui_cpp_dir.join("imgui.cpp"))
        .file(imgui_cpp_dir.join("imgui_draw.cpp"))
        .file(imgui_cpp_dir.join("imgui_tables.cpp"))
        .file(imgui_cpp_dir.join("imgui_widgets.cpp"))
        .file(filagui_resources_c)
        .include(&include_dir)
        .include(&filagui_include_dir)
        .include(&imgui_dir)
        .include(&filagui_generation_root)
        .flag("/std:c++20")
        .flag("/EHsc")
        .flag("/MD")
        .warnings(false)
        .compile("filagui_bindings");
    
    // Step 5: Link Filament libraries
    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!(
        "cargo:rustc-env=FILAMENT_BIN_DIR={}",
        filament_dir.join("bin").display()
    );
    
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
    println!("cargo:rustc-link-lib=static=filagui_bindings");
    
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
        manifest_dir.join("assets").join("bakedColor.mat").display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        manifest_dir
            .join("assets")
            .join("gltf")
            .join("DamagedHelmet.gltf")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("ImGuiHelper.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("ImGuiExtensions.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("libs")
            .join("filagui")
            .join("src")
            .join("materials")
            .join("uiBlit.mat")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_draw.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_tables.cpp")
            .display()
    );
    println!(
        "cargo:rerun-if-changed={}",
        filament_src_dir
            .join("third_party")
            .join("imgui")
            .join("imgui_widgets.cpp")
            .display()
    );
}
