use imgui as imgui_rs;
use imgui_rs::{Context, DrawData, TextureId};
use skia_safe::{AlphaType, Matrix, Paint};
use std::collections::HashMap;

pub struct Renderer {
    // this holds the skia formatted font atlas
    images: HashMap<usize, skia_safe::Paint>,
    img_idx: usize // this is incremented each time an image is registered and is the id returned to the caller
}

impl Renderer {
    pub fn load_image_rgba8(img: &[u8], width: i32, height: i32) -> skia_safe::Paint {
        let mut paint = Paint::default();
        let dimensions = skia_safe::ISize::new(width, height);
        let img_info_rgba8 = skia_safe::ImageInfo::new_n32(dimensions, AlphaType::Unknown, None);

        let pixels = unsafe {
            skia_safe::Data::new_bytes(img)
        };

        let pixmap = skia_safe::Pixmap::new(&img_info_rgba8, img, img_info_rgba8.min_row_bytes());
        let image = skia_safe::Image::from_raster_data(&img_info_rgba8, pixels, pixmap.row_bytes());

        let local_matrix = skia_safe::Matrix::scale((1.0 / width as f32, 1.0 / height as f32));
        let sampling_options = skia_safe::SamplingOptions::new(skia_safe::FilterMode::Nearest, skia_safe::MipmapMode::None);
        let tile_mode = skia_safe::TileMode::Repeat;

        let image_shader = image.unwrap().to_shader((tile_mode, tile_mode), sampling_options, &local_matrix);

        paint.set_shader(image_shader);
        paint.set_color(skia_safe::Color::WHITE);

        return paint;
    }

    pub fn register_image(&mut self, paint: skia_safe::Paint) -> TextureId {
        self.images.insert(self.img_idx, paint);
        self.img_idx += 1;
        return TextureId::new(self.img_idx - 1);
    }

    pub fn update_image(&mut self, texid: &TextureId, paint: skia_safe::Paint) {
        self.images.insert(texid.id(), paint);
    }

    pub fn release_image(&mut self, texid: TextureId) {
        self.images.remove(&texid.id());
    }

    fn build_paint(atlas: &mut imgui::FontAtlasRefMut, font_paint: &mut skia_safe::Paint)
    {
        let imfont_texture = atlas.build_alpha8_texture();
        let dimensions = skia_safe::ISize::new(imfont_texture.width as i32, imfont_texture.height as i32);
        let imgfont_a8 = skia_safe::ImageInfo::new_a8(dimensions);
        
        let pixels = unsafe {
            skia_safe::Data::new_bytes(imfont_texture.data)
        };

        let pixmap = skia_safe::Pixmap::new(&imgfont_a8, imfont_texture.data, imgfont_a8.min_row_bytes());
        let font_image = skia_safe::Image::from_raster_data(&imgfont_a8, pixels, pixmap.row_bytes());

        let local_matrix = skia_safe::Matrix::scale((1.0 / imfont_texture.width as f32, 1.0 / imfont_texture.height as f32));
        let sampling_options = skia_safe::SamplingOptions::new(skia_safe::FilterMode::Nearest, skia_safe::MipmapMode::None);
        let tile_mode = skia_safe::TileMode::Repeat;

        let font_shader = font_image.unwrap().to_shader((tile_mode, tile_mode), sampling_options, &local_matrix);

        font_paint.set_shader(font_shader);
        font_paint.set_color(skia_safe::Color::WHITE);
    }

    pub fn new(im_context: &mut Context) -> Self
    {
        let mut ret = Renderer {
            images: HashMap::new(),
            img_idx: 0,
        };

        let mut font_paint = skia_safe::Paint::default();
        Self::build_paint(&mut im_context.fonts(), &mut font_paint);
        ret.register_image(font_paint);

        ret
    }

    pub fn render_imgui(&self, canvas: &mut skia_safe::Canvas, data: &DrawData, )
    {
        canvas.save();
        let mut matrix = Matrix::new_identity();
        matrix.set_scale((1., 1.), None);
    
        canvas.set_matrix(&matrix.into());
        for draw_list in data.draw_lists() {
            let mut idx: Vec<u16> = Vec::new();
            let mut pos: Vec<skia_safe::Point> = Vec::new();
            let mut uv: Vec<skia_safe::Point> = Vec::new();
            let mut color: Vec<skia_safe::Color> = Vec::new();

            // we've got to translate the vertex buffer from imgui into Skia friendly types
            // thankfully skia_safe gives us a constructor for Color so we don't have to swizzle the colors as Skia expects BGR order
            for vertex in draw_list.vtx_buffer() {
                pos.push(skia_safe::Point {
                    x: vertex.pos[0],
                    y: vertex.pos[1]
                });

                uv.push(skia_safe::Point {
                    x: vertex.uv[0],
                    y: vertex.uv[1]
                });

                color.push(skia_safe::Color::from_argb(
                    vertex.col[3],
                    vertex.col[0],
                    vertex.col[1],
                    vertex.col[2],
                ));
            }
            
            // we build our index buffer
            for index in draw_list.idx_buffer() {
                idx.push(*index);
            }

            // so now we've got to loop through imgui's cmd buffer and draw everything with canvas.draw_vertices
            for cmd in draw_list.commands() {
                let mut arc = skia_safe::AutoCanvasRestore::guard(canvas, true);
                match cmd {
                    imgui::DrawCmd::RawCallback {
                        ..
                    } => {
                        todo!("Raw callbacks unimplemented!")
                    }
                    imgui::DrawCmd::ResetRenderState => {
                        todo!("Reset render state unimplemented!")
                    }
                    imgui::DrawCmd::Elements {
                        count,
                        cmd_params,
                    } => {
                        //TODO: Handle images that aren't our font atlas
                        let id_index = cmd_params.texture_id;
                        let paint = &self.images[&id_index.id()];

                        let clip_rect = cmd_params.clip_rect;
                        let skclip_rect = skia_safe::Rect::new(clip_rect[0], clip_rect[1], clip_rect[2], clip_rect[3]);

                        let vertex_mode = skia_safe::vertices::VertexMode::Triangles;
                        let idx_offset = cmd_params.idx_offset;
                        let idx_slice = Some(&idx[idx_offset .. idx_offset + count]);

                        arc.clip_rect(skclip_rect, skia_safe::ClipOp::default(), true);
                        let vertices = skia_safe::Vertices::new_copy(vertex_mode, &pos, &uv, &color, idx_slice);
                        arc.draw_vertices(&vertices, skia_safe::BlendMode::Modulate, Some(paint));
                    }
                }
            }
        }
        canvas.restore();
    }
}
