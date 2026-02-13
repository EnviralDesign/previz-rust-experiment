// Auto-generated Filament FFI bindings
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
pub type RenderTarget = c_void;

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
    pub fn filament_view_set_visible_layers(view: *mut View, select: u8, values: u8);
    
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
    pub fn filament_material_instance_set_texture_from_ktx(
        engine: *mut Engine,
        instance: *mut MaterialInstance,
        name: *const c_char,
        ktx_path: *const c_char,
        wrap_repeat_u: bool,
        wrap_repeat_v: bool,
        out_texture: *mut *mut Texture,
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
    pub fn filament_renderable_builder_layer_mask(
        wrapper: *mut RenderableBuilderWrapper,
        select: u8,
        values: u8,
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
        material_binding_param_names: *const *const c_char,
        material_binding_count: i32,
        material_binding_sources: *mut c_char,
        material_binding_source_stride: i32,
        material_binding_wrap_repeat_u: *mut bool,
        material_binding_wrap_repeat_v: *mut bool,
        material_binding_srgb: *mut bool,
        material_binding_uv_offset: *mut f32,
        material_binding_uv_scale: *mut f32,
        material_binding_uv_rotation_deg: *mut f32,
        material_binding_pick_index: *mut i32,
        material_binding_apply_index: *mut i32,
        hdr_path: *mut c_char,
        hdr_path_capacity: i32,
        ibl_path: *mut c_char,
        ibl_path_capacity: i32,
        skybox_path: *mut c_char,
        skybox_path_capacity: i32,
        environment_pick_hdr: *mut bool,
        environment_pick_ibl: *mut bool,
        environment_pick_skybox: *mut bool,
        environment_intensity: *mut f32,
        environment_apply: *mut bool,
        environment_generate: *mut bool,
        create_gltf: *mut bool,
        create_light: *mut bool,
        create_environment: *mut bool,
        save_scene: *mut bool,
        load_scene: *mut bool,
        transform_tool_mode: *mut i32,
        delete_selected: *mut bool,
        gizmo_screen_points_xy: *const f32,
        gizmo_visible: bool,
        gizmo_origin_world_xyz: *const f32,
        camera_world_xyz: *const f32,
        gizmo_active_axis: *mut i32,
    );

    // ========================================================================
    // GPU Pick Pass - Texture, RenderTarget, Readback
    // ========================================================================

    pub fn filament_texture_create_2d(
        engine: *mut Engine,
        width: u32,
        height: u32,
        internal_format: u8,
        usage_flags: u32,
    ) -> *mut Texture;

    pub fn filament_render_target_create(
        engine: *mut Engine,
        color: *mut Texture,
        depth: *mut Texture,
    ) -> *mut RenderTarget;

    pub fn filament_engine_destroy_render_target(
        engine: *mut Engine,
        target: *mut RenderTarget,
    );

    pub fn filament_view_set_render_target(
        view: *mut View,
        target: *mut RenderTarget,
    );

    pub fn filament_renderer_read_pixels(
        renderer: *mut Renderer,
        render_target: *mut RenderTarget,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_buffer: *mut u8,
        buffer_size: u32,
    ) -> bool;

    pub fn filament_renderer_read_pixels_swap_chain(
        renderer: *mut Renderer,
        x: u32,
        y: u32,
        width: u32,
        height: u32,
        out_buffer: *mut u8,
        buffer_size: u32,
    ) -> bool;

    // ========================================================================
    // RenderableManager - material swap for pick pass
    // ========================================================================

    pub fn filament_renderable_get_primitive_count(
        engine: *mut Engine,
        entity_id: i32,
    ) -> i32;

    pub fn filament_renderable_get_material_at(
        engine: *mut Engine,
        entity_id: i32,
        primitive_index: i32,
    ) -> *mut MaterialInstance;

    pub fn filament_renderable_set_material_at(
        engine: *mut Engine,
        entity_id: i32,
        primitive_index: i32,
        mi: *mut MaterialInstance,
    );
    pub fn filament_renderable_set_layer_mask(
        engine: *mut Engine,
        entity_id: i32,
        select: u8,
        values: u8,
    );

    // ========================================================================
    // gltfio - entity enumeration
    // ========================================================================

    pub fn filament_gltfio_asset_get_entities(
        asset: *mut FilamentAsset,
        out_entities: *mut i32,
        max_count: i32,
    ) -> i32;

    pub fn filament_gltfio_asset_get_renderable_entity_count(
        asset: *mut FilamentAsset,
    ) -> i32;
}
