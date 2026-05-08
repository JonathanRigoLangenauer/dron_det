use glam::Vec3;
mod detect;
use detect::DetectConfig;
use detect::Detect;

fn main() {

let scale =1.0;
    let mut detecting = Detect::new(DetectConfig {
        min_cube_size: scale,
        analyze_pos: Vec3::new(-200.0, 0.0, -200.0),
        analyze_size: 500.0,
        directory: r"images\Active".to_owned(),
        min_cameras: 3,
        res_cap_dis:scale*10.0,
        

        max_cube_number:10
    });


   
    let target = &detecting.find_target(0, 1);

    let voxels = &detecting.vox;

    detecting.out_box(r"C:\Users\jonat\My project (1)\Assets\voxels.csv");
    detecting.out_write_res(&target);
    detecting.out_rays(r"C:\Users\jonat\My project (1)\Assets\rays.csv");


}