use std::collections::VecDeque;
use std::fs;
use glam::Vec3;
mod camera;
use camera::Camera;
use camera::Ray;


fn main() {

    let mut det = Detect::new(DetectConfig{
        min_cube_size: 0.1,
        analyze_pos :  Vec3::new(-50.0, 0.0, -50.0),
        analyze_size: 150.0,
        directory : r"images\Active".to_owned(),
        
    });


    let target = det.find_target(0,1);
    det.write_out(target);
}




pub struct DetectConfig {
    pub analyze_pos: Vec3,
    pub analyze_size: f32,
    pub min_cube_size: f32,
    pub directory: String,
}

struct Detect{
    rays: Vec<Ray>,
    cameras: Vec<Camera>,
    min_cube_size: f32,
    pos: Vec3,
    size: f32,
    directory: String,
    current_frame: usize,
    frame_delay:usize
    
}

impl Detect{

    fn new(config: DetectConfig)-> Self{

    let parent = &config.directory;    
    let folders: Vec<String> = fs::read_dir(parent)
        .expect("Failed to read directory")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_dir())
        .filter_map(|e| e.file_name().into_string().ok())
        .collect();

    let cameras: Vec<Camera> = folders.iter()
        .filter_map(|name| Camera::from_folder_name(parent, name))
        .collect();

    Detect {
            rays: vec![],
            cameras: cameras,
            min_cube_size: config.min_cube_size,
            pos: config.analyze_pos,
            size: config.analyze_size,
            directory: config.directory,
            current_frame:0,
            frame_delay:1,
        }

    }

fn find_target(&mut self,current_frame:usize,frame_delay:usize)->Vec<(Vec3, f32, f32)>{
    self.current_frame= current_frame;
    self.frame_delay = frame_delay;
    self.get_new_rays();
    let res: Vec<(Vec3, f32, f32)>  = self.find_occupied_voxels();
    res
}

fn write_out(&self,res: Vec<(Vec3, f32, f32)>) {
    let voxel_csv: String = res.iter()
        .map(|(p, s,_i)| format!("{},{},{},{}\n", p.x, p.y, p.z, s))
        .collect();

    fs::write(r"C:\Users\jonat\My project (1)\Assets\voxels.csv", voxel_csv).expect("failed to write voxels");

    let csv: String = self.rays.iter()
        .map(|r| format!("{},{},{},{},{},{}\n", r.pos.x, r.pos.y, r.pos.z, r.dir.x, r.dir.y, r.dir.z))
        .collect();

    fs::write(r"C:\Users\jonat\My project (1)\Assets\rays.csv", csv).expect("failed to write file");

}

fn get_new_rays(&mut self){

    self.rays.clear();
    for (i,cam) in &mut self.cameras.iter().enumerate() {
        println!("{}/{}" ,i, self.cameras.len());
        self.rays.extend(cam.make_vec(0, 1,i as u32));
        
    }
}



fn find_occupied_voxels(&self) -> Vec<(Vec3, f32,f32)> {

    let mut queue: VecDeque<(Vec3, f32, Vec<usize>)> = VecDeque::new();
    queue.push_back((self.pos, self.size, (0..self.rays.len()).collect()));
    
    let mut results = Vec::new();

    while let Some((pos, size, indices)) = queue.pop_front() {
        let hit: Vec<usize> = indices.iter().copied()
            .filter(|&i| self.rays[i].hit_aabb(pos,size))
            .collect();

        let hit_refs: Vec<&Ray> = hit.iter().map(|&i| &self.rays[i]).collect();
        if !self.has_wide_angle_pair(&hit_refs) {
            
            continue;

        }

        let new_size = size / 2.0;
        if new_size <= self.min_cube_size {
            let inten = self.intensity(&hit_refs);
            results.push((pos, size,inten));
            continue;
        }

        for offset in [
            Vec3::ZERO,
            Vec3::new(new_size, 0.0,      0.0),
            Vec3::new(0.0,      new_size, 0.0),
            Vec3::new(0.0,      0.0,      new_size),
            Vec3::new(new_size, new_size, 0.0),
            Vec3::new(new_size, 0.0,      new_size),
            Vec3::new(0.0,      new_size, new_size),
            Vec3::new(new_size, new_size, new_size),
        ] {
            queue.push_back((pos + offset, new_size, hit.clone()));
        }
    }

    results.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    results.truncate(3000);
    results
}


fn intensity(&self,cur_rays: &[&Ray]) -> f32 {
    let mut inten: f32= 0.0;
for i in 0..cur_rays.len() {
        for j in (i + 1)..cur_rays.len() {
            let angle = cur_rays[i].dir.angle_between(cur_rays[j].dir);
            inten+= angle* cur_rays[i].intens* cur_rays[j].intens
        }
    }
    

   inten
}

fn has_wide_angle_pair(&self,cur_rays: &[&Ray]) -> bool {
    for i in 0..cur_rays.len() {
        for j in (i + 1)..cur_rays.len() {
            let angle = cur_rays[i].dir.angle_between(cur_rays[j].dir);
            if cur_rays[i].cam != cur_rays[j].cam && angle > 1.0_f32.to_radians() {
                
                return true;
            }
        }
    }
    false
}

 
}