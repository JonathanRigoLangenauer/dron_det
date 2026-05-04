use glam::Vec3;
mod detect;
use detect::DetectConfig;
use detect::Detect;

fn main() {



    let mut det = Detect::new(DetectConfig {
        min_cube_size: 0.1,
        analyze_pos: Vec3::new(-100.0, 0.0, -100.0),
        analyze_size: 100.0,
        directory: r"images\Active".to_owned(),
        angle: 2.0,
        res_cap_dis:5.0,
    });

    let _target = det.find_target(0, 1);

}