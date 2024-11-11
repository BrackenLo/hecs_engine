//====================================================================

use std::{
    collections::HashSet,
    error::Error,
    fmt::Display,
    hash::{BuildHasherDefault, Hash, Hasher},
};

use common::Size;
use cosmic_text::{Buffer, CacheKey, SwashImage};
use etagere::{euclid::Size2D, AllocId, BucketedAtlasAllocator};
use lru::LruCache;
use rustc_hash::FxHasher;

use crate::{shared::Vertex, texture::Texture, tools};

pub use cosmic_text::{Attrs, Color, Metrics, Shaping, Wrap};

//====================================================================

type FastHasher = BuildHasherDefault<FxHasher>;

pub struct GlyphData {
    alloc_id: AllocId,
    pub uv_start: [f32; 2],
    pub uv_end: [f32; 2],
    pub left: f32,
    pub top: f32,
    pub width: f32,
    pub height: f32,
}

#[derive(Debug)]
pub enum CacheGlyphError {
    NoGlyphImage,
    OutOfSpace,
    LruStorageError,
}

impl Error for CacheGlyphError {}

impl Display for CacheGlyphError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match &self {
            CacheGlyphError::NoGlyphImage => "Unable to get image from proved glyph.",
            CacheGlyphError::OutOfSpace => {
                "Atlas texture is not big enough to store new glyphs - TODO"
            }
            CacheGlyphError::LruStorageError => {
                "Error accessing glyphs from LRU - This shouldn't really happen."
            }
        };

        write!(f, "{}", msg)
    }
}

//====================================================================

pub struct TextAtlas {
    packer: BucketedAtlasAllocator,

    glyphs_in_use: HashSet<CacheKey, FastHasher>,
    cached_glyphs: LruCache<CacheKey, GlyphData, FastHasher>,

    texture: Texture,
    texture_size: Size<u32>,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
}

impl TextAtlas {
    pub fn new(device: &wgpu::Device) -> Self {
        const DEFAULT_START_SIZE: u32 = 256;

        let packer = BucketedAtlasAllocator::new(Size2D::new(
            DEFAULT_START_SIZE as i32,
            DEFAULT_START_SIZE as i32,
        ));
        let glyphs_in_use = HashSet::with_hasher(FastHasher::default());
        let cached_glyphs = LruCache::unbounded_with_hasher(FastHasher::default());

        let texture_size = Size::new(DEFAULT_START_SIZE, DEFAULT_START_SIZE);
        let texture = Texture::from_size(device, texture_size, Some("Text Atlas Texture"), None);

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Atlas Bind Group Layout"),
            entries: &[tools::bgl_texture_entry(0), tools::bgl_sampler_entry(1)],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Atlas Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&texture.view),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&texture.sampler),
                },
            ],
        });

        Self {
            packer,
            glyphs_in_use,
            cached_glyphs,
            texture,
            texture_size,
            bind_group_layout,
            bind_group,
        }
    }

    #[inline]
    pub fn bind_group_layout(&self) -> &wgpu::BindGroupLayout {
        &self.bind_group_layout
    }

    #[inline]
    pub fn bind_group(&self) -> &wgpu::BindGroup {
        &self.bind_group
    }
}

//--------------------------------------------------

impl TextAtlas {
    // Cache glyph if not already and then promote in LRU
    pub fn use_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        font_system: &mut cosmic_text::FontSystem,
        swash_cache: &mut cosmic_text::SwashCache,
        key: &CacheKey,
    ) -> Result<(), CacheGlyphError> {
        // Already has glyph cached
        if self.cached_glyphs.contains(key) {
            self.cached_glyphs.promote(key);
            self.glyphs_in_use.insert(*key);

            Ok(())
        }
        // Try to cache glyph
        else {
            let image = swash_cache
                .get_image_uncached(font_system, *key)
                .ok_or(CacheGlyphError::NoGlyphImage)?;

            self.cache_glyph(device, queue, key, &image)?;

            self.cached_glyphs.promote(key);
            self.glyphs_in_use.insert(*key);
            Ok(())
        }
    }

    #[inline]
    pub fn get_glyph_data(&mut self, key: &CacheKey) -> Option<&GlyphData> {
        self.cached_glyphs.get(key)
    }

    fn cache_glyph(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        key: &CacheKey,
        image: &SwashImage,
    ) -> Result<(), CacheGlyphError> {
        let image_width = image.placement.width;
        let image_height = image.placement.height;

        let size = etagere::Size::new(image_width.max(1) as i32, image_height.max(1) as i32);

        let allocation = loop {
            match self.packer.allocate(size) {
                Some(allocation) => break allocation,

                // Keep trying to free space until error or can allocate
                None => self.free_space(device)?,
            }
        };

        let x = allocation.rectangle.min.x as u32;
        let y = allocation.rectangle.min.y as u32;

        self.texture
            .update_area(queue, &image.data, x, y, image_width, image_height);

        let uv_start = [
            allocation.rectangle.min.x as f32 / self.texture_size.width as f32,
            allocation.rectangle.min.y as f32 / self.texture_size.height as f32,
        ];

        let uv_end = [
            allocation.rectangle.max.x as f32 / self.texture_size.width as f32,
            allocation.rectangle.max.y as f32 / self.texture_size.height as f32,
        ];

        let left = image.placement.left as f32;
        let top = image.placement.top as f32;
        let width = image.placement.width as f32;
        let height = image.placement.height as f32;

        // log::trace!(
        //     "Allocated glyph id {:?}, with size {:?} and uv ({:?}, {:?})",
        //     &key.glyph_id,
        //     size,
        //     uv_start,
        //     uv_end
        // );

        let glyph_data = GlyphData {
            alloc_id: allocation.id,
            uv_start,
            uv_end,
            left,
            top,
            width,
            height,
        };

        self.cached_glyphs.put(*key, glyph_data);

        Ok(())
    }

    fn free_space(&mut self, _device: &wgpu::Device) -> Result<(), CacheGlyphError> {
        //
        match self.cached_glyphs.peek_lru() {
            // Check if last used key is in use. If so, grow atlas
            Some((key, _)) => {
                if self.glyphs_in_use.contains(key) {
                    // TODO - Try to grow glyph cache - Make sure to re-set all glyph data UVs
                    return Err(CacheGlyphError::OutOfSpace);
                }
            }
            // Issues with size of lru
            None => return Err(CacheGlyphError::LruStorageError),
        };

        let (key, val) = self.cached_glyphs.pop_lru().unwrap();

        self.packer.deallocate(val.alloc_id);
        self.cached_glyphs.pop(&key);

        return Ok(());
    }

    #[inline]
    pub fn post_render_trim(&mut self) {
        self.glyphs_in_use.clear();
    }
}

//====================================================================

pub struct TextResources {
    pub font_system: cosmic_text::FontSystem,
    pub swash_cache: cosmic_text::SwashCache,
    pub text_atlas: TextAtlas,
}

impl TextResources {
    pub fn new(device: &wgpu::Device) -> Self {
        Self {
            font_system: cosmic_text::FontSystem::new(),
            swash_cache: cosmic_text::SwashCache::new(),
            text_atlas: TextAtlas::new(device),
        }
    }
}

//====================================================================

#[repr(C)]
#[derive(bytemuck::Pod, bytemuck::Zeroable, Clone, Copy, Debug)]
pub struct TextVertex {
    glyph_pos: [f32; 2],
    glyph_size: [f32; 2],
    uv_start: [f32; 2],
    uv_end: [f32; 2],
    color: u32,
}

impl Vertex for TextVertex {
    fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
        const VERTEX_ATTRIBUTES: [wgpu::VertexAttribute; 5] = wgpu::vertex_attr_array![
            0 => Float32x2,
            1 => Float32x2,
            2 => Float32x2,
            3 => Float32x2,
            4 => Uint32,
        ];

        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<TextVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &VERTEX_ATTRIBUTES,
        }
    }
}

//====================================================================

#[derive(Debug)]
pub struct TextBuffer {
    pub vertex_buffer: wgpu::Buffer,
    pub vertex_count: u32,
    lines: Vec<TextBufferLine>,

    buffer: Buffer,
    color: Color,
}

pub struct TextBufferDescriptor<'a> {
    pub metrics: Metrics,
    pub word_wrap: Wrap,
    pub attributes: Attrs<'a>,
    pub text: &'a str,
    pub width: Option<f32>,
    pub height: Option<f32>,
    pub color: Color,
}

impl<'a> Default for TextBufferDescriptor<'a> {
    fn default() -> Self {
        Self {
            metrics: Metrics::relative(30., 1.2),
            word_wrap: Wrap::WordOrGlyph,
            attributes: Attrs::new(),
            text: "",
            width: Some(800.),
            height: None,
            color: Color::rgb(0, 0, 0),
        }
    }
}

impl TextBuffer {
    pub fn new(
        device: &wgpu::Device,
        font_system: &mut cosmic_text::FontSystem,
        desc: &TextBufferDescriptor,
    ) -> Self {
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: 0,
            usage: wgpu::BufferUsages::VERTEX,
            mapped_at_creation: false,
        });

        let vertex_count = 0;
        let lines = Vec::new();

        let mut buffer = Buffer::new(font_system, desc.metrics);
        buffer.set_size(font_system, desc.width, desc.height);
        buffer.set_wrap(font_system, desc.word_wrap);
        buffer.set_text(font_system, desc.text, desc.attributes, Shaping::Advanced);

        Self {
            vertex_buffer,
            vertex_count,
            lines,
            buffer,
            color: desc.color,
        }
    }

    #[inline]
    pub fn set_metrics(&mut self, font_system: &mut cosmic_text::FontSystem, metrics: Metrics) {
        self.buffer.set_metrics(font_system, metrics);
    }
}

//====================================================================

#[derive(Default, Debug)]
struct TextBufferLine {
    hash: u64,
    length: usize,
}

//====================================================================

struct LocalGlyphData {
    x: f32,
    y: f32,
    key: CacheKey,
    color: Color,
}

//====================================================================

pub fn prep(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    text_resources: &mut TextResources,
    text_buffer: &mut TextBuffer,
) -> Option<Vec<TextVertex>> {
    let mut rebuild_all_lines = false;

    let local_glyph_data = text_buffer
        .buffer
        .layout_runs()
        .enumerate()
        .flat_map(|(index, layout_run)| {
            // Hasher for determining if a line has changed
            let mut hasher = FxHasher::default();

            let mut line_length = 0;

            //--------------------------------------------------

            // Iterate through each glyph in the line - prep and check
            let local_glyph_data = layout_run
                .glyphs
                .iter()
                .map(|glyph| {
                    let physical = glyph.physical((0., 0.), 1.);

                    // Try to prep glyph in atlas
                    if let Err(_) = text_resources.text_atlas.use_glyph(
                        device,
                        queue,
                        &mut text_resources.font_system,
                        &mut text_resources.swash_cache,
                        &physical.cache_key,
                    ) {
                        unimplemented!()
                    }

                    // Check if glyph has specific color to use
                    let color = match glyph.color_opt {
                        Some(color) => color,
                        None => text_buffer.color,
                    };

                    // Hash results to check changes
                    physical.cache_key.hash(&mut hasher);
                    color.hash(&mut hasher);

                    // Count number of glyphs in line
                    line_length += 1;

                    // Data for rebuilding later
                    LocalGlyphData {
                        x: physical.x as f32,
                        y: physical.y as f32 - layout_run.line_y,
                        key: physical.cache_key,
                        color,
                    }
                })
                .collect::<Vec<_>>();

            //--------------------------------------------------

            let line_hash = hasher.finish();

            if text_buffer.lines.len() <= index {
                text_buffer.lines.push(TextBufferLine::default());
            }

            let line_entry = &mut text_buffer.lines[index];

            if line_hash != line_entry.hash {
                // log::trace!("Line '{}' hash updated '{}'", index, line_hash);

                line_entry.hash = line_hash;
                line_entry.length = line_length;

                rebuild_all_lines = true;
            }

            local_glyph_data
        })
        .collect::<Vec<_>>();

    // TODO - OPTIMIZE - Only rebuild lines that need rebuilding
    match rebuild_all_lines {
        true => Some(
            local_glyph_data
                .into_iter()
                .map(|local_data| {
                    let data = text_resources
                        .text_atlas
                        .get_glyph_data(&local_data.key)
                        .unwrap();

                    let x = local_data.x + data.left + data.width / 2.;
                    let y = local_data.y + data.top; // TODO - Run Line

                    TextVertex {
                        glyph_pos: [x, y],
                        glyph_size: [data.width, data.height],
                        uv_start: data.uv_start,
                        uv_end: data.uv_end,
                        color: local_data.color.0,
                    }
                })
                .collect::<Vec<_>>(),
        ),

        false => None,
    }
}

//====================================================================
