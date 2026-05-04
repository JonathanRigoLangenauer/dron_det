mod camera;
use camera::Camera;
use camera::Ray;
use glam::Vec3;
use std::collections::VecDeque;
use std::fs;

pub struct DetectConfig {
    pub analyze_pos: Vec3,
    pub analyze_size: f32,
    pub min_cube_size: f32,
    pub directory: String,
    pub angle: f32,
    pub res_cap_dis: f32,
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
    angel: f32,
    res_cap_dis: f32,
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
            cameras: cameras,
            min_cube_size: config.min_cube_size,
            pos: config.analyze_pos,
            size: config.analyze_size,
            directory: config.directory,
            current_frame: 0,
            frame_delay: 1,
            angel: config.angle,
            res_cap_dis: config.res_cap_dis,
        }
    }

    pub fn find_target(&mut self, current_frame: usize, frame_delay: usize) -> Vec<Res> {
        self.current_frame = current_frame;
        self.frame_delay = frame_delay;
        self.get_new_rays();
        let (res, res_box): (Vec<Res>, Vec<Box>) = self.find_occupied_voxels();

        self.write_out(res_box);
        self.write_res(&res);

        res
    }

    fn write_res(&self, res: &Vec<Res>) {
        let res_csv: String = res
            .iter()
            .map(|r| {
                format!(
                    "{},{},{},{},{},{},{}\n",
                    r.pos.x, r.pos.y, r.pos.z, r.unce.x, r.unce.y, r.unce.z, r.inte
                )
            })
            .collect();

        fs::write(r"C:\Users\jonat\My project (1)\Assets\res.csv", res_csv)
            .expect("failed to write voxels");
    }

    fn write_out(&self, res: Vec<Box>) {
        let voxel_csv: String = res
            .iter()
            .map(|b| format!("{},{},{},{}\n", b.pos.x, b.pos.y, b.pos.z, b.size))
            .collect();

        fs::write(
            r"C:\Users\jonat\My project (1)\Assets\voxels.csv",
            voxel_csv,
        )
        .expect("failed to write voxels");

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

        fs::write(r"C:\Users\jonat\My project (1)\Assets\rays.csv", csv)
            .expect("failed to write file");
    }

    fn get_new_rays(&mut self) {
        self.rays.clear();
        for (i, cam) in &mut self.cameras.iter().enumerate() {
            println!("{}/{}", i, self.cameras.len());
            self.rays.extend(cam.make_vec(0, 1, i as u32));
        }
    }

    fn find_occupied_voxels(&self) -> (Vec<Res>, Vec<Box>) {
        let mut queue: VecDeque<(Vec3, f32, Vec<usize>)> = VecDeque::new();
        queue.push_back((self.pos, self.size, (0..self.rays.len()).collect()));

        let mut results_box: Vec<Box> = Vec::new();
        let mut results: Vec<Res> = Vec::new();

        while let Some((pos, size, indices)) = queue.pop_front() {
            let hit: Vec<usize> = indices
                .iter()
                .copied()
                .filter(|&i| self.rays[i].hit_aabb(pos, size))
                .collect();

            let hit_refs: Vec<&Ray> = hit.iter().map(|&i| &self.rays[i]).collect();
            if !self.has_wide_angle_pair(&hit_refs) {
                continue;
            }

            let new_size = size / 2.0;
            if new_size <= self.min_cube_size {
                let inten = self.intensity(&hit_refs);

                results_box.push(Box {
                    pos: pos,
                    size: size,
                    inten: inten,
                });
                self.add_res(&mut results, pos, inten);
                continue;
            }

            for offset in [
                Vec3::ZERO,
                Vec3::new(new_size, 0.0, 0.0),
                Vec3::new(0.0, new_size, 0.0),
                Vec3::new(0.0, 0.0, new_size),
                Vec3::new(new_size, new_size, 0.0),
                Vec3::new(new_size, 0.0, new_size),
                Vec3::new(0.0, new_size, new_size),
                Vec3::new(new_size, new_size, new_size),
            ] {
                queue.push_back((pos + offset, new_size, hit.clone()));
            }
        }

        results_box.sort_by(|a, b| {
            b.inten
                .partial_cmp(&a.inten)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        results_box.truncate(3000);
        self.res_compress(&mut results);
        (results, results_box)
    }

    fn intensity(&self, cur_rays: &[&Ray]) -> f32 {
        let mut inten: f32 = 0.0;
        for i in 0..cur_rays.len() {
            for j in (i + 1)..cur_rays.len() {
                let angle = cur_rays[i].dir.angle_between(cur_rays[j].dir);
                inten += angle * cur_rays[i].intens * cur_rays[j].intens
            }
        }

        inten
    }

    fn has_wide_angle_pair(&self, cur_rays: &[&Ray]) -> bool {
        for i in 0..cur_rays.len() {
            for j in (i + 1)..cur_rays.len() {
                let angle = cur_rays[i].dir.angle_between(cur_rays[j].dir);
                if cur_rays[i].cam != cur_rays[j].cam && angle > self.angel.to_radians() {
                    return true;
                }
            }
        }
        false
    }

    fn add_res(&self, res: &mut Vec<Res>, pos: Vec3, inte: f32) {
        // Adds the result

        for r in &mut *res {
            if r.pos.distance(pos) < r.unce.length() * 0.1 * r.inte + self.res_cap_dis {
                // find the weighted center of pos and uncertainty
                let ratio = (inte /(r.inte+inte)).abs();
                let mut v = r.pos - pos;
                r.pos -= v * ratio;

                if r.unce.angle_between(v) > 90.0_f32.to_radians() {
                    v = v.reflect(r.unce.normalize())
                }
                let v_unce_diff = r.unce - v;
                r.unce -= v_unce_diff * ratio;

                r.inte += inte;

                return;
            }
        }
        res.push(Res {
            pos: pos,
            unce: Vec3::ZERO,
            inte: inte,
        })
    }

    fn res_compress(&self, res: &mut Vec<Res>) {
        // Adds the result

        let mut remove = vec![];
        for i in 0..res.len() {
            for j in (i + 1)..res.len() {
                let diff = &res[i].pos - &res[j].pos;

                
               
                if !remove.contains(&i) &&(res[i].unce + res[j].unce).length() * 3.0  > diff.length()
                {
                    // find the weighted center of pos and uncertainty

                    let ratio = (res[i].inte /( res[j].inte+res[i].inte)).abs();
                    let mut v = res[j].pos - res[i].pos;
                    res[j].pos -= v * ratio;

                    if res[j].unce.angle_between(v) > 90.0_f32.to_radians() {
                        v = v.reflect(res[j].unce.normalize())
                    }
                    let v_unce_diff = res[j].unce - v;
                    res[j].unce -= v_unce_diff * ratio;

                    res[j].inte += res[i].inte;
                    remove.push(i)
                }
            }
        }
        
        remove.sort_unstable_by(|a, b| b.cmp(a));
        
        for rem in remove {
            res.remove(rem);
        }
    }
}

pub struct Res {
    pos: Vec3,
    unce: Vec3,
    inte: f32,
}

struct Box {
    pos: Vec3,
    size: f32,
    inten: f32,
}
