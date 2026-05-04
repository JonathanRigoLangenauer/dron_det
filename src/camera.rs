use glam::{EulerRot, Quat, Vec3};
use image::{ImageReader, Rgb, RgbImage};
use std::fs;
use std::path::PathBuf;
use std::f32::consts::PI;

pub struct Camera {
    pub pos: Vec3,
    pub rot: Vec3,
    pub fov: f32,
    pub frames: Vec<PathBuf>,
    pub sensitivity: u8,
    pub ratio: f32,
}

impl Camera {
    pub fn from_folder_name(parent: &str, name: &str) -> Option<Camera> {
        if !name.starts_with("Cam_") {
            return None;
        }

        let mut x: Option<f32> = None;
        let mut y: Option<f32> = None;
        let mut z: Option<f32> = None;
        let mut fov: Option<f32> = None;
        let mut rot_x: Option<f32> = None;
        let mut rot_y: Option<f32> = None;
        let mut rot_z: Option<f32> = None;

        let tokens: Vec<&str> = name.split('_').collect();
        let mut i = 1;
        while i < tokens.len() {
            let token = tokens[i];
            if token.starts_with('X') {
                x = token[1..].parse().ok();
            } else if token.starts_with('Y') {
                y = token[1..].parse().ok();
            } else if token.starts_with('Z') {
                z = token[1..].parse().ok();
            } else if token.starts_with("FOV") {
                fov = token[3..].parse().ok();
            } else if token.starts_with("Rot") {
                rot_x = token[3..].parse().ok();
                if i + 2 < tokens.len() {
                    rot_y = tokens[i + 1].parse().ok();
                    rot_z = tokens[i + 2].parse().ok();
                    i += 2;
                }
            }
            i += 1;
        }

        // Collect and sort all frame_XXXX.png files in the camera folder
        let cam_path = PathBuf::from(parent).join(name);
        let mut frames: Vec<PathBuf> = fs::read_dir(&cam_path)
            .ok()?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .filter(|p| {
                p.file_name()
                    .and_then(|n| n.to_str())
                    .map(|n| n.starts_with("frame_") && n.ends_with(".png"))
                    .unwrap_or(false)
            })
            .collect();

        // Sort by filename so frames are in chronological order
        frames.sort_by(|a, b| a.file_name().cmp(&b.file_name()));

        Some(Camera {
            pos: Vec3 {
                x: x?,
                y: y?,
                z: z?,
            },
            rot: Vec3 {
                x: rot_x?,
                y: rot_y?,
                z: rot_z?,
            },
            fov: fov?,
            frames,
            sensitivity: 2,
            ratio: 16.0 / 9.0,
        })
    }

    pub fn move_diff_image(
        &self,
        current_frame: usize,
        frame_delay: usize,
    ) -> Result<RgbImage, String> {
        let delayed = current_frame + frame_delay;
        if delayed >= self.frames.len() {
            return Err(format!(
                "No images at frame_delay+current_frame {} loaded",
                delayed
            ));
        }

        let img_a = ImageReader::open(&self.frames[current_frame])
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?
            .into_rgb8();

        let img_b = ImageReader::open(&self.frames[delayed])
            .map_err(|e| e.to_string())?
            .decode()
            .map_err(|e| e.to_string())?
            .into_rgb8();

        if img_a.dimensions() != img_b.dimensions() {
            return Err(format!("Image dimensions do not match"));
        }

        let (width, height) = img_a.dimensions();
        let mut result: RgbImage = RgbImage::new(width, height);

        for (x, y, pixel) in result.enumerate_pixels_mut() {
            let Rgb([ar, ag, ab]) = *img_a.get_pixel(x, y);
            let Rgb([br, bg, bb]) = *img_b.get_pixel(x, y);
            let r = ar.abs_diff(br);
            let g = ag.abs_diff(bg);
            let b = ab.abs_diff(bb);
            let gray = ((r as u16 + g as u16 + b as u16) / 3) as u8;
            *pixel = Rgb([gray, gray, gray]);
        }

        let out_path = self.frames[current_frame]
            .parent()
            .map(|p| p.join(format!("diff_{}_{}.png", current_frame, delayed)))
            .ok_or_else(|| "Could not determine output path".to_string())?;
        result.save(&out_path).map_err(|e| e.to_string())?;

        Ok(result)
    }

    pub fn make_vec(&self, current_frame: usize, frame_delay: usize,cam: u32) -> Vec<Ray> { // for each non black pixel contruct a ray;
        let image_diff = self.move_diff_image(current_frame, frame_delay);

        let mut image_diff = match image_diff {
            Ok(val) => val,
            Err(e) => panic!("image_diff not successfully created{}", e),
        };
        // makes a canvas that is 1z from the camera away and then draw a ray to each pixel:
        let y_scale: f32 =  ((self.fov/360.0*2.0*PI)/2.0).tan();
        
        let x_scale: f32 = y_scale * self.ratio;
        let z_dir: f32 = 1.0;
        let height: f32 = image_diff.height() as f32/2.0;
        let width: f32 = image_diff.width()as f32/2.0;
        let mut noise = 0;
        let rotation = Quat::from_euler(
                    EulerRot::YXZ,
                    self.rot.y/360.0*2.0*PI,
                    self.rot.x/360.0*2.0*PI,
                    self.rot.z/360.0*2.0*PI,
                );
        
        let mut ray_in_image: Vec<Ray> = Vec::new();
        for (x, y, pixel) in image_diff.enumerate_pixels_mut() {
            let Rgb([r, _g, _b]) = *pixel;
            if r > self.sensitivity as u8 {
              
                let x_dir = ((x as f32 - width)/width)*x_scale;
                let y_dir = ((-(y as f32 - height)/height))*y_scale;
                // LH Y-up: negate Y and Z angles (glam uses RH convention)
                
                let dir = rotation * Vec3::new(x_dir, y_dir, z_dir);
                noise+=1;
                ray_in_image.push( Ray{ dir:dir, pos: self.pos,intens: r as f32,cam:cam});

            }
        }
          
        
        for  ray in &mut ray_in_image{
            ray.intens=ray.intens/(noise as f32);
          
        }

        ray_in_image
    }
}

#[derive(Debug)]
pub struct Ray {
    pub pos: Vec3,
    pub dir: Vec3,
    pub intens: f32,
    pub cam: u32,
}


impl Ray{

    pub fn hit_aabb(&self, pos: Vec3, size: f32) -> bool {
    let aabb_min = pos;
    let aabb_max = pos + Vec3::splat(size);
    let inv_dir = 1.0 / &self.dir;
    let t1 = (aabb_min - &self.pos) * inv_dir;
    let t2 = (aabb_max - &self.pos) * inv_dir;
    let tmin = t1.min(t2).max_element();
    let tmax = t1.max(t2).min_element();
    tmax >= tmin && tmax >= 0.0

}
}
