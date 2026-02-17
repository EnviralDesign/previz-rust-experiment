use crate::filament::{
    Camera, ElementType, Engine, Entity, IndexBuffer, IndexType, Material, MaterialInstance,
    PrimitiveType, RenderTarget, Renderer, Scene, Texture, TextureInternalFormat, TextureUsage,
    VertexAttribute, VertexBuffer, View,
};

const MAX_VERTICES: usize = 96_000;
const MAX_INDICES: usize = 288_000;
const LAYER_EGUI: u8 = 0x10;

#[derive(Clone, Copy)]
struct UiVertex {
    pos: [f32; 2],
    uv: [f32; 2],
    color: [u8; 4],
}

pub struct EguiOverlay {
    _scene: Scene,
    view: View,
    camera: Camera,
    _entity: Entity,
    _material: Material,
    material_instance: MaterialInstance,
    vertex_buffer: VertexBuffer,
    index_buffer: IndexBuffer,
    atlas_texture_id: Option<egui::TextureId>,
    atlas_texture: Option<Texture>,
    atlas_texture_size: Option<[u32; 2]>,
    atlas_size: Option<[u32; 2]>,
    atlas_pixels: Vec<u8>,
    positions: Vec<[f32; 3]>,
    uvs: Vec<[f32; 2]>,
    colors: Vec<[u8; 4]>,
    indices: Vec<u32>,
    last_index_count: usize,
    warned_texture_mismatch: bool,
    warned_mesh_overflow: bool,
}

impl EguiOverlay {
    pub fn new(engine: &mut Engine, width: u32, height: u32) -> Option<Self> {
        let mut scene = engine.create_scene()?;
        let mut view = engine.create_view()?;
        let mut entity_manager = engine.entity_manager()?;
        let camera_entity = entity_manager.create();
        let mut camera = engine.create_camera(camera_entity)?;
        camera.set_projection_ortho(
            0.0,
            width.max(1) as f64,
            height.max(1) as f64,
            0.0,
            -1.0,
            1.0,
        );
        view.set_viewport(0, 0, width.max(1), height.max(1));
        view.set_scene(&mut scene);
        view.set_camera(&mut camera);
        view.set_post_processing_enabled(false);
        view.set_visible_layers(0xFF, LAYER_EGUI);

        let mut material =
            engine.create_material(include_bytes!(concat!(env!("OUT_DIR"), "/eguiUi.filamat")))?;
        let mut material_instance = material.create_instance()?;

        let mut vertex_buffer = engine
            .vertex_buffer_builder()
            .vertex_count(MAX_VERTICES as u32)
            .buffer_count(3)
            .attribute(VertexAttribute::Position, 0, ElementType::Float3, 0, 12)
            .attribute(VertexAttribute::UV0, 1, ElementType::Float2, 0, 8)
            .attribute(VertexAttribute::Color, 2, ElementType::UByte4, 0, 4)
            .normalized(VertexAttribute::Color, true)
            .build()?;
        let mut index_buffer = engine
            .index_buffer_builder()
            .index_count(MAX_INDICES as u32)
            .buffer_type(IndexType::UInt)
            .build()?;

        let entity = entity_manager.create();
        engine
            .renderable_builder(1)
            .bounding_box(
                [width.max(1) as f32 * 0.5, height.max(1) as f32 * 0.5, 0.0],
                [width.max(1) as f32 * 0.5, height.max(1) as f32 * 0.5, 1.0],
            )
            .material(0, &mut material_instance)
            .geometry(
                0,
                PrimitiveType::Triangles,
                &mut vertex_buffer,
                &mut index_buffer,
            )
            .layer_mask(0xFF, LAYER_EGUI)
            .culling(false)
            .build(entity);
        scene.add_entity(entity);

        let mut positions = vec![[0.0, 0.0, 0.0]; MAX_VERTICES];
        let mut uvs = vec![[0.0, 0.0]; MAX_VERTICES];
        let mut colors = vec![[0, 0, 0, 0]; MAX_VERTICES];
        let indices = vec![0u32; MAX_INDICES];
        positions[0] = [0.0, 0.0, 0.0];
        uvs[0] = [0.0, 0.0];
        colors[0] = [0, 0, 0, 0];
        vertex_buffer.set_buffer_at(0, &positions[..1], 0);
        vertex_buffer.set_buffer_at(1, &uvs[..1], 0);
        vertex_buffer.set_buffer_at(2, &colors[..1], 0);
        index_buffer.set_buffer(&indices[..1], 0);

        Some(Self {
            _scene: scene,
            view,
            camera,
            _entity: entity,
            _material: material,
            material_instance,
            vertex_buffer,
            index_buffer,
            atlas_texture_id: None,
            atlas_texture: None,
            atlas_texture_size: None,
            atlas_size: None,
            atlas_pixels: Vec::new(),
            positions,
            uvs,
            colors,
            indices,
            last_index_count: 1,
            warned_texture_mismatch: false,
            warned_mesh_overflow: false,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        let w = width.max(1);
        let h = height.max(1);
        self.view.set_viewport(0, 0, w, h);
        self.camera
            .set_projection_ortho(0.0, w as f64, h as f64, 0.0, -1.0, 1.0);
    }

    pub fn set_render_target(&mut self, target: Option<&RenderTarget>) {
        self.view.set_render_target(target);
    }

    pub fn update(
        &mut self,
        engine: &mut Engine,
        clipped_primitives: &[egui::ClippedPrimitive],
        textures_delta: &egui::TexturesDelta,
        pixels_per_point: f32,
        screen_size_px: [u32; 2],
    ) -> Result<(), String> {
        self.resize(screen_size_px[0], screen_size_px[1]);
        self.apply_textures(engine, textures_delta)?;
        self.build_mesh(clipped_primitives, pixels_per_point);
        self.upload_mesh();
        Ok(())
    }

    pub fn render(&self, renderer: &mut Renderer) {
        renderer.render(&self.view);
    }

    fn apply_textures(
        &mut self,
        engine: &mut Engine,
        textures_delta: &egui::TexturesDelta,
    ) -> Result<(), String> {
        let mut atlas_dirty = false;

        for (texture_id, image_delta) in &textures_delta.set {
            if let Some(current) = self.atlas_texture_id {
                if current != *texture_id {
                    if !self.warned_texture_mismatch {
                        log::warn!(
                            "egui texture id {:?} unsupported; current atlas is {:?}.",
                            texture_id,
                            current
                        );
                        self.warned_texture_mismatch = true;
                    }
                    continue;
                }
            } else {
                self.atlas_texture_id = Some(*texture_id);
            }

            let (w, h, pixels) = image_to_rgba8(image_delta)?;
            if let Some([x, y]) = image_delta.pos {
                let x = u32::try_from(x).map_err(|_| "egui atlas x overflow".to_string())?;
                let y = u32::try_from(y).map_err(|_| "egui atlas y overflow".to_string())?;
                let required_w = x.saturating_add(w);
                let required_h = y.saturating_add(h);
                self.ensure_atlas_capacity(required_w, required_h)?;

                let [atlas_w, atlas_h] = self
                    .atlas_size
                    .ok_or_else(|| "missing egui atlas size".to_string())?;
                if x.saturating_add(w) > atlas_w || y.saturating_add(h) > atlas_h {
                    return Err("egui partial texture update exceeds atlas bounds".to_string());
                }

                let row_bytes = (w as usize) * 4;
                for row in 0..(h as usize) {
                    let src = row * row_bytes;
                    let dst = (((y as usize) + row) * (atlas_w as usize) + (x as usize)) * 4;
                    self.atlas_pixels[dst..dst + row_bytes]
                        .copy_from_slice(&pixels[src..src + row_bytes]);
                }
                atlas_dirty = true;
            } else {
                self.atlas_size = Some([w, h]);
                self.atlas_pixels = pixels;
                atlas_dirty = true;
            }
        }

        for texture_id in &textures_delta.free {
            let replaced_in_set = textures_delta
                .set
                .iter()
                .any(|(set_id, _)| set_id == texture_id);
            if Some(*texture_id) == self.atlas_texture_id && !replaced_in_set {
                self.clear_atlas_state();
                break;
            }
        }

        if !atlas_dirty {
            return Ok(());
        }
        let [w, h] = self
            .atlas_size
            .ok_or_else(|| "missing egui atlas dimensions after update".to_string())?;
        let expected = (w as usize) * (h as usize) * 4;
        if self.atlas_pixels.len() != expected {
            return Err(format!(
                "egui atlas byte count mismatch: have {}, expected {}",
                self.atlas_pixels.len(),
                expected
            ));
        }

        let needs_texture_recreate = self.atlas_texture.is_none()
            || self
                .atlas_texture_size
                .map(|size| size != [w, h])
                .unwrap_or(true);
        if needs_texture_recreate {
                let texture = engine
                    .create_texture_2d(
                        w,
                        h,
                        TextureInternalFormat::Rgba8,
                        TextureUsage::or(TextureUsage::Sampleable, TextureUsage::Uploadable),
                    )
                    .ok_or_else(|| "failed to create egui atlas texture".to_string())?;
            self.atlas_texture = Some(texture);
            if !self
                .material_instance
                .set_texture(
                    "albedo",
                    self.atlas_texture
                        .as_ref()
                        .ok_or_else(|| "missing egui atlas texture".to_string())?,
                    true,
                    false,
                    false,
                )
            {
                return Err("failed to bind egui atlas texture".to_string());
            }
            self.atlas_texture_size = Some([w, h]);
        }
        if let Some(texture) = self.atlas_texture.as_mut() {
            if !engine.set_texture_image_rgba8(texture, w, h, &self.atlas_pixels) {
                return Err("failed to upload egui atlas texture".to_string());
            }
        } else {
            return Err("missing egui atlas texture".to_string());
        }

        Ok(())
    }

    fn ensure_atlas_capacity(&mut self, required_w: u32, required_h: u32) -> Result<(), String> {
        let [cur_w, cur_h] = self.atlas_size.unwrap_or([0, 0]);
        if cur_w >= required_w && cur_h >= required_h {
            return Ok(());
        }

        let new_w = cur_w.max(required_w).max(1);
        let new_h = cur_h.max(required_h).max(1);
        let mut new_pixels = vec![0u8; (new_w as usize) * (new_h as usize) * 4];

        if cur_w > 0 && cur_h > 0 && !self.atlas_pixels.is_empty() {
            let copy_w_bytes = (cur_w as usize) * 4;
            for row in 0..(cur_h as usize) {
                let src = row * copy_w_bytes;
                let dst = row * (new_w as usize) * 4;
                new_pixels[dst..dst + copy_w_bytes]
                    .copy_from_slice(&self.atlas_pixels[src..src + copy_w_bytes]);
            }
        }

        self.atlas_size = Some([new_w, new_h]);
        self.atlas_pixels = new_pixels;
        Ok(())
    }

    fn clear_atlas_state(&mut self) {
        self.atlas_texture_id = None;
        self.atlas_texture = None;
        self.atlas_texture_size = None;
        self.atlas_size = None;
        self.atlas_pixels.clear();
    }

    fn build_mesh(&mut self, clipped_primitives: &[egui::ClippedPrimitive], pixels_per_point: f32) {
        let ppp = pixels_per_point.max(0.01);
        let mut vertex_count = 1usize;
        let mut index_count = 0usize;
        self.warned_mesh_overflow = false;

        for clipped in clipped_primitives {
            let egui::epaint::Primitive::Mesh(mesh) = &clipped.primitive else {
                continue;
            };
            if let Some(texture_id) = self.atlas_texture_id {
                if mesh.texture_id != texture_id {
                    continue;
                }
            }

            let clip_min_x = clipped.clip_rect.min.x * ppp;
            let clip_min_y = clipped.clip_rect.min.y * ppp;
            let clip_max_x = clipped.clip_rect.max.x * ppp;
            let clip_max_y = clipped.clip_rect.max.y * ppp;
            let clip_rect = [clip_min_x, clip_min_y, clip_max_x, clip_max_y];

            let mut tri = 0usize;
            while tri + 2 < mesh.indices.len() {
                let i0 = mesh.indices[tri] as usize;
                let i1 = mesh.indices[tri + 1] as usize;
                let i2 = mesh.indices[tri + 2] as usize;
                tri += 3;
                if i0 >= mesh.vertices.len() || i1 >= mesh.vertices.len() || i2 >= mesh.vertices.len() {
                    continue;
                }

                let v0 = mesh_vertex_to_ui(mesh.vertices[i0], ppp);
                let v1 = mesh_vertex_to_ui(mesh.vertices[i1], ppp);
                let v2 = mesh_vertex_to_ui(mesh.vertices[i2], ppp);
                let clipped_poly = clip_triangle([v0, v1, v2], clip_rect);
                if clipped_poly.len() < 3 {
                    continue;
                }

                for i in 1..(clipped_poly.len() - 1) {
                    if vertex_count + 3 >= MAX_VERTICES || index_count + 3 >= MAX_INDICES {
                        if !self.warned_mesh_overflow {
                            log::warn!(
                                "egui mesh overflow: clamping to {} vertices / {} indices.",
                                MAX_VERTICES,
                                MAX_INDICES
                            );
                            self.warned_mesh_overflow = true;
                        }
                        break;
                    }
                    let a = clipped_poly[0];
                    let b = clipped_poly[i];
                    let c = clipped_poly[i + 1];
                    self.positions[vertex_count] = [a.pos[0], a.pos[1], 0.0];
                    self.uvs[vertex_count] = a.uv;
                    self.colors[vertex_count] = a.color;
                    self.indices[index_count] = vertex_count as u32;
                    vertex_count += 1;
                    index_count += 1;

                    self.positions[vertex_count] = [b.pos[0], b.pos[1], 0.0];
                    self.uvs[vertex_count] = b.uv;
                    self.colors[vertex_count] = b.color;
                    self.indices[index_count] = vertex_count as u32;
                    vertex_count += 1;
                    index_count += 1;

                    self.positions[vertex_count] = [c.pos[0], c.pos[1], 0.0];
                    self.uvs[vertex_count] = c.uv;
                    self.colors[vertex_count] = c.color;
                    self.indices[index_count] = vertex_count as u32;
                    vertex_count += 1;
                    index_count += 1;
                }
                if self.warned_mesh_overflow {
                    break;
                }
            }
            if self.warned_mesh_overflow {
                break;
            }
        }

        for index in index_count..MAX_INDICES {
            self.indices[index] = 0;
        }
        self.last_index_count = index_count.max(1);
        if vertex_count == 1 {
            self.positions[0] = [0.0, 0.0, 0.0];
            self.uvs[0] = [0.0, 0.0];
            self.colors[0] = [0, 0, 0, 0];
        }
        for index in 0..self.last_index_count {
            if self.indices[index] as usize >= vertex_count {
                self.indices[index] = 0;
            }
        }
    }

    fn upload_mesh(&mut self) {
        self.vertex_buffer
            .set_buffer_at(0, &self.positions[..MAX_VERTICES.min(self.positions.len())], 0);
        self.vertex_buffer
            .set_buffer_at(1, &self.uvs[..MAX_VERTICES.min(self.uvs.len())], 0);
        self.vertex_buffer
            .set_buffer_at(2, &self.colors[..MAX_VERTICES.min(self.colors.len())], 0);
        // Filament renderables created with a fixed index count can still consume
        // the full index range; upload the full buffer with unused entries zeroed.
        self.index_buffer.set_buffer(&self.indices[..MAX_INDICES], 0);
    }
}

fn image_to_rgba8(image_delta: &egui::epaint::image::ImageDelta) -> Result<(u32, u32, Vec<u8>), String> {
    match &image_delta.image {
        egui::ImageData::Color(image) => {
            let w = u32::try_from(image.width()).map_err(|_| "egui color image width overflow".to_string())?;
            let h = u32::try_from(image.height()).map_err(|_| "egui color image height overflow".to_string())?;
            let mut out = Vec::with_capacity(image.pixels.len() * 4);
            for pixel in &image.pixels {
                out.extend_from_slice(&pixel.to_array());
            }
            Ok((w, h, out))
        }
        egui::ImageData::Font(image) => {
            let w = u32::try_from(image.width()).map_err(|_| "egui font image width overflow".to_string())?;
            let h = u32::try_from(image.height()).map_err(|_| "egui font image height overflow".to_string())?;
            let mut out = Vec::with_capacity((w as usize) * (h as usize) * 4);
            for pixel in image.srgba_pixels(None) {
                out.extend_from_slice(&pixel.to_array());
            }
            Ok((w, h, out))
        }
    }
}

fn mesh_vertex_to_ui(v: egui::epaint::Vertex, pixels_per_point: f32) -> UiVertex {
    UiVertex {
        pos: [v.pos.x * pixels_per_point, v.pos.y * pixels_per_point],
        uv: [v.uv.x, v.uv.y],
        color: v.color.to_array(),
    }
}

fn clip_triangle(triangle: [UiVertex; 3], rect: [f32; 4]) -> Vec<UiVertex> {
    let mut polygon = vec![triangle[0], triangle[1], triangle[2]];
    polygon = clip_polygon_edge(
        &polygon,
        |p| p.pos[0] >= rect[0],
        |a, b| intersect_at_x(a, b, rect[0]),
    );
    polygon = clip_polygon_edge(
        &polygon,
        |p| p.pos[0] <= rect[2],
        |a, b| intersect_at_x(a, b, rect[2]),
    );
    polygon = clip_polygon_edge(
        &polygon,
        |p| p.pos[1] >= rect[1],
        |a, b| intersect_at_y(a, b, rect[1]),
    );
    clip_polygon_edge(
        &polygon,
        |p| p.pos[1] <= rect[3],
        |a, b| intersect_at_y(a, b, rect[3]),
    )
}

fn clip_polygon_edge<FIn, FIntersect>(
    input: &[UiVertex],
    inside: FIn,
    intersect: FIntersect,
) -> Vec<UiVertex>
where
    FIn: Fn(UiVertex) -> bool,
    FIntersect: Fn(UiVertex, UiVertex) -> UiVertex,
{
    if input.is_empty() {
        return Vec::new();
    }
    let mut output = Vec::new();
    let mut prev = *input.last().unwrap_or(&input[0]);
    let mut prev_inside = inside(prev);
    for &current in input {
        let cur_inside = inside(current);
        match (prev_inside, cur_inside) {
            (true, true) => {
                output.push(current);
            }
            (true, false) => {
                output.push(intersect(prev, current));
            }
            (false, true) => {
                output.push(intersect(prev, current));
                output.push(current);
            }
            (false, false) => {}
        }
        prev = current;
        prev_inside = cur_inside;
    }
    output
}

fn intersect_at_x(a: UiVertex, b: UiVertex, x: f32) -> UiVertex {
    let dx = b.pos[0] - a.pos[0];
    let t = if dx.abs() <= f32::EPSILON {
        0.0
    } else {
        ((x - a.pos[0]) / dx).clamp(0.0, 1.0)
    };
    interpolate_vertex(a, b, t)
}

fn intersect_at_y(a: UiVertex, b: UiVertex, y: f32) -> UiVertex {
    let dy = b.pos[1] - a.pos[1];
    let t = if dy.abs() <= f32::EPSILON {
        0.0
    } else {
        ((y - a.pos[1]) / dy).clamp(0.0, 1.0)
    };
    interpolate_vertex(a, b, t)
}

fn interpolate_vertex(a: UiVertex, b: UiVertex, t: f32) -> UiVertex {
    let inv_t = 1.0 - t;
    let color = [
        ((a.color[0] as f32) * inv_t + (b.color[0] as f32) * t).round() as u8,
        ((a.color[1] as f32) * inv_t + (b.color[1] as f32) * t).round() as u8,
        ((a.color[2] as f32) * inv_t + (b.color[2] as f32) * t).round() as u8,
        ((a.color[3] as f32) * inv_t + (b.color[3] as f32) * t).round() as u8,
    ];
    UiVertex {
        pos: [
            a.pos[0] * inv_t + b.pos[0] * t,
            a.pos[1] * inv_t + b.pos[1] * t,
        ],
        uv: [a.uv[0] * inv_t + b.uv[0] * t, a.uv[1] * inv_t + b.uv[1] * t],
        color,
    }
}
