// ffmpeg -framerate 30 -start_number 1 -i ./animation/%00d.png -pix_fmt yuv420p out.mp4

#![allow(dead_code)]
#![allow(unused_variables)]
#![allow(unused_imports)]
#![allow(unused_mut)]

extern crate image;

use image::{Rgb, RgbImage, ColorType, save_buffer};
use rayon::prelude::*;
use std::fs::File;
use std::sync::{Arc, Mutex};
use std::ops::Add;
use colorgrad::Color;
use colorgrad::Gradient;

struct Complex{
    rel: f64,
    img: f64
}

impl Complex{
    fn distance_from_origin(&self) -> f64{
        return (self.rel * self.rel) + (self.img * self.img);
    }

    fn square(&self) -> Complex{
        return Complex { rel: ((self.rel * self.rel) - (self.img * self.img)), 
                         img: (2.0 * self.rel * self.img) }
    }

    fn add(&self, c: &Complex) -> Complex{
        return Complex { rel: (self.rel + c.rel), img: (self.img + c.img) }
    }
}

struct MandelbrottZoom{
    width: u32,
    height: u32,
    max_iterations: i32,
    starting_zoom: f64,
    ending_zoom: f64,
    center: Complex,
    dry_run: bool,
    color_pallette: Gradient,
    number_of_frames: i32

}

impl MandelbrottZoom{

    fn render_animation(&self){
        (0..self.number_of_frames).into_par_iter().for_each(|frame|{
            self.render_animation_frame(frame)
        });
    }

    fn run_multithreaded(&self){
        let img = Arc::new(Mutex::new(RgbImage::new(self.width, self.height)));
        let img_in = Arc::clone(&img);

        (0..self.height).into_par_iter().for_each(|i| {
            self.render_row_parallel(&img_in, i as i32);
        });

        img.lock().unwrap().save("hello.png").unwrap();
    }

    fn run_multithreaded_fast(&self){
        let mut pixels = vec![0; self.width as usize * self.height as usize * 3];
        let bands: Vec<(usize, &mut [u8])> = pixels.chunks_mut(self.width as usize * 3).enumerate().collect();

        bands.into_par_iter().for_each(|(i, band)| {
            self.render_line(band, i);
        });

        let _ = write_image("filename", &pixels, (self.height, self.width));

    }

    fn run_singlethreaded(&self){
        let mut img = RgbImage::new(self.width, self.height);

        for y in 0..self.height{
            for x in 0..self.width{
                let point: Complex = self.pixel_to_point(x as i32, y as i32, 0.0);
                let pixel_value: i32 = self.iterations(point);
                let pixel = get_pixel_color(&self.color_pallette, self.max_iterations, pixel_value);
                img.put_pixel(x as u32, y as u32, pixel);
            }
        }
        img.save("rendered_on_1_thread.png").unwrap();
    }

    fn render_animation_frame(&self, frame_index: i32, ){

        let total_zoom_range = self.starting_zoom - self.ending_zoom;
        let zoom_step = total_zoom_range / (self.number_of_frames as f64);
        let zoom_offset = frame_index as f64 * zoom_step;

        let mut img = RgbImage::new(self.width, self.height);

        for y in 0..self.height{
            for x in 0..self.width{
                let point: Complex = self.pixel_to_point(x as i32, y as i32, zoom_offset);
                let pixel_value: i32 = self.iterations(point);
                let pixel = get_pixel_color(&self.color_pallette, self.max_iterations, pixel_value);
                img.put_pixel(x as u32, y as u32, pixel);
            }
        }
        img.save("./animation/".to_string() + &frame_index.to_string()+ ".png").unwrap();
    }

    fn render_row_parallel(&self, img: &Arc<Mutex<RgbImage<>>>, row_index: i32){
        let mut row: Vec<Rgb<u8>> = Vec::new();
        for column_index in 0..self.width{
            let point: Complex = self.pixel_to_point(column_index as i32, row_index as i32, 0.0);
            let pixel_value: i32 = self.iterations(point);
            row.push(get_pixel_color(&self.color_pallette, self.max_iterations, pixel_value));
        }

        let mut imglock = img.lock().unwrap();
        for pixel in 0..row.len(){
            imglock.put_pixel(pixel as u32, row_index as u32, row[pixel])
        }
    }

    fn render_line(&self, pixels: &mut [u8], y: usize){

        for x in 0..self.width{
            let point: Complex = self.pixel_to_point(x as i32, y as i32, 0.0);
            let pixel_value: i32 = self.iterations(point);

            let pixel = get_pixel_color_raw(&self.color_pallette, self.max_iterations, pixel_value);

            pixels[x as usize * 3] = pixel[0];
            pixels[x as usize * 3 + 1] = pixel[1];
            pixels[x as usize * 3 + 2] = pixel[2];
        }
    }

    fn pixel_to_point(&self, pix_x: i32, pix_y: i32, zoom_offset: f64) -> Complex{
        let dx = (self.width / 2) as f64;
        let dy = (self.height / 2) as f64;

        let top_left_rel = (self.center.rel) - (dx * self.starting_zoom);
        let top_left_img = (self.center.img) + (dy * self.starting_zoom);

        let top_left = Complex {rel: top_left_rel, img: top_left_img};

        return Complex { rel: (top_left.rel + (pix_x as f64 * (self.starting_zoom + zoom_offset))), img: (top_left.img - (pix_y as f64 * (self.starting_zoom + zoom_offset))) }
    }

    fn iterations(&self, point: Complex) -> i32{
        let mut z = Complex { rel: (0.0), img: (0.0) };

        for iteration in 0..self.max_iterations {
            z = z.square();
            z = z.add(&point);

            if z.distance_from_origin() >= 4.0{
                return iteration;
            }
        }
        return self.max_iterations;
    }


}

fn get_pixel_color(gradient: &Gradient, max_iteration: i32, iter_count: i32) -> Rgb<u8>{

    let interpolation_value = iter_count as f64 / max_iteration as f64;
    let color = gradient.at(interpolation_value).to_rgba8();

    return Rgb([color[0], color[1], color[2]]);
}

fn get_pixel_color_raw(gradient: &Gradient, max_iteration: i32, iter_count: i32) -> Vec<u8>{

    let interpolation_value = iter_count as f64 / max_iteration as f64;
    let color = gradient.at(interpolation_value).to_rgba8();

    return vec![color[0], color[1], color[2]];
}

fn write_image(
    filename: &str,
    pixels: &[u8],
    bounds: (u32, u32),
) -> Result<(), image::ImageError>{

    let output = File::create(filename)?;
    return image::save_buffer("./output.png", pixels, bounds.1, bounds.0, ColorType::Rgb8);
}

fn main() {

    let frame = MandelbrottZoom {
        width: 1920,
        height: 1080,
        max_iterations: 100000,
        starting_zoom: 4.0 / 1920 as f64,
        ending_zoom: 20.0 / 1920 as f64,
        center: Complex {rel: -1.674409674093473, img: 0.000004716540768697223 },
        dry_run: false,
        color_pallette: colorgrad::rainbow(),
        number_of_frames: 500
    };
    // frame.render_animation();

    frame.render_animation()

}
