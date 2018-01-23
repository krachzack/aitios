
use super::tri::Triangle;
use super::vtx::Position;

use ::image::{ImageBuffer, Pixel};

use std::ops::{Deref, DerefMut};

pub trait Rasterize {
    fn rasterize<F>(&self, raster_width: usize, raster_height: usize, render_pixel_at: F)
        where F : FnMut(usize, usize);

    /// Renders a thing already transfomed into image space into the given image buffer
    fn rasterize_to_image<P, C, F>(&self, buf: &mut ImageBuffer<P, C>, shader_fn: F)
        where P: Pixel + 'static,
            C: Deref<Target = [P::Subpixel]> + DerefMut,
            F : Fn(usize, usize) -> P
    {
        self.rasterize(
            buf.width() as usize, buf.height() as usize,
            |x, y| buf.put_pixel(x as u32, y as u32, shader_fn(x, y))
        )
    }
}

impl<V> Rasterize for Triangle<V>
    where V : Position {

    /// See: http://forum.devmaster.net/t/advanced-rasterization/6145
    #[allow(non_snake_case)]
    fn rasterize<F>(&self, raster_width: usize, raster_height: usize, mut render_pixel_at: F)
        where F : FnMut(usize, usize)
    {
        /*if self.area() < 0.00000001 {
            return; // ignore zero area triangles
        }*/

        let v1 = self.vertices[0].position();
        let v2 = self.vertices[1].position();
        let v3 = self.vertices[2].position();

        // 28.4 fixed-point coordinates
        let Y1 = (16.0 * v1.y).round() as i64;
        let Y2 = (16.0 * v2.y).round() as i64;
        let Y3 = (16.0 * v3.y).round() as i64;

        let X1 = (16.0 * v1.x).round() as i64;
        let X2 = (16.0 * v2.x).round() as i64;
        let X3 = (16.0 * v3.x).round() as i64;

        // Deltas
        let DX12 = X1 - X2;
        let DX23 = X2 - X3;
        let DX31 = X3 - X1;

        let DY12 = Y1 - Y2;
        let DY23 = Y2 - Y3;
        let DY31 = Y3 - Y1;

        // Fixed-point deltas
        let FDX12 = DX12 << 4;
        let FDX23 = DX23 << 4;
        let FDX31 = DX31 << 4;

        let FDY12 = DY12 << 4;
        let FDY23 = DY23 << 4;
        let FDY31 = DY31 << 4;

        // Bounding rectangle
        let mut minx = ([X1, X2, X3].iter().min().unwrap() + 0xF) >> 4;
        let mut maxx = ([X1, X2, X3].iter().max().unwrap() + 0xF) >> 4;
        let mut miny = ([Y1, Y2, Y3].iter().min().unwrap() + 0xF) >> 4;
        let mut maxy = ([Y1, Y2, Y3].iter().max().unwrap() + 0xF) >> 4;

        // Clamp to raster size "cull"
        {
            let last_x = raster_width as i64;
            let last_y = raster_height as i64;

            if minx < 0 { minx = 0; }
            if minx > last_x { minx = last_x; }
            if maxx < 0 { maxx = 0; }
            if maxx > last_x { maxx = last_x; }

            if miny < 0 { miny = 0; }
            if miny > last_y { miny = last_y; }
            if maxy < 0 { maxy = 0; }
            if maxy > last_y { maxy = last_y; }
        }

        // Half-edge constants
        let mut C1 = DY12 * X1 - DX12 * Y1;
        let mut C2 = DY23 * X2 - DX23 * Y2;
        let mut C3 = DY31 * X3 - DX31 * Y3;

        // Correct for fill convention
        if DY12 < 0 || (DY12 == 0 && DX12 > 0) { C1 += 1; }
        if DY23 < 0 || (DY23 == 0 && DX23 > 0) { C2 += 1; }
        if DY31 < 0 || (DY31 == 0 && DX31 > 0) { C3 += 1; }

        let mut CY1 = C1 + DX12 * (miny << 4) - DY12 * (minx << 4);
        let mut CY2 = C2 + DX23 * (miny << 4) - DY23 * (minx << 4);
        let mut CY3 = C3 + DX31 * (miny << 4) - DY31 * (minx << 4);

        for y in miny..maxy {
            let mut CX1 = CY1;
            let mut CX2 = CY2;
            let mut CX3 = CY3;

            for x in minx..maxx {
                if CX1 > 0 && CX2 > 0 && CX3 > 0 {
                //if CX1 >= 0 && CX2 >= 0 && CX3 >= 0 {
                    let x = x as usize;
                    let y = y as usize;
                    render_pixel_at(x, y);
                }

                CX1 -= FDY12;
                CX2 -= FDY23;
                CX3 -= FDY31;
            }

            CY1 += FDX12;
            CY2 += FDX23;
            CY3 += FDX31;
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use ::geom::scene::Scene;
    use ::image::{self, Rgb};
    use std::fs::File;
    use ::cgmath::{Vector2, Vector3};

    #[test]
    fn test_render_positions() {
        let entity = &Scene::load_from_file("test-scenes/buddha-scene-iron-concrete/buddha-scene-iron-concrete.obj").entities[0];

        let mut world_positions = ImageBuffer::from_pixel(4096, 4096, Rgb { data: [0, 0, 0] });

        entity.triangles()
            .map(|t| Triangle::new(
                (t.vertices[0].position, Vector2::new(t.vertices[0].texcoords.x * 4096.0, (1.0 - t.vertices[0].texcoords.y) * 4096.0)),
                (t.vertices[1].position, Vector2::new(t.vertices[1].texcoords.x * 4096.0, (1.0 - t.vertices[1].texcoords.y) * 4096.0)),
                (t.vertices[2].position, Vector2::new(t.vertices[2].texcoords.x * 4096.0, (1.0 - t.vertices[2].texcoords.y) * 4096.0))
            ))
            .for_each(|t| t.rasterize_to_image(&mut world_positions, |x, y| {
                let interpolated_position = t.interpolate_at(Vector3::new(x as f32, y as f32, 0.0), |v| v.0);
                let color = [
                    (interpolated_position.x.fract() * 255.0) as u8,
                    (interpolated_position.y.fract() * 255.0) as u8,
                    (interpolated_position.z.fract() * 255.0) as u8
                ];
                Rgb { data: color }
            }));

        let ref mut fout = File::create("test.png").unwrap();

        // Write the contents of this image to the Writer in PNG format.
        image::ImageRgb8(world_positions).save(fout, image::PNG).unwrap();
    }

    /*impl Position for (Vector3<f32>, Vector2<f32>) {
        // Triangles in UV space
        fn position(&self) -> Vector3<f32> {
            Vector3::new(self.1.x, self.1.y, 0.0)
        }
    }*/
}
