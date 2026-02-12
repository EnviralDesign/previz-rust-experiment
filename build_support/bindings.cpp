
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
#include <filament/TextureSampler.h>
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
#include <cfloat>
#include <filament/TransformManager.h>
#include <gltfio/AssetLoader.h>
#include <gltfio/FilamentAsset.h>
#include <gltfio/FilamentInstance.h>
#include <gltfio/MaterialProvider.h>
#include <gltfio/ResourceLoader.h>
#include <gltfio/TextureProvider.h>
#include <utils/EntityManager.h>
#include <backend/DriverEnums.h>
#include <backend/PixelBufferDescriptor.h>
#include <filament/RenderTarget.h>
#include <image/Ktx1Bundle.h>
#include <ktxreader/Ktx1Reader.h>
#include <fstream>
#include <vector>
#include <atomic>

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

bool filament_material_instance_set_texture_from_ktx(
    Engine* engine,
    MaterialInstance* instance,
    const char* name,
    const char* ktx_path,
    bool wrap_repeat_u,
    bool wrap_repeat_v,
    Texture** out_texture
) {
    if (!engine || !instance || !name || !ktx_path || !out_texture) {
        return false;
    }
    Material const* material = instance->getMaterial();
    if (!material || !material->hasParameter(name)) {
        return false;
    }
    std::vector<uint8_t> bytes;
    if (!read_file_bytes(ktx_path, bytes)) {
        return false;
    }
    auto* bundle = new image::Ktx1Bundle(bytes.data(), (uint32_t)bytes.size());
    Texture* texture = ktxreader::Ktx1Reader::createTexture(engine, bundle, false);
    if (!texture) {
        return false;
    }
    TextureSampler sampler;
    sampler.setWrapModeS(
        wrap_repeat_u ? TextureSampler::WrapMode::REPEAT : TextureSampler::WrapMode::CLAMP_TO_EDGE
    );
    sampler.setWrapModeT(
        wrap_repeat_v ? TextureSampler::WrapMode::REPEAT : TextureSampler::WrapMode::CLAMP_TO_EDGE
    );
    instance->setParameter(name, texture, sampler);
    *out_texture = texture;
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
    const char** material_binding_param_names,
    int material_binding_count,
    char* material_binding_sources,
    int material_binding_source_stride,
    bool* material_binding_wrap_repeat_u,
    bool* material_binding_wrap_repeat_v,
    bool* material_binding_srgb,
    float* material_binding_uv_offset,
    float* material_binding_uv_scale,
    float* material_binding_uv_rotation_deg,
    int* material_binding_pick_index,
    int* material_binding_apply_index,
    char* hdr_path,
    int hdr_path_capacity,
    char* ibl_path,
    int ibl_path_capacity,
    char* skybox_path,
    int skybox_path_capacity,
    bool* environment_pick_hdr,
    bool* environment_pick_ibl,
    bool* environment_pick_skybox,
    float* environment_intensity,
    bool* environment_apply,
    bool* environment_generate,
    bool* create_gltf,
    bool* create_light,
    bool* create_environment,
    bool* save_scene,
    bool* load_scene,
    int* transform_tool_mode,
    bool* delete_selected,
    const float* gizmo_screen_points_xy,
    bool gizmo_visible,
    const float* gizmo_origin_world_xyz,
    const float* camera_world_xyz,
    int* gizmo_active_axis
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

        // Left sidebar - single window with Main Menu and Hierarchy as groups
        ImGui::SetNextWindowPos(work_pos, ImGuiCond_Always);
        ImGui::SetNextWindowSize(ImVec2(left_width, work_size.y), ImGuiCond_Always);
        ImGui::Begin("Scene", nullptr, ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoResize);

        // Main Menu group
        if (ImGui::CollapsingHeader("Main Menu", ImGuiTreeNodeFlags_DefaultOpen)) {
            if (create_gltf) {
                *create_gltf = false;
                if (ImGui::Button("Load GLTF...", ImVec2(-1, 0))) {
                    *create_gltf = true;
                }
            }
            if (create_light) {
                *create_light = false;
                if (ImGui::Button("Add Light", ImVec2(-1, 0))) {
                    *create_light = true;
                }
            }
            if (create_environment) {
                *create_environment = false;
                if (ImGui::Button("Add Environment", ImVec2(-1, 0))) {
                    *create_environment = true;
                }
            }
            ImGui::Separator();
            if (save_scene) {
                *save_scene = false;
                if (ImGui::Button("Save Scene...", ImVec2(-1, 0))) {
                    *save_scene = true;
                }
            }
            if (load_scene) {
                *load_scene = false;
                if (ImGui::Button("Load Scene...", ImVec2(-1, 0))) {
                    *load_scene = true;
                }
            }
        }

        // Hierarchy group - takes remaining space
        if (ImGui::CollapsingHeader("Hierarchy", ImGuiTreeNodeFlags_DefaultOpen)) {
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
                ImGui::PushID(i);  // Ensure unique ID for each item
                if (ImGui::Selectable(name, selected)) {
                    if (selected_index) {
                        *selected_index = i;
                    }
                    current = i;
                }
                ImGui::PopID();
            }
                // Deselect when clicking in empty space below the list
                if (ImGui::IsWindowHovered(ImGuiHoveredFlags_RootAndChildWindows) &&
                    ImGui::IsMouseClicked(0) &&
                    !ImGui::IsAnyItemHovered()) {
                    if (selected_index) {
                        *selected_index = -1;
                    }
                }
            }
        }

        ImGui::End();

        ImGui::SetNextWindowPos(
            ImVec2(work_pos.x + work_size.x - right_width - gutter, work_pos.y),
            ImGuiCond_Always
        );
        ImGui::SetNextWindowSize(ImVec2(right_width, work_size.y), ImGuiCond_Always);
        int current = selected_index ? *selected_index : -1;
        const char* selected_name = "None";
        if (current >= 0 && current < object_count && object_names) {
            const char* name = object_names[current];
            if (name) {
                selected_name = name;
            }
        }
        ImGui::Begin("Inspector", nullptr, ImGuiWindowFlags_NoMove | ImGuiWindowFlags_NoResize);
        ImGui::Text("Inspector - %s", selected_name);
        ImGui::Separator();
        if (ImGui::CollapsingHeader("Tools", ImGuiTreeNodeFlags_DefaultOpen)) {
            ImGui::TextUnformatted("Shortcuts: Q/W/E/R tools, Delete removes selection");
            if (transform_tool_mode) {
                ImGui::TextUnformatted("Transform");
                int mode = *transform_tool_mode;
                if (ImGui::RadioButton("Select", mode == 0)) {
                    mode = 0;
                }
                ImGui::SameLine();
                if (ImGui::RadioButton("Translate", mode == 1)) {
                    mode = 1;
                }
                ImGui::SameLine();
                if (ImGui::RadioButton("Rotate", mode == 2)) {
                    mode = 2;
                }
                ImGui::SameLine();
                if (ImGui::RadioButton("Scale", mode == 3)) {
                    mode = 3;
                }
                *transform_tool_mode = mode;
            }
            if (delete_selected) {
                *delete_selected = false;
                if (ImGui::Button("Delete Selected", ImVec2(-1, 0))) {
                    *delete_selected = true;
                }
            }
            ImGui::Separator();
        }

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
            if (material_binding_pick_index) {
                *material_binding_pick_index = -1;
            }
            if (material_binding_apply_index) {
                *material_binding_apply_index = -1;
            }
            if (!material_names || material_count <= 0) {
                ImGui::TextUnformatted("No materials loaded.");
            } else {
                int current = selected_material_index ? *selected_material_index : -1;
                if (current < 0 || current >= material_count) {
                    current = 0;
                    if (selected_material_index) {
                        *selected_material_index = 0;
                    }
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
            ImGui::Separator();
            if (material_binding_param_names && material_binding_count > 0 &&
                material_binding_sources && material_binding_source_stride > 1) {
                ImGui::TextUnformatted("Texture Bindings");
                for (int i = 0; i < material_binding_count; ++i) {
                    const char* param_name = material_binding_param_names[i] ? material_binding_param_names[i] : "texture";
                    char* source = material_binding_sources + (i * material_binding_source_stride);
                    bool* wrap_u = material_binding_wrap_repeat_u ? &material_binding_wrap_repeat_u[i] : nullptr;
                    bool* wrap_v = material_binding_wrap_repeat_v ? &material_binding_wrap_repeat_v[i] : nullptr;
                    bool* srgb = material_binding_srgb ? &material_binding_srgb[i] : nullptr;
                    float* uv_offset = material_binding_uv_offset ? &material_binding_uv_offset[i * 2] : nullptr;
                    float* uv_scale = material_binding_uv_scale ? &material_binding_uv_scale[i * 2] : nullptr;
                    float* uv_rotation = material_binding_uv_rotation_deg ? &material_binding_uv_rotation_deg[i] : nullptr;

                    ImGui::PushID(i);
                    ImGui::SeparatorText(param_name);
                    float button_w = 32.0f;
                    float spacing = ImGui::GetStyle().ItemInnerSpacing.x;
                    ImGui::SetNextItemWidth(-button_w - spacing);
                    ImGui::InputText("##TextureSource", source, (size_t)material_binding_source_stride);
                    ImGui::SameLine();
                    if (ImGui::Button("...", ImVec2(button_w, 0))) {
                        if (material_binding_pick_index) {
                            *material_binding_pick_index = i;
                        }
                    }
                    if (srgb) {
                        ImGui::Checkbox("sRGB", srgb);
                        ImGui::SameLine();
                    }
                    if (wrap_u) {
                        ImGui::Checkbox("Wrap U", wrap_u);
                        ImGui::SameLine();
                    }
                    if (wrap_v) {
                        ImGui::Checkbox("Wrap V", wrap_v);
                    }
                    if (uv_offset) {
                        ImGui::DragFloat2("UV Offset", uv_offset, 0.001f, -100.0f, 100.0f, "%.3f");
                    }
                    if (uv_scale) {
                        ImGui::DragFloat2("UV Scale", uv_scale, 0.001f, -100.0f, 100.0f, "%.3f");
                    }
                    if (uv_rotation) {
                        ImGui::DragFloat("UV Rotation (deg)", uv_rotation, 0.1f, -360.0f, 360.0f, "%.2f");
                    }
                    if (ImGui::Button("Apply", ImVec2(-1, 0))) {
                        if (material_binding_apply_index) {
                            *material_binding_apply_index = i;
                        }
                    }
                    ImGui::PopID();
                }
            } else {
                ImGui::TextUnformatted("Texture binding rows unavailable.");
            }
            if (!has_material) {
                ImGui::EndDisabled();
            }
        }

        bool show_environment = selected_kind && *selected_kind == 2;
        if (show_environment && ImGui::CollapsingHeader("Environment", ImGuiTreeNodeFlags_DefaultOpen)) {
            float button_w = 32.0f;
            float spacing = ImGui::GetStyle().ItemInnerSpacing.x;
            if (hdr_path && hdr_path_capacity > 0) {
                ImGui::TextUnformatted("HDR Source");
                ImGui::SetNextItemWidth(-button_w - spacing);
                ImGui::InputText("##EnvHdr", hdr_path, (size_t)hdr_path_capacity);
                if (environment_pick_hdr) {
                    ImGui::SameLine();
                    *environment_pick_hdr = false;
                    if (ImGui::Button("...##PickHdr", ImVec2(button_w, 0))) {
                        *environment_pick_hdr = true;
                    }
                }
            }
            if (environment_intensity) {
                ImGui::SliderFloat("Intensity", environment_intensity, 0.0f, 200000.0f, "%.1f");
            }
            if (environment_apply) {
                *environment_apply = false;
                if (ImGui::Button("Apply HDR Environment")) {
                    *environment_apply = true;
                }
            }
        }

        // Viewport gizmo overlay: axis / plane / ring handles with mode-specific picking.
        if (gizmo_active_axis) {
            if (!ImGui::IsMouseDown(0)) {
                *gizmo_active_axis = 0;
            }
            if (gizmo_visible && gizmo_screen_points_xy && transform_tool_mode) {
                ImVec2 center(gizmo_screen_points_xy[0], gizmo_screen_points_xy[1]);
                ImVec2 x_end(gizmo_screen_points_xy[2], gizmo_screen_points_xy[3]);
                ImVec2 y_end(gizmo_screen_points_xy[4], gizmo_screen_points_xy[5]);
                ImVec2 z_end(gizmo_screen_points_xy[6], gizmo_screen_points_xy[7]);
                bool has_x = std::isfinite(x_end.x) && std::isfinite(x_end.y);
                bool has_y = std::isfinite(y_end.x) && std::isfinite(y_end.y);
                bool has_z = std::isfinite(z_end.x) && std::isfinite(z_end.y);
                auto* draw = ImGui::GetForegroundDrawList();
                if (*transform_tool_mode == 0) {
                    *gizmo_active_axis = 0;
                } else {
                    auto vsub = [](ImVec2 a, ImVec2 b) -> ImVec2 { return ImVec2(a.x - b.x, a.y - b.y); };
                    auto vadd = [](ImVec2 a, ImVec2 b) -> ImVec2 { return ImVec2(a.x + b.x, a.y + b.y); };
                    auto vmul = [](ImVec2 a, float s) -> ImVec2 { return ImVec2(a.x * s, a.y * s); };
                    auto vlen = [](ImVec2 v) -> float { return std::sqrt(v.x * v.x + v.y * v.y); };
                    auto vnorm = [&](ImVec2 v) -> ImVec2 {
                        float l = vlen(v);
                        if (l <= 1e-5f) return ImVec2(0, 0);
                        return ImVec2(v.x / l, v.y / l);
                    };
                    auto distance_to_segment = [](ImVec2 p, ImVec2 a, ImVec2 b) -> float {
                        float vx = b.x - a.x;
                        float vy = b.y - a.y;
                        float wx = p.x - a.x;
                        float wy = p.y - a.y;
                        float c1 = vx * wx + vy * wy;
                        if (c1 <= 0.0f) {
                            float dx = p.x - a.x;
                            float dy = p.y - a.y;
                            return std::sqrt(dx * dx + dy * dy);
                        }
                        float c2 = vx * vx + vy * vy;
                        if (c2 <= c1) {
                            float dx = p.x - b.x;
                            float dy = p.y - b.y;
                            return std::sqrt(dx * dx + dy * dy);
                        }
                        float t = c1 / c2;
                        ImVec2 proj(a.x + t * vx, a.y + t * vy);
                        float dx = p.x - proj.x;
                        float dy = p.y - proj.y;
                        return std::sqrt(dx * dx + dy * dy);
                    };
                    auto cross2 = [](ImVec2 a, ImVec2 b, ImVec2 c) -> float {
                        ImVec2 ab(b.x - a.x, b.y - a.y);
                        ImVec2 ac(c.x - a.x, c.y - a.y);
                        return ab.x * ac.y - ab.y * ac.x;
                    };
                    auto point_in_triangle = [&](ImVec2 p, ImVec2 a, ImVec2 b, ImVec2 c) -> bool {
                        float c1 = cross2(a, b, p);
                        float c2 = cross2(b, c, p);
                        float c3 = cross2(c, a, p);
                        bool has_neg = (c1 < 0) || (c2 < 0) || (c3 < 0);
                        bool has_pos = (c1 > 0) || (c2 > 0) || (c3 > 0);
                        return !(has_neg && has_pos);
                    };
                    auto point_in_quad = [&](ImVec2 p, ImVec2 a, ImVec2 b, ImVec2 c, ImVec2 d) -> bool {
                        return point_in_triangle(p, a, b, c) || point_in_triangle(p, a, c, d);
                    };
                    auto polyline_distance = [&](ImVec2 p, const std::vector<ImVec2>& pts, bool closed) -> float {
                        if (pts.size() < 2) {
                            return FLT_MAX;
                        }
                        float best = FLT_MAX;
                        for (size_t i = 0; i + 1 < pts.size(); ++i) {
                            best = std::min(best, distance_to_segment(p, pts[i], pts[i + 1]));
                        }
                        if (closed) {
                            best = std::min(best, distance_to_segment(p, pts.back(), pts.front()));
                        }
                        return best;
                    };
                    auto draw_arrow = [&](ImVec2 a, ImVec2 b, ImU32 color, float thickness) {
                        draw->AddLine(a, b, color, thickness);
                        ImVec2 dir = vnorm(vsub(b, a));
                        ImVec2 ortho(-dir.y, dir.x);
                        const float head_len = 12.0f;
                        const float head_w = 6.0f;
                        ImVec2 base = vsub(b, vmul(dir, head_len));
                        ImVec2 l = vadd(base, vmul(ortho, head_w));
                        ImVec2 r = vsub(base, vmul(ortho, head_w));
                        draw->AddTriangleFilled(b, l, r, color);
                    };
                    auto draw_square_head = [&](ImVec2 a, ImVec2 b, ImU32 color, float thickness) {
                        draw->AddLine(a, b, color, thickness);
                        ImVec2 dir = vnorm(vsub(b, a));
                        ImVec2 ortho(-dir.y, dir.x);
                        const float size = 6.0f;
                        ImVec2 c0 = vadd(vadd(b, vmul(dir, size)), vmul(ortho, size));
                        ImVec2 c1 = vadd(vadd(b, vmul(dir, size)), vmul(ortho, -size));
                        ImVec2 c2 = vadd(vadd(b, vmul(dir, -size)), vmul(ortho, -size));
                        ImVec2 c3 = vadd(vadd(b, vmul(dir, -size)), vmul(ortho, size));
                        draw->AddQuadFilled(c0, c1, c2, c3, color);
                    };
                    auto ring_points = [&](ImVec2 b1, ImVec2 b2, float radius_scale) -> std::vector<ImVec2> {
                        std::vector<ImVec2> pts;
                        const int segments = 64;
                        pts.reserve((size_t)segments);
                        for (int i = 0; i < segments; ++i) {
                            float t = (float)i / (float)segments;
                            float a = t * 6.28318530718f;
                            float ca = std::cos(a);
                            float sa = std::sin(a);
                            ImVec2 p = vadd(center, vadd(vmul(b1, ca * radius_scale), vmul(b2, sa * radius_scale)));
                            pts.push_back(p);
                        }
                        return pts;
                    };
                    auto normalize3 = [](float v[3]) {
                        float len = std::sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2]);
                        if (len > 1e-6f) {
                            v[0] /= len;
                            v[1] /= len;
                            v[2] /= len;
                        }
                    };
                    // Returns continuous dot-product of each ring sample's 3D
                    // normal against the view direction.  Positive = front hemisphere.
                    auto ring_front_dot = [&](int ring_axis, int count) -> std::vector<float> {
                        std::vector<float> dots;
                        dots.resize((size_t)count, 1.0f);
                        if (!gizmo_origin_world_xyz || !camera_world_xyz || count <= 0) {
                            return dots;
                        }
                        float view_dir[3] = {
                            camera_world_xyz[0] - gizmo_origin_world_xyz[0],
                            camera_world_xyz[1] - gizmo_origin_world_xyz[1],
                            camera_world_xyz[2] - gizmo_origin_world_xyz[2],
                        };
                        normalize3(view_dir);
                        for (int i = 0; i < count; ++i) {
                            float t = (float)i / (float)count;
                            float a = t * 6.28318530718f;
                            float ca = std::cos(a);
                            float sa = std::sin(a);
                            float n[3] = {0.0f, 0.0f, 0.0f};
                            if (ring_axis == 0) { n[1] = ca; n[2] = sa; }
                            else if (ring_axis == 1) { n[0] = ca; n[2] = sa; }
                            else { n[0] = ca; n[1] = sa; }
                            dots[(size_t)i] = n[0]*view_dir[0] + n[1]*view_dir[1] + n[2]*view_dir[2];
                        }
                        return dots;
                    };

                    // Exact-split pick distance: splits each segment at the precise
                    // hemisphere boundary (dot == 0) and inner-circle boundary so
                    // that only truly visible arc portions participate in picking.
                    auto ring_distance_clipped = [&](ImVec2 p, const std::vector<ImVec2>& pts, const std::vector<float>& dot_vals, float inner_r) -> float {
                        if (pts.size() < 2) return FLT_MAX;
                        float inner_r2 = inner_r * inner_r;
                        float best = FLT_MAX;
                        for (size_t i = 0; i < pts.size(); ++i) {
                            size_t j = (i + 1) % pts.size();
                            const ImVec2& sa = pts[i];
                            const ImVec2& sb = pts[j];
                            float da = dot_vals.empty() ? 1.0f : dot_vals[i];
                            float db = dot_vals.empty() ? 1.0f : dot_vals[j];
                            // Hemisphere visible t-range
                            float th0 = 0.0f, th1 = 1.0f;
                            if (da < 0.0f && db < 0.0f) continue;
                            if (da < 0.0f) { float tc = -da / (db - da); th0 = tc; }
                            else if (db < 0.0f) { float tc = -da / (db - da); th1 = tc; }
                            if (th0 >= th1) continue;
                            // Inner circle: |P(t)-center|^2 = inner_r^2
                            ImVec2 d0 = vsub(sa, center);
                            ImVec2 e = vsub(sb, sa);
                            float qA = e.x*e.x + e.y*e.y;
                            float qB = 2.0f*(d0.x*e.x + d0.y*e.y);
                            float qC = d0.x*d0.x + d0.y*d0.y - inner_r2;
                            auto dist_range = [&](float ts, float te) {
                                if (ts >= te) return;
                                ImVec2 pa(sa.x + ts*(sb.x-sa.x), sa.y + ts*(sb.y-sa.y));
                                ImVec2 pb(sa.x + te*(sb.x-sa.x), sa.y + te*(sb.y-sa.y));
                                best = std::min(best, distance_to_segment(p, pa, pb));
                            };
                            float disc = qB*qB - 4*qA*qC;
                            if (qA < 1e-10f || disc < 0.0f) {
                                if (qC >= 0.0f) dist_range(th0, th1);
                                continue;
                            }
                            float sq = std::sqrt(disc);
                            float t1 = (-qB - sq) / (2*qA);
                            float t2 = (-qB + sq) / (2*qA);
                            dist_range(th0, std::min(th1, t1));
                            dist_range(std::max(th0, t2), th1);
                        }
                        return best;
                    };

                    // Exact-split ring drawing: each segment is clipped to the
                    // front-hemisphere manifold (linear dot interpolation == 0) and
                    // to the inner occluder circle (quadratic in t), then only the
                    // visible sub-segments are emitted.
                    auto draw_ring_clipped = [&](const std::vector<ImVec2>& pts, const std::vector<float>& dot_vals, float inner_r, ImU32 color, float thick) {
                        if (pts.size() < 2) return;
                        float inner_r2 = inner_r * inner_r;
                        for (size_t i = 0; i < pts.size(); ++i) {
                            size_t j = (i + 1) % pts.size();
                            const ImVec2& sa = pts[i];
                            const ImVec2& sb = pts[j];
                            float da = dot_vals.empty() ? 1.0f : dot_vals[i];
                            float db = dot_vals.empty() ? 1.0f : dot_vals[j];
                            // Hemisphere visible t-range
                            float th0 = 0.0f, th1 = 1.0f;
                            if (da < 0.0f && db < 0.0f) continue;
                            if (da < 0.0f) { float tc = -da / (db - da); th0 = tc; }
                            else if (db < 0.0f) { float tc = -da / (db - da); th1 = tc; }
                            if (th0 >= th1) continue;
                            // Inner-circle clipping
                            ImVec2 d0 = vsub(sa, center);
                            ImVec2 e = vsub(sb, sa);
                            float qA = e.x*e.x + e.y*e.y;
                            float qB = 2.0f*(d0.x*e.x + d0.y*e.y);
                            float qC = d0.x*d0.x + d0.y*d0.y - inner_r2;
                            auto draw_range = [&](float ts, float te) {
                                if (ts >= te) return;
                                ImVec2 pa(sa.x + ts*(sb.x-sa.x), sa.y + ts*(sb.y-sa.y));
                                ImVec2 pb(sa.x + te*(sb.x-sa.x), sa.y + te*(sb.y-sa.y));
                                draw->AddLine(pa, pb, color, thick);
                            };
                            float disc = qB*qB - 4*qA*qC;
                            if (qA < 1e-10f || disc < 0.0f) {
                                if (qC >= 0.0f) draw_range(th0, th1);
                                continue;
                            }
                            float sq = std::sqrt(disc);
                            float t1 = (-qB - sq) / (2*qA);
                            float t2 = (-qB + sq) / (2*qA);
                            draw_range(th0, std::min(th1, t1));
                            draw_range(std::max(th0, t2), th1);
                        }
                    };

                    ImVec2 mouse = ImGui::GetIO().MousePos;
                    ImVec2 vx = has_x ? vsub(x_end, center) : ImVec2(0, 0);
                    ImVec2 vy = has_y ? vsub(y_end, center) : ImVec2(0, 0);
                    ImVec2 vz = has_z ? vsub(z_end, center) : ImVec2(0, 0);
                    ImVec2 ux = vnorm(vx);
                    ImVec2 uy = vnorm(vy);
                    ImVec2 uz = vnorm(vz);
                    float len_x = has_x ? vlen(vx) : 0.0f;
                    float len_y = has_y ? vlen(vy) : 0.0f;
                    float len_z = has_z ? vlen(vz) : 0.0f;
                    float axis_ref_len = std::max(1.0f, std::max(len_x, std::max(len_y, len_z)));
                    ImVec2 ex = x_end;
                    ImVec2 ey = y_end;
                    ImVec2 ez = z_end;
                    auto fade_from_ratio = [](float ratio) -> float {
                        // 20% -> 100% visible, 10% -> 0% visible, below 10% stays hidden.
                        const float visible_start = 0.20f;
                        const float hidden_end = 0.10f;
                        if (ratio <= hidden_end) return 0.0f;
                        if (ratio >= visible_start) return 1.0f;
                        return (ratio - hidden_end) / (visible_start - hidden_end);
                    };
                    auto axis_visibility = [&](float len) -> float {
                        float r = (axis_ref_len > 1e-5f) ? (len / axis_ref_len) : 0.0f;
                        return fade_from_ratio(r);
                    };
                    auto alpha_color = [](ImU32 color, float alpha_scale) -> ImU32 {
                        ImVec4 c = ImGui::ColorConvertU32ToFloat4(color);
                        c.w *= alpha_scale;
                        return ImGui::GetColorU32(c);
                    };
                    float vis_x = axis_visibility(len_x);
                    float vis_y = axis_visibility(len_y);
                    float vis_z = axis_visibility(len_z);
                    float vis_xy = std::min(vis_x, vis_y);
                    float vis_xz = std::min(vis_x, vis_z);
                    float vis_yz = std::min(vis_y, vis_z);

                    int mode = *transform_tool_mode;
                    int hover_handle = 0;
                    float best_dist = 10.0f;
                    const float pick_visibility_threshold = 0.2f;
                    float plane_in = 0.22f;
                    float plane_out = 0.38f;
                    // Inner arcball/clip sphere: just inside the colored ring radius (~0.9).
                    float rotate_inner_clip_r = axis_ref_len * 0.86f;
                    int active = *gizmo_active_axis;

                    // Freeze hover feedback once a gizmo handle is active.
                    if (active == 0) {
                        // Axis handles.
                        if (has_x && vis_x >= pick_visibility_threshold) {
                            float d = distance_to_segment(mouse, center, ex);
                            if (d < best_dist) { best_dist = d; hover_handle = (mode == 1) ? 1 : (mode == 2) ? 11 : 21; }
                        }
                        if (has_y && vis_y >= pick_visibility_threshold) {
                            float d = distance_to_segment(mouse, center, ey);
                            if (d < best_dist) { best_dist = d; hover_handle = (mode == 1) ? 2 : (mode == 2) ? 12 : 22; }
                        }
                        if (has_z && vis_z >= pick_visibility_threshold) {
                            float d = distance_to_segment(mouse, center, ez);
                            if (d < best_dist) { best_dist = d; hover_handle = (mode == 1) ? 3 : (mode == 2) ? 13 : 23; }
                        }

                        // Plane handles for translate/scale.
                        if (mode == 1 || mode == 3) {
                            if (has_x && has_y && vis_xy >= pick_visibility_threshold) {
                                ImVec2 a = vadd(center, vadd(vmul(vx, plane_in), vmul(vy, plane_in)));
                                ImVec2 b = vadd(center, vadd(vmul(vx, plane_out), vmul(vy, plane_in)));
                                ImVec2 c = vadd(center, vadd(vmul(vx, plane_out), vmul(vy, plane_out)));
                                ImVec2 d = vadd(center, vadd(vmul(vx, plane_in), vmul(vy, plane_out)));
                                if (point_in_quad(mouse, a, b, c, d)) hover_handle = (mode == 1) ? 4 : 24;
                            }
                            if (has_x && has_z && vis_xz >= pick_visibility_threshold) {
                                ImVec2 a = vadd(center, vadd(vmul(vx, plane_in), vmul(vz, plane_in)));
                                ImVec2 b = vadd(center, vadd(vmul(vx, plane_out), vmul(vz, plane_in)));
                                ImVec2 c = vadd(center, vadd(vmul(vx, plane_out), vmul(vz, plane_out)));
                                ImVec2 d = vadd(center, vadd(vmul(vx, plane_in), vmul(vz, plane_out)));
                                if (point_in_quad(mouse, a, b, c, d)) hover_handle = (mode == 1) ? 5 : 25;
                            }
                            if (has_y && has_z && vis_yz >= pick_visibility_threshold) {
                                ImVec2 a = vadd(center, vadd(vmul(vy, plane_in), vmul(vz, plane_in)));
                                ImVec2 b = vadd(center, vadd(vmul(vy, plane_out), vmul(vz, plane_in)));
                                ImVec2 c = vadd(center, vadd(vmul(vy, plane_out), vmul(vz, plane_out)));
                                ImVec2 d = vadd(center, vadd(vmul(vy, plane_in), vmul(vz, plane_out)));
                                if (point_in_quad(mouse, a, b, c, d)) hover_handle = (mode == 1) ? 6 : 26;
                            }
                        }

                        if (mode == 2) {
                            float ring_pick_thresh = 8.0f;
                            std::vector<ImVec2> ring_x = ring_points(vy, vz, 0.9f);
                            std::vector<ImVec2> ring_y = ring_points(vx, vz, 0.9f);
                            std::vector<ImVec2> ring_z = ring_points(vx, vy, 0.9f);
                            std::vector<float> front_x = ring_front_dot(0, (int)ring_x.size());
                            std::vector<float> front_y = ring_front_dot(1, (int)ring_y.size());
                            std::vector<float> front_z = ring_front_dot(2, (int)ring_z.size());
                            float dx = ring_distance_clipped(mouse, ring_x, front_x, rotate_inner_clip_r);
                            float dy = ring_distance_clipped(mouse, ring_y, front_y, rotate_inner_clip_r);
                            float dz = ring_distance_clipped(mouse, ring_z, front_z, rotate_inner_clip_r);
                            if (vis_x >= pick_visibility_threshold && dx < best_dist && dx < ring_pick_thresh) { best_dist = dx; hover_handle = 11; }
                            if (vis_y >= pick_visibility_threshold && dy < best_dist && dy < ring_pick_thresh) { best_dist = dy; hover_handle = 12; }
                            if (vis_z >= pick_visibility_threshold && dz < best_dist && dz < ring_pick_thresh) { best_dist = dz; hover_handle = 13; }
                            float white_r = axis_ref_len * 1.05f;
                            float d = std::abs(vlen(vsub(mouse, center)) - white_r);
                            if (d < best_dist && d < 9.0f) { hover_handle = 14; }
                            float d_inner = vlen(vsub(mouse, center));
                            if (d_inner <= rotate_inner_clip_r && hover_handle == 0) {
                                hover_handle = 15;
                            }
                        }
                        if (mode == 3) {
                            float uniform_r = axis_ref_len * 1.25f;
                            float d = std::abs(vlen(vsub(mouse, center)) - uniform_r);
                            if (d < best_dist && d < 9.0f) { hover_handle = 27; }
                        }
                    }

                    if (*gizmo_active_axis == 0 && hover_handle != 0 && ImGui::IsMouseClicked(0)) {
                        *gizmo_active_axis = hover_handle;
                    }

                    auto active_or_hover = [&](int id) -> bool { return active == id || hover_handle == id; };
                    float draw_vis_x = (active == 1 || active == 11 || active == 21) ? 1.0f : vis_x;
                    float draw_vis_y = (active == 2 || active == 12 || active == 22) ? 1.0f : vis_y;
                    float draw_vis_z = (active == 3 || active == 13 || active == 23) ? 1.0f : vis_z;
                    ImU32 c_x = alpha_color(
                        active_or_hover((mode == 2) ? 11 : (mode == 3) ? 21 : 1) ? IM_COL32(255, 220, 220, 255) : IM_COL32(230, 80, 80, 255),
                        draw_vis_x
                    );
                    ImU32 c_y = alpha_color(
                        active_or_hover((mode == 2) ? 12 : (mode == 3) ? 22 : 2) ? IM_COL32(220, 255, 220, 255) : IM_COL32(80, 230, 80, 255),
                        draw_vis_y
                    );
                    ImU32 c_z = alpha_color(
                        active_or_hover((mode == 2) ? 13 : (mode == 3) ? 23 : 3) ? IM_COL32(220, 220, 255, 255) : IM_COL32(80, 140, 255, 255),
                        draw_vis_z
                    );
                    float thickness = (*gizmo_active_axis != 0) ? 4.0f : 3.0f;

                    draw->AddCircleFilled(center, 4.0f, IM_COL32(255, 255, 255, 220));
                    if (mode == 1) {
                        if (has_x) draw_arrow(center, ex, c_x, thickness);
                        if (has_y) draw_arrow(center, ey, c_y, thickness);
                        if (has_z) draw_arrow(center, ez, c_z, thickness);
                    } else if (mode == 3) {
                        if (has_x) draw_square_head(center, ex, c_x, thickness);
                        if (has_y) draw_square_head(center, ey, c_y, thickness);
                        if (has_z) draw_square_head(center, ez, c_z, thickness);
                    } else if (mode != 2) {
                        if (has_x) draw->AddLine(center, ex, c_x, thickness);
                        if (has_y) draw->AddLine(center, ey, c_y, thickness);
                        if (has_z) draw->AddLine(center, ez, c_z, thickness);
                    }

                    if (mode == 1 || mode == 3) {
                        if (has_x && has_y) {
                            ImVec2 a = vadd(center, vadd(vmul(vx, plane_in), vmul(vy, plane_in)));
                            ImVec2 b = vadd(center, vadd(vmul(vx, plane_out), vmul(vy, plane_in)));
                            ImVec2 c = vadd(center, vadd(vmul(vx, plane_out), vmul(vy, plane_out)));
                            ImVec2 d = vadd(center, vadd(vmul(vx, plane_in), vmul(vy, plane_out)));
                            float pv = (active == ((mode == 1) ? 4 : 24)) ? 1.0f : vis_xy;
                            ImU32 cc = alpha_color(
                                active_or_hover((mode == 1) ? 4 : 24) ? IM_COL32(255, 230, 110, 165) : IM_COL32(255, 230, 110, 90),
                                pv
                            );
                            draw->AddQuadFilled(a, b, c, d, cc);
                        }
                        if (has_x && has_z) {
                            ImVec2 a = vadd(center, vadd(vmul(vx, plane_in), vmul(vz, plane_in)));
                            ImVec2 b = vadd(center, vadd(vmul(vx, plane_out), vmul(vz, plane_in)));
                            ImVec2 c = vadd(center, vadd(vmul(vx, plane_out), vmul(vz, plane_out)));
                            ImVec2 d = vadd(center, vadd(vmul(vx, plane_in), vmul(vz, plane_out)));
                            float pv = (active == ((mode == 1) ? 5 : 25)) ? 1.0f : vis_xz;
                            ImU32 cc = alpha_color(
                                active_or_hover((mode == 1) ? 5 : 25) ? IM_COL32(255, 230, 110, 165) : IM_COL32(255, 230, 110, 90),
                                pv
                            );
                            draw->AddQuadFilled(a, b, c, d, cc);
                        }
                        if (has_y && has_z) {
                            ImVec2 a = vadd(center, vadd(vmul(vy, plane_in), vmul(vz, plane_in)));
                            ImVec2 b = vadd(center, vadd(vmul(vy, plane_out), vmul(vz, plane_in)));
                            ImVec2 c = vadd(center, vadd(vmul(vy, plane_out), vmul(vz, plane_out)));
                            ImVec2 d = vadd(center, vadd(vmul(vy, plane_in), vmul(vz, plane_out)));
                            float pv = (active == ((mode == 1) ? 6 : 26)) ? 1.0f : vis_yz;
                            ImU32 cc = alpha_color(
                                active_or_hover((mode == 1) ? 6 : 26) ? IM_COL32(255, 230, 110, 165) : IM_COL32(255, 230, 110, 90),
                                pv
                            );
                            draw->AddQuadFilled(a, b, c, d, cc);
                        }
                    }

                    if (mode == 2) {
                        float inner_alpha = active_or_hover(15) ? 0.20f : 0.12f;
                        ImU32 inner_color = alpha_color(IM_COL32(255, 255, 255, 255), inner_alpha);
                        draw->AddCircleFilled(center, rotate_inner_clip_r, inner_color, 48);
                        std::vector<ImVec2> ring_x = ring_points(vy, vz, 0.9f);
                        std::vector<ImVec2> ring_y = ring_points(vx, vz, 0.9f);
                        std::vector<ImVec2> ring_z = ring_points(vx, vy, 0.9f);
                        std::vector<float> front_x = ring_front_dot(0, (int)ring_x.size());
                        std::vector<float> front_y = ring_front_dot(1, (int)ring_y.size());
                        std::vector<float> front_z = ring_front_dot(2, (int)ring_z.size());
                        draw_ring_clipped(
                            ring_x,
                            front_x,
                            rotate_inner_clip_r,
                            alpha_color(active_or_hover(11) ? IM_COL32(255, 220, 220, 255) : IM_COL32(230, 80, 80, 220), draw_vis_x),
                            thickness
                        );
                        draw_ring_clipped(
                            ring_y,
                            front_y,
                            rotate_inner_clip_r,
                            alpha_color(active_or_hover(12) ? IM_COL32(220, 255, 220, 255) : IM_COL32(80, 230, 80, 220), draw_vis_y),
                            thickness
                        );
                        draw_ring_clipped(
                            ring_z,
                            front_z,
                            rotate_inner_clip_r,
                            alpha_color(active_or_hover(13) ? IM_COL32(220, 220, 255, 255) : IM_COL32(80, 140, 255, 220), draw_vis_z),
                            thickness
                        );
                        float white_r = axis_ref_len * 1.05f;
                        ImU32 wc = active_or_hover(14) ? IM_COL32(255, 255, 255, 255) : IM_COL32(240, 240, 240, 200);
                        draw->AddCircle(center, white_r, wc, 64, 2.5f);
                    }

                    if (mode == 3) {
                        float uniform_r = axis_ref_len * 1.25f;
                        ImU32 wc = active_or_hover(27) ? IM_COL32(255, 255, 255, 255) : IM_COL32(240, 240, 240, 200);
                        draw->AddCircle(center, uniform_r, wc, 64, 2.5f);
                    }
                }
            } else {
                *gizmo_active_axis = 0;
            }
        }

        ImGui::End();
    });
}

// ============================================================================
// GPU Pick Pass - Texture, RenderTarget, Readback, Material Swap
// ============================================================================

Texture* filament_texture_create_2d(
    Engine* engine,
    uint32_t width,
    uint32_t height,
    uint8_t internal_format,
    uint32_t usage_flags
) {
    if (!engine || width == 0 || height == 0) {
        return nullptr;
    }
    return Texture::Builder()
        .width(width)
        .height(height)
        .levels(1)
        .format(static_cast<Texture::InternalFormat>(internal_format))
        .usage(static_cast<Texture::Usage>(usage_flags))
        .build(*engine);
}

RenderTarget* filament_render_target_create(
    Engine* engine,
    Texture* color,
    Texture* depth
) {
    if (!engine || !color) {
        return nullptr;
    }
    auto builder = RenderTarget::Builder()
        .texture(RenderTarget::AttachmentPoint::COLOR, color);
    if (depth) {
        builder.texture(RenderTarget::AttachmentPoint::DEPTH, depth);
    }
    return builder.build(*engine);
}

void filament_engine_destroy_render_target(Engine* engine, RenderTarget* target) {
    if (engine && target) {
        engine->destroy(target);
    }
}

void filament_view_set_render_target(View* view, RenderTarget* target) {
    if (!view) {
        return;
    }
    // pass nullptr to clear the render target (render to swap chain)
    view->setRenderTarget(target);
}

// Synchronous 1x1 pixel readback from a render target.
// Must be called AFTER endFrame() and BEFORE the next beginFrame().
// Caller must call engine->flushAndWait() afterwards to ensure completion.
bool filament_renderer_read_pixels(
    Renderer* renderer,
    RenderTarget* render_target,
    uint32_t x,
    uint32_t y,
    uint32_t width,
    uint32_t height,
    uint8_t* out_buffer,
    uint32_t buffer_size
) {
    if (!renderer || !render_target || !out_buffer || buffer_size == 0) {
        return false;
    }
    const uint32_t pixel_count = width * height;
    const uint32_t required = pixel_count * 4; // RGBA8
    if (buffer_size < required) {
        return false;
    }
    std::atomic<bool> done{false};
    auto pbd = backend::PixelBufferDescriptor(
        out_buffer,
        required,
        backend::PixelDataFormat::RGBA,
        backend::PixelDataType::UBYTE,
        [](void* /*buffer*/, size_t /*size*/, void* user) {
            auto* flag = static_cast<std::atomic<bool>*>(user);
            flag->store(true, std::memory_order_release);
        },
        &done
    );
    renderer->readPixels(render_target, x, y, width, height, std::move(pbd));
    return true; // Caller must flushAndWait() to complete
}

// ============================================================================
// RenderableManager - material swap for pick pass
// ============================================================================

int32_t filament_renderable_get_primitive_count(
    Engine* engine,
    int32_t entity_id
) {
    if (!engine) return 0;
    auto& rm = engine->getRenderableManager();
    Entity entity = Entity::import(entity_id);
    auto instance = rm.getInstance(entity);
    if (!instance) return 0;
    return static_cast<int32_t>(rm.getPrimitiveCount(instance));
}

MaterialInstance* filament_renderable_get_material_at(
    Engine* engine,
    int32_t entity_id,
    int32_t primitive_index
) {
    if (!engine) return nullptr;
    auto& rm = engine->getRenderableManager();
    Entity entity = Entity::import(entity_id);
    auto instance = rm.getInstance(entity);
    if (!instance) return nullptr;
    return rm.getMaterialInstanceAt(instance, static_cast<size_t>(primitive_index));
}

void filament_renderable_set_material_at(
    Engine* engine,
    int32_t entity_id,
    int32_t primitive_index,
    MaterialInstance* mi
) {
    if (!engine || !mi) return;
    auto& rm = engine->getRenderableManager();
    Entity entity = Entity::import(entity_id);
    auto instance = rm.getInstance(entity);
    if (!instance) return;
    rm.setMaterialInstanceAt(instance, static_cast<size_t>(primitive_index), mi);
}

// Get all renderable entities from a gltfio FilamentAsset
int32_t filament_gltfio_asset_get_entities(
    FilamentAsset* asset,
    int32_t* out_entities,
    int32_t max_count
) {
    if (!asset || !out_entities || max_count <= 0) return 0;
    size_t count = asset->getRenderableEntityCount();
    if (count > static_cast<size_t>(max_count)) {
        count = static_cast<size_t>(max_count);
    }
    const Entity* entities = asset->getRenderableEntities();
    for (size_t i = 0; i < count; i++) {
        out_entities[i] = Entity::smuggle(entities[i]);
    }
    return static_cast<int32_t>(count);
}

int32_t filament_gltfio_asset_get_renderable_entity_count(
    FilamentAsset* asset
) {
    if (!asset) return 0;
    return static_cast<int32_t>(asset->getRenderableEntityCount());
}

} // extern "C"
