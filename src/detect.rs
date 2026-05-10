mod camera;
use camera::Camera;
use camera::Ray;
use glam::Vec3;
use rayon::prelude::*;
use std::collections::HashSet;
use std::fs;

pub struct DetectConfig {
    pub analyze_pos: Vec3,
    pub analyze_size: f32,
    pub min_cube_size: f32,
    pub directory: String,
    pub min_cameras: u32,
    pub result_capture_distance: f32,
    pub max_cube_number: u32,
    pub sensitivity: f32,
    pub out_folder: String,
}

pub struct Detect {
    pub rays: Vec<Ray>,
    pub voxel: Vec<Voxel>,
    pub result: Vec<Res>,
    cameras: Vec<Camera>,
    /// the size at which we stop the recursion and consider it a hit.
    min_cube_size: f32,
    /// the minimum amount of different cameras a voxels rays have to have to consider it hit
    min_cameras: u32,
    pos: Vec3,
    size: f32,
    #[allow(dead_code)]
    directory: String,
    current_frame: usize,
    frame_delay: usize,
    /// the minimum radius of a sphere result in which it fuses with other results
    result_capture_range: f32,
    /// how many cubes will be considered as hit
    max_cube_number: u32,
    out_folder: String,
}

pub struct Res {
    pub pos: Vec3,
    pub radius: f32,
    pub num_cameras: f32,
}

pub struct Voxel {
    pub pos: Vec3,
    pub size: f32,
    pub num_cameras: f32,
}

impl Detect {
    pub fn new(config: DetectConfig) -> Self {
        let parent = &config.directory;
        let folders: Vec<String> = fs::read_dir(parent)
            .expect("Failed to read directory")
            .filter_map(|e| e.ok())
            .filter(|e| e.path().is_dir())
            .filter_map(|e| e.file_name().into_string().ok())
            .collect();

        let cameras: Vec<Camera> = folders
            .iter()
            .filter_map(|name| Camera::from_folder_name(parent, name, config.sensitivity))
            .collect();

        Detect {
            rays: vec![],
            voxel: vec![],
            result: vec![],
            cameras: cameras,
            min_cube_size: config.min_cube_size,
            pos: config.analyze_pos,
            size: config.analyze_size,
            directory: config.directory,
            current_frame: 0,
            frame_delay: 1,
            min_cameras: config.min_cameras,
            result_capture_range: config.result_capture_distance,
            max_cube_number: config.max_cube_number,
            out_folder: config.out_folder,
        }
    }

    pub fn find_target(&mut self, current_frame: usize, frame_delay: usize) {
        self.current_frame = current_frame;
        self.frame_delay = frame_delay;
        self.get_new_rays();
        self.result = self.find_occupied_voxels();
    }
    pub fn out(&self) {
        let folder = format!(
            "{}/output_frame_{}_{}",
            &self.out_folder, self.current_frame, self.frame_delay
        );
        std::fs::create_dir_all(&folder).expect("Failed to create output folder");
        self.out_write_res(&folder);
        self.out_box(&folder);
        self.out_rays(&folder);
    }

    pub fn out_write_res(&self, path: &str) {
        let res_csv: String = self
            .result
            .iter()
            .map(|r| {
                format!(
                    "{},{},{},{},{},{},{}\n",
                    r.pos.x, r.pos.y, r.pos.z, r.radius, r.radius, r.radius, r.num_cameras
                )
            })
            .collect();

        fs::write(&format!("{}/res.csv", path), res_csv).expect("failed to write voxels");
    }

    pub fn out_box(&self, path: &str) {
        let voxel_csv: String = self
            .voxel
            .iter()
            .map(|b| format!("{},{},{},{}\n", b.pos.x, b.pos.y, b.pos.z, b.size))
            .collect();

        fs::write(&format!("{}/vox.csv", path), voxel_csv).expect("failed to write voxels");
    }

    pub fn out_rays(&self, path: &str) {
        let csv: String = self
            .rays
            .iter()
            .map(|r| {
                format!(
                    "{},{},{},{},{},{}\n",
                    r.pos.x, r.pos.y, r.pos.z, r.dir.x, r.dir.y, r.dir.z
                )
            })
            .collect();

        fs::write(&format!("{}/rays.csv", path), csv).expect("failed to write file");
    }

    /// collects all the rays from all the cameras
    fn get_new_rays(&mut self) {
        self.rays.clear();

        let total = self.cameras.len();
        self.rays = self
            .cameras
            .par_iter()
            .enumerate()
            .flat_map(|(i, cam)| {
                log::info!("camera {} of {} cameras", i, total);
                cam.make_rays(self.current_frame, self.frame_delay, i as u32)
            })
            .collect();
    }

    fn num_of_cameras(&self, cur_rays: &[&Ray]) -> f32 {
        let cam_num: HashSet<u32> = cur_rays.iter().map(|ray| ray.cam).collect();
        cam_num.len() as f32
    }

    fn find_occupied_voxels(&mut self) -> Vec<Res> {
        use rayon::prelude::*;
        // Seed with the root node
        let mut current_level: Vec<(Vec3, f32, Vec<usize>)> =
            vec![(self.pos, self.size, (0..self.rays.len()).collect())];

        self.voxel.clear();
        let levels: u32 = (self.size / self.min_cube_size).log2() as u32;
        let mut level: u32 = 0;
        while !current_level.is_empty() {
            log::info!("voxel level {} of {}", level, levels);
            level += 1;

            let outputs: Vec<(Vec<(Vec3, f32, Vec<usize>)>, Vec<Voxel>)> = current_level
                .par_iter()
                .map(|(pos, size, indices)| {
                    let hit: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&i| self.rays[i].hit_aabb(*pos, *size))
                        .collect();

                    let hit_refs: Vec<&Ray> = hit.iter().map(|&i| &self.rays[i]).collect();
                
                    if self.num_of_cameras(&hit_refs) < self.min_cameras as f32 {
                        return (vec![], vec![]);
                    }

                    let new_size = size / 2.0;
                    if new_size <= self.min_cube_size {
                        let num_cameras = self.num_of_cameras(&hit_refs);

                        return (
                            vec![],
                            vec![Voxel {
                                pos: *pos,
                                size: *size,
                                num_cameras,
                            }],
                        );
                    }

                    let children = [
                        Vec3::ZERO,
                        Vec3::new(new_size, 0.0, 0.0),
                        Vec3::new(0.0, new_size, 0.0),
                        Vec3::new(0.0, 0.0, new_size),
                        Vec3::new(new_size, new_size, 0.0),
                        Vec3::new(new_size, 0.0, new_size),
                        Vec3::new(0.0, new_size, new_size),
                        Vec3::new(new_size, new_size, new_size),
                    ]
                    .iter()
                    .map(|&offset| (*pos + offset, new_size, hit.clone()))
                    .collect();

                    (children, vec![])
                })
                .collect();

            // Flatten outputs back into the next level and leaf results
            current_level = outputs
                .iter()
                .flat_map(|(children, _)| children.iter().cloned())
                .collect();

            self.voxel
                .extend(outputs.into_iter().flat_map(|(_, leaves)| leaves));
        }

        let mut results: Vec<Res> = Vec::new();
        self.voxel
            .sort_by(|a, b| a.num_cameras.total_cmp(&b.num_cameras));
        for b in &self.voxel {
            self.add_res(&mut results, b.pos, b.num_cameras);
        }
        results.sort_by(|a, b| a.num_cameras.total_cmp(&b.num_cameras));
        self.res_compress(&mut results);

        self.voxel.reverse();
        self.voxel.truncate(self.max_cube_number as usize);

        results
    }

    fn add_res(&self, res: &mut Vec<Res>, pos: Vec3, num_camera: f32) {
        for r in &mut *res {
            if (r.pos - pos).length() < r.radius {
                let ratio = (num_camera / (r.num_cameras + num_camera)).abs();
                let v = r.pos - pos;
                r.pos -= v * ratio;

                r.radius += self.min_cube_size * 0.1;
                r.num_cameras += num_camera;

                return;
            }
        }
        res.push(Res {
            pos: pos,
            radius: self.result_capture_range,
            num_cameras: num_camera,
        })
    }

    pub fn res_compress(&self, res: &mut Vec<Res>) {
        let mut remove = vec![];
        for i in 0..res.len() {
            for j in (i + 1)..res.len() {
                let diff = &res[i].pos - &res[j].pos;

                if !remove.contains(&i)
                    && !remove.contains(&j)
                    && &res[i].radius + &res[j].radius > diff.length()
                {
                    let ratio =
                        (res[i].num_cameras / (res[j].num_cameras + res[i].num_cameras)).abs();
                    let v = res[j].pos - res[i].pos;
                    res[j].pos -= v * ratio;

                    res[j].radius += res[i].radius;

                    res[j].num_cameras += res[i].num_cameras;
                    remove.push(i)
                };
            }
        }

        remove.sort_unstable_by(|a, b| b.cmp(a));
        for rem in remove {
            res.remove(rem);
        }
    }
}
