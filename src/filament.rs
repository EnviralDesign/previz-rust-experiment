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
            let ptr = ffi::filament_engine_create_camera(self.ptr.as_ptr() as *mut _, entity.id);
            NonNull::new(ptr as *mut c_void).map(|ptr| Camera {
                ptr,
                engine: self.ptr,
            })
        }
    }

    /// Get the entity manager
    pub fn entity_manager(&mut self) -> Option<EntityManager> {
        unsafe {
            let ptr = ffi::filament_engine_get_entity_manager(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| EntityManager { ptr })
        }
    }

    /// Get the transform manager
    pub fn transform_manager(&mut self) -> Option<TransformManager> {
        unsafe {
            let ptr = ffi::filament_engine_get_transform_manager(self.ptr.as_ptr() as *mut _);
            NonNull::new(ptr as *mut c_void).map(|ptr| TransformManager { ptr })
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

    /// Update a directional light's parameters
    pub fn set_directional_light(
        &mut self,
        entity: Entity,
        color: [f32; 3],
        intensity: f32,
        direction: [f32; 3],
    ) {
        unsafe {
            ffi::filament_light_set_directional(
                self.ptr.as_ptr() as *mut _,
                entity.id,
                color[0],
                color[1],
                color[2],
                intensity,
                direction[0],
                direction[1],
                direction[2],
            );
        }
    }

    /// Create an indirect light from a KTX environment map.
    pub fn create_indirect_light_from_ktx(
        &mut self,
        ktx_path: &str,
        intensity: f32,
    ) -> Option<(IndirectLight, Texture)> {
        let c_path = match CString::new(ktx_path) {
            Ok(path) => path,
            Err(_) => {
                log::warn!("Invalid KTX path (contains NUL byte).");
                return None;
            }
        };
        unsafe {
            let mut texture_ptr: *mut c_void = std::ptr::null_mut();
            let light_ptr = ffi::filament_create_indirect_light_from_ktx(
                self.ptr.as_ptr() as *mut _,
                c_path.as_ptr(),
                intensity,
                &mut texture_ptr as *mut *mut c_void,
            );
            let light = NonNull::new(light_ptr as *mut c_void).map(|ptr| IndirectLight {
                ptr,
                engine: self.ptr,
            })?;
            let texture = NonNull::new(texture_ptr).map(|ptr| Texture {
                ptr,
                engine: self.ptr,
                owned: true,
            })?;
            Some((light, texture))
        }
    }

    /// Create a skybox from a KTX cubemap.
    pub fn create_skybox_from_ktx(&mut self, ktx_path: &str) -> Option<(Skybox, Texture)> {
        let c_path = match CString::new(ktx_path) {
            Ok(path) => path,
            Err(_) => {
                log::warn!("Invalid KTX path (contains NUL byte).");
                return None;
            }
        };
        unsafe {
            let mut texture_ptr: *mut c_void = std::ptr::null_mut();
            let skybox_ptr = ffi::filament_create_skybox_from_ktx(
                self.ptr.as_ptr() as *mut _,
                c_path.as_ptr(),
                &mut texture_ptr as *mut *mut c_void,
            );
            let skybox = NonNull::new(skybox_ptr as *mut c_void).map(|ptr| Skybox {
                ptr,
                engine: self.ptr,
            })?;
            let texture = NonNull::new(texture_ptr).map(|ptr| Texture {
                ptr,
                engine: self.ptr,
                owned: true,
            })?;
            Some((skybox, texture))
        }
    }

    pub fn bind_material_texture_from_ktx(
        &mut self,
        material_instance: &mut MaterialInstance,
        param_name: &str,
        ktx_path: &str,
        wrap_repeat_u: bool,
        wrap_repeat_v: bool,
    ) -> Option<Texture> {
        let c_param = match CString::new(param_name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid texture parameter name (contains NUL byte).");
                return None;
            }
        };
        let c_path = match CString::new(ktx_path) {
            Ok(path) => path,
            Err(_) => {
                log::warn!("Invalid texture path (contains NUL byte).");
                return None;
            }
        };
        unsafe {
            let mut texture_ptr: *mut ffi::Texture = std::ptr::null_mut();
            let ok = ffi::filament_material_instance_set_texture_from_ktx(
                self.ptr.as_ptr() as *mut _,
                material_instance.ptr.as_ptr() as *mut _,
                c_param.as_ptr(),
                c_path.as_ptr(),
                wrap_repeat_u,
                wrap_repeat_v,
                &mut texture_ptr as *mut *mut ffi::Texture,
            );
            if !ok {
                return None;
            }
            NonNull::new(texture_ptr as *mut c_void).map(|ptr| Texture {
                ptr,
                engine: self.ptr,
                owned: false,
            })
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
            let material =
                ffi::filament_material_builder_build(builder, self.ptr.as_ptr() as *mut _);
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
            ffi::filament_renderer_render(self.ptr.as_ptr() as *mut _, view.ptr.as_ptr() as *mut _);
        }
    }

    /// Set clear color and options
    pub fn set_clear_options(
        &mut self,
        r: f32,
        g: f32,
        b: f32,
        a: f32,
        clear: bool,
        discard: bool,
    ) {
        unsafe {
            ffi::filament_renderer_set_clear_options(
                self.ptr.as_ptr() as *mut _,
                r,
                g,
                b,
                a,
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

    /// Set the indirect light for the scene.
    pub fn set_indirect_light(&mut self, light: Option<&IndirectLight>) {
        unsafe {
            let ptr = light
                .map(|value| value.ptr.as_ptr() as *mut _)
                .unwrap_or(std::ptr::null_mut());
            ffi::filament_scene_set_indirect_light(self.ptr.as_ptr() as *mut _, ptr);
        }
    }

    /// Set the skybox for the scene.
    pub fn set_skybox(&mut self, skybox: Option<&Skybox>) {
        unsafe {
            let ptr = skybox
                .map(|value| value.ptr.as_ptr() as *mut _)
                .unwrap_or(std::ptr::null_mut());
            ffi::filament_scene_set_skybox(self.ptr.as_ptr() as *mut _, ptr);
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

/// Texture
pub struct Texture {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
    owned: bool,
}

impl Drop for Texture {
    fn drop(&mut self) {
        if !self.owned {
            return;
        }
        unsafe {
            ffi::filament_engine_destroy_texture(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Indirect light
pub struct IndirectLight {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl IndirectLight {
    pub fn set_intensity(&mut self, intensity: f32) {
        unsafe {
            ffi::filament_indirect_light_set_intensity(self.ptr.as_ptr() as *mut _, intensity);
        }
    }
}

impl Drop for IndirectLight {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_indirect_light(
                self.engine.as_ptr() as *mut _,
                self.ptr.as_ptr() as *mut _,
            );
        }
    }
}

/// Skybox
pub struct Skybox {
    ptr: NonNull<c_void>,
    engine: NonNull<c_void>,
}

impl Drop for Skybox {
    fn drop(&mut self) {
        unsafe {
            ffi::filament_engine_destroy_skybox(
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
            ffi::filament_view_set_scene(self.ptr.as_ptr() as *mut _, scene.ptr.as_ptr() as *mut _);
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
            ffi::filament_view_set_post_processing_enabled(self.ptr.as_ptr() as *mut _, enabled);
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
    pub fn set_projection_ortho(
        &mut self,
        left: f64,
        right: f64,
        bottom: f64,
        top: f64,
        near: f64,
        far: f64,
    ) {
        unsafe {
            ffi::filament_camera_set_projection_ortho(
                self.ptr.as_ptr() as *mut _,
                left,
                right,
                bottom,
                top,
                near,
                far,
            );
        }
    }

    /// Set perspective projection
    pub fn set_projection_perspective(
        &mut self,
        fov_degrees: f64,
        aspect: f64,
        near: f64,
        far: f64,
    ) {
        unsafe {
            ffi::filament_camera_set_projection_perspective(
                self.ptr.as_ptr() as *mut _,
                fov_degrees,
                aspect,
                near,
                far,
            );
        }
    }

    /// Look at a target position
    pub fn look_at(&mut self, eye: [f32; 3], center: [f32; 3], up: [f32; 3]) {
        unsafe {
            ffi::filament_camera_look_at(
                self.ptr.as_ptr() as *mut _,
                eye[0],
                eye[1],
                eye[2],
                center[0],
                center[1],
                center[2],
                up[0],
                up[1],
                up[2],
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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

/// Transform manager
pub struct TransformManager {
    ptr: NonNull<c_void>,
}

impl TransformManager {
    pub fn set_transform(&mut self, entity: Entity, matrix4x4: &[f32; 16]) {
        unsafe {
            ffi::filament_transform_manager_set_transform(
                self.ptr.as_ptr() as *mut _,
                entity.id,
                matrix4x4.as_ptr(),
            );
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
            NonNull::new(ptr as *mut c_void).map(|ptr| MaterialInstance { ptr, owned: true })
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

    pub fn name(&self) -> String {
        unsafe {
            let ptr = ffi::filament_material_instance_get_name(self.ptr.as_ptr() as *mut _);
            if ptr.is_null() {
                return "Material".to_string();
            }
            let c_str = std::ffi::CStr::from_ptr(ptr);
            c_str.to_string_lossy().to_string()
        }
    }

    pub fn has_parameter(&self, name: &str) -> bool {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return false;
            }
        };
        unsafe {
            ffi::filament_material_instance_has_parameter(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
            )
        }
    }

    pub fn set_float(&mut self, name: &str, value: f32) {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return;
            }
        };
        unsafe {
            ffi::filament_material_instance_set_float(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                value,
            );
        }
    }

    pub fn set_float3(&mut self, name: &str, value: [f32; 3]) {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return;
            }
        };
        unsafe {
            ffi::filament_material_instance_set_float3(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                value[0],
                value[1],
                value[2],
            );
        }
    }

    pub fn set_float4(&mut self, name: &str, value: [f32; 4]) {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return;
            }
        };
        unsafe {
            ffi::filament_material_instance_set_float4(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                value[0],
                value[1],
                value[2],
                value[3],
            );
        }
    }

    pub fn get_float(&self, name: &str) -> Option<f32> {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return None;
            }
        };
        let mut value = 0.0f32;
        let ok = unsafe {
            ffi::filament_material_instance_get_float(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                &mut value as *mut f32,
            )
        };
        if ok {
            Some(value)
        } else {
            None
        }
    }

    pub fn get_float3(&self, name: &str) -> Option<[f32; 3]> {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return None;
            }
        };
        let mut value = [0.0f32; 3];
        let ok = unsafe {
            ffi::filament_material_instance_get_float3(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                value.as_mut_ptr(),
            )
        };
        if ok {
            Some(value)
        } else {
            None
        }
    }

    pub fn get_float4(&self, name: &str) -> Option<[f32; 4]> {
        let c_name = match CString::new(name) {
            Ok(name) => name,
            Err(_) => {
                log::warn!("Invalid parameter name (contains NUL byte).");
                return None;
            }
        };
        let mut value = [0.0f32; 4];
        let ok = unsafe {
            ffi::filament_material_instance_get_float4(
                self.ptr.as_ptr() as *mut _,
                c_name.as_ptr(),
                value.as_mut_ptr(),
            )
        };
        if ok {
            Some(value)
        } else {
            None
        }
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
            let ptr =
                ffi::filament_gltfio_create_stb_texture_provider(engine.ptr.as_ptr() as *mut _);
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
        let c_path = match gltf_path {
            Some(path) => match CString::new(path) {
                Ok(path) => Some(path),
                Err(_) => {
                    log::warn!("Invalid glTF path (contains NUL byte).");
                    return None;
                }
            },
            None => None,
        };
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
        let c_mime = match CString::new(mime_type) {
            Ok(mime) => mime,
            Err(_) => {
                log::warn!("Invalid mime type (contains NUL byte).");
                return;
            }
        };
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

    pub fn root_entity(&mut self) -> Entity {
        unsafe {
            let id = ffi::filament_gltfio_asset_get_root(self.ptr.as_ptr() as *mut _);
            Entity { id }
        }
    }

    pub fn material_instances(&mut self) -> (Vec<MaterialInstance>, Vec<String>) {
        let mut instances = Vec::new();
        let mut names = Vec::new();
        let instance_ptr =
            unsafe { ffi::filament_gltfio_asset_get_instance(self.ptr.as_ptr() as *mut _) };
        let Some(instance) = NonNull::new(instance_ptr as *mut c_void) else {
            return (instances, names);
        };
        let count = unsafe {
            ffi::filament_gltfio_instance_get_material_instance_count(instance.as_ptr() as *mut _)
        };
        for index in 0..count {
            let mi_ptr = unsafe {
                ffi::filament_gltfio_instance_get_material_instance(
                    instance.as_ptr() as *mut _,
                    index,
                )
            };
            if let Some(mi) = NonNull::new(mi_ptr as *mut c_void) {
                let material = MaterialInstance {
                    ptr: mi,
                    owned: false,
                };
                names.push(material.name());
                instances.push(material);
            }
        }
        (instances, names)
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

/// filagui ImGui helper
pub struct ImGuiHelper {
    ptr: NonNull<c_void>,
}

impl ImGuiHelper {
    pub fn create(engine: &mut Engine, view: &mut View, font_path: Option<&str>) -> Option<Self> {
        let c_path = match font_path {
            Some(path) => match CString::new(path) {
                Ok(path) => Some(path),
                Err(_) => {
                    log::warn!("Invalid font path (contains NUL byte).");
                    return None;
                }
            },
            None => None,
        };
        let path_ptr = c_path
            .as_ref()
            .map(|path| path.as_ptr())
            .unwrap_or(std::ptr::null());
        unsafe {
            let ptr = ffi::filagui_imgui_helper_create(
                engine.ptr.as_ptr() as *mut _,
                view.ptr.as_ptr() as *mut _,
                path_ptr,
            );
            NonNull::new(ptr as *mut c_void).map(|ptr| ImGuiHelper { ptr })
        }
    }

    pub fn set_display_size(
        &mut self,
        width: i32,
        height: i32,
        scale_x: f32,
        scale_y: f32,
        flip_vertical: bool,
    ) {
        unsafe {
            ffi::filagui_imgui_helper_set_display_size(
                self.ptr.as_ptr() as *mut _,
                width,
                height,
                scale_x,
                scale_y,
                flip_vertical,
            );
        }
    }

    pub fn render_text(&mut self, delta_seconds: f32, title: &str, body: &str) {
        let c_title = match CString::new(title) {
            Ok(title) => title,
            Err(_) => {
                log::warn!("Invalid UI title text (contains NUL byte).");
                return;
            }
        };
        let c_body = match CString::new(body) {
            Ok(body) => body,
            Err(_) => {
                log::warn!("Invalid UI body text (contains NUL byte).");
                return;
            }
        };
        unsafe {
            ffi::filagui_imgui_helper_render_text(
                self.ptr.as_ptr() as *mut _,
                delta_seconds,
                c_title.as_ptr(),
                c_body.as_ptr(),
            );
        }
    }

    pub fn render_controls(&mut self, delta_seconds: f32) {
        unsafe {
            ffi::filagui_imgui_helper_render_controls(self.ptr.as_ptr() as *mut _, delta_seconds);
        }
    }

    pub fn render_overlay(&mut self, delta_seconds: f32, title: &str, body: &str) {
        let c_title = match CString::new(title) {
            Ok(title) => title,
            Err(_) => {
                log::warn!("Invalid UI title text (contains NUL byte).");
                return;
            }
        };
        let c_body = match CString::new(body) {
            Ok(body) => body,
            Err(_) => {
                log::warn!("Invalid UI body text (contains NUL byte).");
                return;
            }
        };
        unsafe {
            ffi::filagui_imgui_helper_render_overlay(
                self.ptr.as_ptr() as *mut _,
                delta_seconds,
                c_title.as_ptr(),
                c_body.as_ptr(),
            );
        }
    }

    pub fn render_scene_ui(
        &mut self,
        delta_seconds: f32,
        assets_title: &str,
        assets_body: &str,
        object_names: &[*const c_char],
        selected_index: &mut i32,
        selected_kind: &mut i32,
        can_edit_transform: &mut bool,
        position_xyz: &mut [f32; 3],
        rotation_deg_xyz: &mut [f32; 3],
        scale_xyz: &mut [f32; 3],
        light_color_rgb: &mut [f32; 3],
        light_intensity: &mut f32,
        light_dir_xyz: &mut [f32; 3],
        material_names: &[*const c_char],
        selected_material_index: &mut i32,
        material_base_color_rgba: &mut [f32; 4],
        material_metallic: &mut f32,
        material_roughness: &mut f32,
        material_emissive_rgb: &mut [f32; 3],
        material_binding_param_names: &[*const c_char],
        material_binding_sources: &mut [u8],
        material_binding_source_stride: i32,
        material_binding_wrap_repeat_u: &mut [bool],
        material_binding_wrap_repeat_v: &mut [bool],
        material_binding_srgb: &mut [bool],
        material_binding_uv_offset: &mut [f32],
        material_binding_uv_scale: &mut [f32],
        material_binding_uv_rotation_deg: &mut [f32],
        material_binding_pick_index: &mut i32,
        material_binding_apply_index: &mut i32,
        hdr_path: &mut [u8],
        ibl_path: &mut [u8],
        skybox_path: &mut [u8],
        environment_pick_hdr: &mut bool,
        environment_pick_ibl: &mut bool,
        environment_pick_skybox: &mut bool,
        environment_intensity: &mut f32,
        environment_apply: &mut bool,
        environment_generate: &mut bool,
        create_gltf: &mut bool,
        create_light: &mut bool,
        create_environment: &mut bool,
        save_scene: &mut bool,
        load_scene: &mut bool,
        transform_tool_mode: &mut i32,
        delete_selected: &mut bool,
        gizmo_screen_points_xy: &[f32; 8],
        gizmo_visible: bool,
        gizmo_origin_world_xyz: &[f32; 3],
        camera_world_xyz: &[f32; 3],
        gizmo_active_axis: &mut i32,
    ) {
        let c_title = match CString::new(assets_title) {
            Ok(title) => title,
            Err(_) => {
                log::warn!("Invalid scene UI title text (contains NUL byte).");
                return;
            }
        };
        let c_body = match CString::new(assets_body) {
            Ok(body) => body,
            Err(_) => {
                log::warn!("Invalid scene UI body text (contains NUL byte).");
                return;
            }
        };
        let names_ptr = if object_names.is_empty() {
            std::ptr::null()
        } else {
            object_names.as_ptr()
        };
        let material_ptr = if material_names.is_empty() {
            std::ptr::null()
        } else {
            material_names.as_ptr()
        };
        unsafe {
            ffi::filagui_imgui_helper_render_scene_ui(
                self.ptr.as_ptr() as *mut _,
                delta_seconds,
                c_title.as_ptr(),
                c_body.as_ptr(),
                names_ptr,
                object_names.len() as i32,
                selected_index as *mut i32,
                selected_kind as *mut i32,
                can_edit_transform as *mut bool,
                position_xyz.as_mut_ptr(),
                rotation_deg_xyz.as_mut_ptr(),
                scale_xyz.as_mut_ptr(),
                light_color_rgb.as_mut_ptr(),
                light_intensity as *mut f32,
                light_dir_xyz.as_mut_ptr(),
                material_ptr,
                material_names.len() as i32,
                selected_material_index as *mut i32,
                material_base_color_rgba.as_mut_ptr(),
                material_metallic as *mut f32,
                material_roughness as *mut f32,
                material_emissive_rgb.as_mut_ptr(),
                material_binding_param_names.as_ptr(),
                material_binding_param_names.len() as i32,
                material_binding_sources.as_mut_ptr() as *mut c_char,
                material_binding_source_stride,
                material_binding_wrap_repeat_u.as_mut_ptr(),
                material_binding_wrap_repeat_v.as_mut_ptr(),
                material_binding_srgb.as_mut_ptr(),
                material_binding_uv_offset.as_mut_ptr(),
                material_binding_uv_scale.as_mut_ptr(),
                material_binding_uv_rotation_deg.as_mut_ptr(),
                material_binding_pick_index as *mut i32,
                material_binding_apply_index as *mut i32,
                hdr_path.as_mut_ptr() as *mut c_char,
                hdr_path.len() as i32,
                ibl_path.as_mut_ptr() as *mut c_char,
                ibl_path.len() as i32,
                skybox_path.as_mut_ptr() as *mut c_char,
                skybox_path.len() as i32,
                environment_pick_hdr as *mut bool,
                environment_pick_ibl as *mut bool,
                environment_pick_skybox as *mut bool,
                environment_intensity as *mut f32,
                environment_apply as *mut bool,
                environment_generate as *mut bool,
                create_gltf as *mut bool,
                create_light as *mut bool,
                create_environment as *mut bool,
                save_scene as *mut bool,
                load_scene as *mut bool,
                transform_tool_mode as *mut i32,
                delete_selected as *mut bool,
                gizmo_screen_points_xy.as_ptr(),
                gizmo_visible,
                gizmo_origin_world_xyz.as_ptr(),
                camera_world_xyz.as_ptr(),
                gizmo_active_axis as *mut i32,
            );
        }
    }

    pub fn add_mouse_pos(&mut self, x: f32, y: f32) {
        unsafe {
            ffi::filagui_imgui_helper_add_mouse_pos(self.ptr.as_ptr() as *mut _, x, y);
        }
    }

    pub fn add_mouse_button(&mut self, button: i32, down: bool) {
        unsafe {
            ffi::filagui_imgui_helper_add_mouse_button(self.ptr.as_ptr() as *mut _, button, down);
        }
    }

    pub fn add_mouse_wheel(&mut self, wheel_x: f32, wheel_y: f32) {
        unsafe {
            ffi::filagui_imgui_helper_add_mouse_wheel(
                self.ptr.as_ptr() as *mut _,
                wheel_x,
                wheel_y,
            );
        }
    }

    pub fn add_key_event(&mut self, key: i32, down: bool) {
        unsafe {
            ffi::filagui_imgui_helper_add_key_event(self.ptr.as_ptr() as *mut _, key, down);
        }
    }

    pub fn add_input_character(&mut self, codepoint: u32) {
        unsafe {
            ffi::filagui_imgui_helper_add_input_character(self.ptr.as_ptr() as *mut _, codepoint);
        }
    }

    pub fn want_capture_mouse(&mut self) -> bool {
        unsafe { ffi::filagui_imgui_helper_want_capture_mouse(self.ptr.as_ptr() as *mut _) }
    }

    pub fn want_capture_keyboard(&mut self) -> bool {
        unsafe { ffi::filagui_imgui_helper_want_capture_keyboard(self.ptr.as_ptr() as *mut _) }
    }
}

impl Drop for ImGuiHelper {
    fn drop(&mut self) {
        unsafe {
            ffi::filagui_imgui_helper_destroy(self.ptr.as_ptr() as *mut _);
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
            let ptr =
                ffi::filament_vertex_buffer_builder_build(self.ptr, self.engine.as_ptr() as *mut _);
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
            let ptr =
                ffi::filament_index_buffer_builder_build(self.ptr, self.engine.as_ptr() as *mut _);
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
                center[0],
                center[1],
                center[2],
                half_extent[0],
                half_extent[1],
                half_extent[2],
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
