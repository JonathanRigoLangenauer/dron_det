mod camera;
use camera::Camera;
use camera::Ray;
use glam::Vec3;
use std::fs;


pub struct DetectConfig {
    pub analyze_pos: Vec3,
    pub analyze_size: f32,
    pub min_cube_size: f32,
    pub directory: String,
    pub min_cameras: u32,
    pub res_cap_dis: f32,
    pub max_cube_number: u32,
    
}

pub struct Detect {
    rays: Vec<Ray>,
    cameras: Vec<Camera>,
    min_cube_size: f32,
    pos: Vec3,
    size: f32,
    directory: String,
    current_frame: usize,
    frame_delay: usize,
    min_cameras: u32,
    result_capture_range: f32,
    max_cube_number: u32,
pub   vox: Vec<Voxel>
}

pub struct Res {
    pos: Vec3,
    radius: f32,
    inten: f32,
}

pub struct Voxel {
    pos: Vec3,
    size: f32,
    inten: f32,
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
            .filter_map(|name| Camera::from_folder_name(parent, name))
            .collect();

        Detect {
            rays: vec![],
            vox:  vec![],
            cameras: cameras,
            min_cube_size: config.min_cube_size,
            pos: config.analyze_pos,
            size: config.analyze_size,
            directory: config.directory,
            current_frame: 0,
            frame_delay: 1,
            min_cameras: config.min_cameras,
            result_capture_range: config.res_cap_dis,
            max_cube_number: config.max_cube_number,
        }
    }

    pub fn find_target(&mut self, current_frame: usize, frame_delay: usize) -> Vec<Res> {
        self.current_frame = current_frame;
        self.frame_delay = frame_delay;
        self.get_new_rays();
        let res: Vec<Res> = self.find_occupied_voxels();


        res
    }

   pub fn out_write_res(&self, res: &Vec<Res>) {
        let res_csv: String = res
            .iter()
            .map(|r| {
                format!(
                    "{},{},{},{},{},{},{}\n",
                    r.pos.x, r.pos.y, r.pos.z, r.radius, r.radius, r.radius, r.inten
                )
            })
            .collect();

        fs::write(r"C:\Users\jonat\My project (1)\Assets\res.csv", res_csv)
            .expect("failed to write voxels");
    }

   pub fn out_box(&self,path: &str) {

        let voxel_csv: String = self.vox
            .iter()
            .map(|b| format!("{},{},{},{}\n", b.pos.x, b.pos.y, b.pos.z, b.size))
            .collect();

        fs::write(
            path,
            voxel_csv,
        )
        .expect("failed to write voxels");

        
    }
    
   pub fn out_rays(&self,path:&str){
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

        fs::write(path, csv)
            .expect("failed to write file");
    }





    fn get_new_rays(&mut self) {
        use rayon::prelude::*;

        let total = self.cameras.len();
        self.rays = self
            .cameras
            .par_iter()
            .enumerate()
            .flat_map(|(i, cam)| {
                println!("{}/{}", i, total);
                cam.make_vec(0, 1, i as u32)
            })
            .collect();
    }
    fn intensity(&self, cur_rays: &[&Ray]) -> f32 {
        let mut cam_num: Vec<u32> = vec![];
        for ray in cur_rays {
            if !cam_num.contains(&ray.cam) {
                cam_num.push(ray.cam);
            }
        }
        cam_num.len() as f32
    }

    fn find_occupied_voxels(&mut self) -> Vec<Res>  {




        use rayon::prelude::*;

        // Seed with the root node
        let mut current_level: Vec<(Vec3, f32, Vec<usize>)> =
            vec![(self.pos, self.size, (0..self.rays.len()).collect())];

        self.vox.clear();
        let levels: u32 = (self.size/self.min_cube_size).log2() as u32;
        let mut level: u32 = 0;
        while !current_level.is_empty() {

            println!("{}/{}",level,levels);
            level+=1;
           
            let outputs: Vec<(Vec<(Vec3, f32, Vec<usize>)>, Vec<Voxel>)> = current_level
                .par_iter()
                .map(|(pos, size, indices)| {
                    let hit: Vec<usize> = indices
                        .iter()
                        .copied()
                        .filter(|&i| self.rays[i].hit_aabb(*pos, *size))
                        .collect();

                    let hit_refs: Vec<&Ray> = hit.iter().map(|&i| &self.rays[i]).collect();

                    if !self.has_wide_angle_pair(&hit_refs) {
                        return (vec![], vec![]);
                    }

                    let new_size = size / 2.0;
                    if new_size <= self.min_cube_size {
                        let inten = self.intensity(&hit_refs);
                       
                            return (
                                vec![],
                                vec![Voxel {
                                    pos: *pos,
                                    size: *size,
                                    inten,
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

            self.vox.extend(outputs.into_iter().flat_map(|(_, leaves)| leaves));
        }

   
        let mut results: Vec<Res> = Vec::new();
        self.vox.sort_by(|a, b| a.inten.total_cmp(&b.inten));
        for b in &self.vox {
            self.add_res(&mut results, b.pos, b.inten);
        }
        results.sort_by(|a, b| a.inten.total_cmp(&b.inten));
        self.res_compress(&mut results);  //Needs more work to be useful

        self.vox.reverse();
        self.vox.truncate(self.max_cube_number as usize);

        results
    }

    fn has_wide_angle_pair(&self, cur_rays: &[&Ray]) -> bool {
        let mut cam_num: Vec<u32> = vec![];
        for ray in cur_rays {
            if !cam_num.contains(&ray.cam) {
                cam_num.push(ray.cam);
            }
        }
        cam_num.len() >= self.min_cameras as usize
    }

    fn add_res(&self, res: &mut Vec<Res>, pos: Vec3, inte: f32) {
        // Adds the result

        for r in &mut *res {
            if (r.pos - pos).length() < r.radius {
                let ratio = (inte / (r.inten + inte)).abs();
                let v = r.pos - pos;
                r.pos -= v * ratio;

                r.radius += self.min_cube_size * 0.1;
                r.inten += inte;

                return;
            }
        }
        res.push(Res {
            pos: pos,
            radius: self.result_capture_range,
            inten: inte,
        })
    }

    fn res_compress(&self, res: &mut Vec<Res>) {
        // Adds the result

        let mut remove = vec![];
        for i in 0..res.len() {
            for j in (i + 1)..res.len() {
                let diff = &res[i].pos - &res[j].pos;

                if !remove.contains(&i) && &res[i].radius + &res[j].radius > diff.length() {
                    let ratio = (res[i].inten / (res[j].inten + res[i].inten)).abs();
                    let mut v = res[j].pos - res[i].pos;
                    res[j].pos -= v * ratio;

                    res[j].radius += res[i].radius;

                    res[j].inten += res[i].inten;
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

