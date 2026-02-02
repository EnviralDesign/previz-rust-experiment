//! Safe wrappers around Filament FFI
#![allow(dead_code)]
//!
//! This module provides idiomatic Rust wrappers that handle
//! the unsafe FFI calls and resource management.

use crate::ffi;
use std::ffi::{c_char, c_void, CString};
use std::ptr::NonNull;

/// Backend rendering API
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backend {
    Default = 0,
    OpenGL = 1,
    Vulkan = 2,
    Metal = 3,
    WebGPU = 4,
    Noop = 5,
}

/// Element types for vertex attributes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElementType {
    Byte = 0,
    Byte2 = 1,
    Byte3 = 2,
    Byte4 = 3,
    UByte = 4,
    UByte2 = 5,
    UByte3 = 6,
    UByte4 = 7,
    Short = 8,
    Short2 = 9,
    Short3 = 10,
    Short4 = 11,
    UShort = 12,
    UShort2 = 13,
    UShort3 = 14,
    UShort4 = 15,
    Int = 16,
    UInt = 17,
    Float = 18,
    Float2 = 19,
    Float3 = 20,
    Float4 = 21,
    Half = 22,
    Half2 = 23,
    Half3 = 24,
    Half4 = 25,
}

/// Primitive types for geometry
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PrimitiveType {
    Points = 0,
    Lines = 1,
    LineStrip = 3,
    Triangles = 4,
    TriangleStrip = 5,
}

/// Vertex attribute enum
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VertexAttribute {
    Position = 0,
    Tangent = 1,
    Color = 2,
    UV0 = 3,
    UV1 = 4,
    BoneIndices = 5,
    BoneWeights = 6,
    Custom0 = 8,
    Custom1 = 9,
    Custom2 = 10,
    Custom3 = 11,
    Custom4 = 12,
    Custom5 = 13,
    Custom6 = 14,
    Custom7 = 15,
}

/// Index buffer type enum
/// Values must match backend::ElementType
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IndexType {
    UShort = 12, // ElementType::USHORT
    UInt = 17,   // ElementType::UINT
}

/// Filament Engine - the main entry point for all Filament operations
pub struct Engine {
    ptr: NonNull<c_void>,
}

impl Engine {
    /// Create a new Filament engine with the specified backend
    pub fn create(backend: Backend) -> Option<Self> {
        unsafe {
            let ptr = ffi::filament_engine_create(backend as u8);
            NonNull::new(ptr as *mut c_void).map(|ptr| Engine { ptr })
        }
    }

    /// Create a swap chain for a native window
    pub fn create_swap_chain(&mut self, native_window: *mut c_void) -> Option<SwapChain> {
        unsafe {
            let ptr = ffi::filament_engine_create_swap_chain(
                self.ptr.as_ptr() as *mut _,
                native_window,
                0, // flags
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| SwapChain { 
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Create a renderer
    pub fn create_renderer(&mut self) -> Option<Renderer> {
        unsafe {
            let ptr = ffi::filament_engine_create_renderer(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| Renderer { 
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Create a scene
    pub fn create_scene(&mut self) -> Option<Scene> {
        unsafe {
            let ptr = ffi::filament_engine_create_scene(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| Scene { 
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Create a view
    pub fn create_view(&mut self) -> Option<View> {
        unsafe {
            let ptr = ffi::filament_engine_create_view(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| View { 
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Create a camera
    pub fn create_camera(&mut self, entity: Entity) -> Option<Camera> {
        unsafe {
            let ptr = ffi::filament_engine_create_camera(
                self.ptr.as_ptr() as *mut _,
                entity.id,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| Camera { 
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Get the entity manager
    pub fn entity_manager(&mut self) -> EntityManager {
        unsafe {
            let ptr = ffi::filament_engine_get_entity_manager(self.ptr.as_ptr() as *mut _);
            EntityManager { 
                ptr: NonNull::new(ptr as *mut c_void).expect("EntityManager is null"),
            }
        }
    }

    /// Create a directional light and return its entity
    pub fn create_directional_light(
        &mut self,
        entity_manager: &mut EntityManager,
        color: [f32; 3],
        intensity: f32,
        direction: [f32; 3],
    ) -> Entity {
        unsafe {
            let id = ffi::filament_light_create_directional(
                self.ptr.as_ptr() as *mut _,
                entity_manager.ptr.as_ptr() as *mut _,
                color[0],
                color[1],
                color[2],
                intensity,
                direction[0],
                direction[1],
                direction[2],
            );
            Entity { id }
        }
    }

    /// Create a material from package bytes
    pub fn create_material(&mut self, package: &[u8]) -> Option<Material> {
        unsafe {
            let builder = ffi::filament_material_builder_create();
            ffi::filament_material_builder_package(
                builder,
                package.as_ptr() as *const c_void,
                package.len(),
            );
            let material = ffi::filament_material_builder_build(builder, self.ptr.as_ptr() as *mut _);
            ffi::filament_material_builder_destroy(builder);
            NonNull::new(material as *mut c_void).map(|ptr| Material { ptr })
        }
    }

    /// Create a vertex buffer builder
    pub fn vertex_buffer_builder(&mut self) -> VertexBufferBuilder {
        unsafe {
            let ptr = ffi::filament_vertex_buffer_builder_create();
            VertexBufferBuilder {
                ptr,
                engine: self.ptr,
            }
        }
    }

    /// Create an index buffer builder
    pub fn index_buffer_builder(&mut self) -> IndexBufferBuilder {
        unsafe {
            let ptr = ffi::filament_index_buffer_builder_create();
            IndexBufferBuilder {
                ptr,
                engine: self.ptr,
            }
        }
    }

    /// Create a renderable builder
    pub fn renderable_builder(&mut self, primitive_count: usize) -> RenderableBuilder {
        unsafe {
            let ptr = ffi::filament_renderable_builder_create(primitive_count);
            RenderableBuilder {
                ptr,
                engine: self.ptr,
            }
        }
    }

    /// Flush and wait for all pending commands
    pub fn flush_and_wait(&mut self) {
        unsafe {
            ffi::filament_engine_flush_and_wait(self.ptr.as_ptr() as *mut _);
        }
    }

    /// Get raw pointer (for advanced use)
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        unsafe {
            let mut ptr = self.ptr.as_ptr() as *mut _;
            ffi::filament_engine_destroy(&mut ptr);
        }
    }
}

/// Swap chain for presenting to a window
pub struct SwapChain {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl SwapChain {
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

impl Drop for SwapChain {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_swap_chain(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Renderer
pub struct Renderer {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl Renderer {
    /// Begin a new frame
    pub fn begin_frame(&mut self, swap_chain: &mut SwapChain) -> bool {
        unsafe {
            ffi::filament_renderer_begin_frame(
                self.ptr.as_ptr() as *mut _,
                swap_chain.ptr.as_ptr() as *mut _,
            )
        }
    }

    /// End the current frame
    pub fn end_frame(&mut self) {
        unsafe {
            ffi::filament_renderer_end_frame(self.ptr.as_ptr() as *mut _);
        }
    }

    /// Render a view
    pub fn render(&mut self, view: &View) {
        unsafe {
            ffi::filament_renderer_render(
                self.ptr.as_ptr() as *mut _,
                view.ptr.as_ptr() as *mut _,
            );
        }
    }

    /// Set clear color and options
    pub fn set_clear_options(&mut self, r: f32, g: f32, b: f32, a: f32, clear: bool, discard: bool) {
        unsafe {
            ffi::filament_renderer_set_clear_options(
                self.ptr.as_ptr() as *mut _,
                r, g, b, a,
                clear,
                discard,
            );
        }
    }
}

impl Drop for Renderer {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_renderer(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Scene - container for entities
pub struct Scene {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl Scene {
    /// Add an entity to the scene
    pub fn add_entity(&mut self, entity: Entity) {
        unsafe {
            ffi::filament_scene_add_entity(self.ptr.as_ptr() as *mut _, entity.id);
        }
    }

    /// Remove an entity from the scene
    pub fn remove_entity(&mut self, entity: Entity) {
        unsafe {
            ffi::filament_scene_remove_entity(self.ptr.as_ptr() as *mut _, entity.id);
        }
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_scene(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// View - defines what to render and how
pub struct View {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl View {
    /// Set the scene to render
    pub fn set_scene(&mut self, scene: &mut Scene) {
        unsafe {
            ffi::filament_view_set_scene(
                self.ptr.as_ptr() as *mut _,
                scene.ptr.as_ptr() as *mut _,
            );
        }
    }

    /// Set the camera to use
    pub fn set_camera(&mut self, camera: &mut Camera) {
        unsafe {
            ffi::filament_view_set_camera(
                self.ptr.as_ptr() as *mut _,
                camera.ptr.as_ptr() as *mut _,
            );
        }
    }

    /// Set the viewport
    pub fn set_viewport(&mut self, left: i32, bottom: i32, width: u32, height: u32) {
        unsafe {
            ffi::filament_view_set_viewport(
                self.ptr.as_ptr() as *mut _,
                left,
                bottom,
                width,
                height,
            );
        }
    }

    /// Enable or disable post-processing
    pub fn set_post_processing_enabled(&mut self, enabled: bool) {
        unsafe {
            ffi::filament_view_set_post_processing_enabled(
                self.ptr.as_ptr() as *mut _,
                enabled,
            );
        }
    }
}

impl Drop for View {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_view(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Camera
pub struct Camera {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl Camera {
    /// Set orthographic projection
    pub fn set_projection_ortho(&mut self, left: f64, right: f64, bottom: f64, top: f64, near: f64, far: f64) {
        unsafe {
            ffi::filament_camera_set_projection_ortho(
                self.ptr.as_ptr() as *mut _,
                left, right, bottom, top, near, far,
            );
        }
    }

    /// Set perspective projection
    pub fn set_projection_perspective(&mut self, fov_degrees: f64, aspect: f64, near: f64, far: f64) {
        unsafe {
            ffi::filament_camera_set_projection_perspective(
                self.ptr.as_ptr() as *mut _,
                fov_degrees, aspect, near, far,
            );
        }
    }

    /// Look at a target position
    pub fn look_at(&mut self, eye: [f32; 3], center: [f32; 3], up: [f32; 3]) {
        unsafe {
            ffi::filament_camera_look_at(
                self.ptr.as_ptr() as *mut _,
                eye[0], eye[1], eye[2],
                center[0], center[1], center[2],
                up[0], up[1], up[2],
            );
        }
    }
}

impl Drop for Camera {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_camera(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Entity identifier
/// Entity identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Entity {
    pub id: i32,
}

/// Entity manager
pub struct EntityManager {
    ptr: NonNull<c_void>,
}

impl EntityManager {
    /// Create a new entity
    pub fn create(&mut self) -> Entity {
        unsafe {
            let id = ffi::filament_entity_manager_create(self.ptr.as_ptr() as *mut _);
            Entity { id }
        }
    }

    /// Destroy an entity
    pub fn destroy(&mut self, entity: Entity) {
        unsafe {
            ffi::filament_entity_manager_destroy(self.ptr.as_ptr() as *mut _, entity.id);
        }
    }
}

/// Material
pub struct Material {
    ptr: NonNull<c_void>,
}

impl Material {
    /// Get the default material instance
    pub fn default_instance(&mut self) -> Option<MaterialInstance> {
        unsafe {
            let ptr = ffi::filament_material_get_default_instance(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| MaterialInstance { 
                ptr,
                owned: false, // Default instance is not owned
            })
        }
    }

    /// Create a new material instance
    pub fn create_instance(&mut self) -> Option<MaterialInstance> {
        unsafe {
            let ptr = ffi::filament_material_create_instance(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| MaterialInstance { 
                ptr,
                owned: true,
            })
        }
    }
}

/// Material instance
pub struct MaterialInstance {
    ptr: NonNull<c_void>,
    owned: bool,
}

impl MaterialInstance {
    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

// Note: MaterialInstance drop is complex because default instances aren't owned
// For now we'll leak owned instances (proper cleanup requires engine reference)

/// gltfio material provider (jit shader provider)
pub struct GltfMaterialProvider {
    ptr: NonNull<c_void>,
}

impl GltfMaterialProvider {
    pub fn create_jit(engine: &mut Engine, optimize: bool) -> Option<Self> {
        unsafe {
            let ptr = ffi::filament_gltfio_create_jit_shader_provider(
                engine.ptr.as_ptr() as *mut _,
                optimize,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| GltfMaterialProvider { ptr })
        }
    }
}

impl Drop for GltfMaterialProvider {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_gltfio_material_provider_destroy_materials(self.ptr.as_ptr() as *mut _);
            ffi::filament_gltfio_destroy_material_provider(self.ptr.as_ptr() as *mut _);
        }
    }
}

/// gltfio texture provider (stb image)
pub struct GltfTextureProvider {
    ptr: NonNull<c_void>,
}

impl GltfTextureProvider {
    pub fn create_stb(engine: &mut Engine) -> Option<Self> {
        unsafe {
            let ptr = ffi::filament_gltfio_create_stb_texture_provider(
                engine.ptr.as_ptr() as *mut _,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| GltfTextureProvider { ptr })
        }
    }
}

impl Drop for GltfTextureProvider {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_gltfio_destroy_texture_provider(self.ptr.as_ptr() as *mut _);
        }
    }
}

/// gltfio asset loader
pub struct GltfAssetLoader {
    ptr: NonNull<c_void>,
}

impl GltfAssetLoader {
    pub fn create(
        engine: &mut Engine,
        material_provider: &mut GltfMaterialProvider,
        entity_manager: &mut EntityManager,
    ) -> Option<Self> {
        unsafe {
            let ptr = ffi::filament_gltfio_asset_loader_create(
                engine.ptr.as_ptr() as *mut _,
                material_provider.ptr.as_ptr() as *mut _,
                entity_manager.ptr.as_ptr() as *mut _,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| GltfAssetLoader { ptr })
        }
    }

    pub fn create_asset_from_json(&mut self, bytes: &[u8]) -> Option<GltfAsset> {
        unsafe {
            let ptr = ffi::filament_gltfio_asset_loader_create_asset_from_json(
                self.ptr.as_ptr() as *mut _,
                bytes.as_ptr(),
                bytes.len() as u32,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| GltfAsset {
                ptr,
                loader: self.ptr,
            })
        }
    }
}

impl Drop for GltfAssetLoader {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_gltfio_asset_loader_destroy(self.ptr.as_ptr() as *mut _);
        }
    }
}

/// gltfio resource loader
pub struct GltfResourceLoader {
    ptr: NonNull<c_void>,
}

impl GltfResourceLoader {
    pub fn create(
        engine: &mut Engine,
        gltf_path: Option<&str>,
        normalize_skinning_weights: bool,
    ) -> Option<Self> {
        let c_path = gltf_path.map(|path| CString::new(path).expect("Invalid gltf path"));
        let path_ptr = c_path
            .as_ref()
            .map(|path| path.as_ptr())
            .unwrap_or(std::ptr::null());
        unsafe {
            let ptr = ffi::filament_gltfio_resource_loader_create(
                engine.ptr.as_ptr() as *mut _,
                path_ptr,
                normalize_skinning_weights,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| GltfResourceLoader { ptr })
        }
    }

    pub fn add_texture_provider(&mut self, mime_type: &str, provider: &mut GltfTextureProvider) {
        let c_mime = CString::new(mime_type).expect("Invalid mime type");
        unsafe {
            ffi::filament_gltfio_resource_loader_add_texture_provider(
                self.ptr.as_ptr() as *mut _,
                c_mime.as_ptr() as *const c_char,
                provider.ptr.as_ptr() as *mut _,
            );
        }
    }

    pub fn load_resources(&mut self, asset: &mut GltfAsset) -> bool {
        unsafe {
            ffi::filament_gltfio_resource_loader_load_resources(
                self.ptr.as_ptr() as *mut _,
                asset.ptr.as_ptr() as *mut _,
            )
        }
    }
}

impl Drop for GltfResourceLoader {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_gltfio_resource_loader_destroy(self.ptr.as_ptr() as *mut _);
        }
    }
}

/// gltfio asset
pub struct GltfAsset {
    ptr: NonNull<c_void>,
    loader: NonNull<c_void>,
}

impl GltfAsset {
    pub fn add_entities_to_scene(&mut self, scene: &mut Scene) {
        unsafe {
            ffi::filament_gltfio_asset_add_entities_to_scene(
                self.ptr.as_ptr() as *mut _,
                scene.ptr.as_ptr() as *mut _,
            );
        }
    }

    pub fn release_source_data(&mut self) {
        unsafe {
            ffi::filament_gltfio_asset_release_source_data(self.ptr.as_ptr() as *mut _);
        }
    }

    pub fn bounding_box(&mut self) -> ([f32; 3], [f32; 3]) {
        let mut center = [0.0f32; 3];
        let mut extent = [0.0f32; 3];
        unsafe {
            ffi::filament_gltfio_asset_get_bounding_box(
                self.ptr.as_ptr() as *mut _,
                center.as_mut_ptr(),
                extent.as_mut_ptr(),
            );
        }
        (center, extent)
    }
}

impl Drop for GltfAsset {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_gltfio_asset_loader_destroy_asset(
                self.loader.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Vertex buffer builder
pub struct VertexBufferBuilder {
    ptr: *mut c_void,
    engine: NonNull<c_void>,
}

impl VertexBufferBuilder {
    pub fn vertex_count(self, count: u32) -> Self {
        unsafe {
            ffi::filament_vertex_buffer_builder_vertex_count(self.ptr, count);
        }
        self
    }

    pub fn buffer_count(self, count: u8) -> Self {
        unsafe {
            ffi::filament_vertex_buffer_builder_buffer_count(self.ptr, count);
        }
        self
    }

    pub fn attribute(
        self,
        attribute: VertexAttribute,
        buffer_index: u8,
        element_type: ElementType,
        byte_offset: u32,
        byte_stride: u8,
    ) -> Self {
        unsafe {
            ffi::filament_vertex_buffer_builder_attribute(
                self.ptr,
                attribute as u8,
                buffer_index,
                element_type as u8,
                byte_offset,
                byte_stride,
            );
        }
        self
    }

    pub fn normalized(self, attribute: VertexAttribute, normalized: bool) -> Self {
        unsafe {
            ffi::filament_vertex_buffer_builder_normalized(self.ptr, attribute as u8, normalized);
        }
        self
    }

    pub fn build(self) -> Option<VertexBuffer> {
        unsafe {
            let ptr = ffi::filament_vertex_buffer_builder_build(
                self.ptr,
                self.engine.as_ptr() as *mut _,
            );
            ffi::filament_vertex_buffer_builder_destroy(self.ptr);
            NonNull::new(ptr as *mut c_void).map(|ptr| VertexBuffer { 
                ptr,
                engine: self.engine,
            })
        }
    }
}

/// Vertex buffer
pub struct VertexBuffer {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl VertexBuffer {
    /// Set buffer data for a specific buffer slot
    pub fn set_buffer_at<T>(&mut self, buffer_index: u8, data: &[T], dest_offset: u32) {
        unsafe {
            ffi::filament_vertex_buffer_set_buffer_at(
                self.ptr.as_ptr() as *mut _,
                self.engine.as_ptr() as *mut _,
                buffer_index,
                data.as_ptr() as *const c_void,
                data.len() * std::mem::size_of::<T>(),
                dest_offset,
            );
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

/// Index buffer builder
pub struct IndexBufferBuilder {
    ptr: *mut c_void,
    engine: NonNull<c_void>,
}

impl IndexBufferBuilder {
    pub fn index_count(self, count: u32) -> Self {
        unsafe {
            ffi::filament_index_buffer_builder_index_count(self.ptr, count);
        }
        self
    }

    pub fn buffer_type(self, index_type: IndexType) -> Self {
        unsafe {
            ffi::filament_index_buffer_builder_buffer_type(self.ptr, index_type as u8);
        }
        self
    }

    pub fn build(self) -> Option<IndexBuffer> {
        unsafe {
            let ptr = ffi::filament_index_buffer_builder_build(
                self.ptr,
                self.engine.as_ptr() as *mut _,
            );
            ffi::filament_index_buffer_builder_destroy(self.ptr);
            NonNull::new(ptr as *mut c_void).map(|ptr| IndexBuffer { 
                ptr,
                engine: self.engine,
            })
        }
    }
}

/// Index buffer
pub struct IndexBuffer {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl IndexBuffer {
    /// Set buffer data
    pub fn set_buffer<T>(&mut self, data: &[T], dest_offset: u32) {
        unsafe {
            ffi::filament_index_buffer_set_buffer(
                self.ptr.as_ptr() as *mut _,
                self.engine.as_ptr() as *mut _,
                data.as_ptr() as *const c_void,
                data.len() * std::mem::size_of::<T>(),
                dest_offset,
            );
        }
    }

    pub fn as_ptr(&self) -> *mut c_void {
        self.ptr.as_ptr()
    }
}

/// Renderable builder
pub struct RenderableBuilder {
    ptr: *mut c_void,
    engine: NonNull<c_void>,
}

impl RenderableBuilder {
    pub fn bounding_box(self, center: [f32; 3], half_extent: [f32; 3]) -> Self {
        unsafe {
            ffi::filament_renderable_builder_bounding_box(
                self.ptr,
                center[0], center[1], center[2],
                half_extent[0], half_extent[1], half_extent[2],
            );
        }
        self
    }

    pub fn material(self, index: usize, material_instance: &mut MaterialInstance) -> Self {
        unsafe {
            ffi::filament_renderable_builder_material(
                self.ptr,
                index,
                material_instance.ptr.as_ptr() as *mut _,
            );
        }
        self
    }

    pub fn geometry(
        self,
        index: usize,
        primitive_type: PrimitiveType,
        vertex_buffer: &mut VertexBuffer,
        index_buffer: &mut IndexBuffer,
    ) -> Self {
        unsafe {
            ffi::filament_renderable_builder_geometry(
                self.ptr,
                index,
                primitive_type as u8,
                vertex_buffer.ptr.as_ptr() as *mut _,
                index_buffer.ptr.as_ptr() as *mut _,
            );
        }
        self
    }

    pub fn culling(self, enabled: bool) -> Self {
        unsafe {
            ffi::filament_renderable_builder_culling(self.ptr, enabled);
        }
        self
    }

    pub fn build(self, entity: Entity) {
        unsafe {
            ffi::filament_renderable_builder_build(
                self.ptr,
                self.engine.as_ptr() as *mut _,
                entity.id,
            );
            ffi::filament_renderable_builder_destroy(self.ptr);
        }
    }
}
