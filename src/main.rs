use glam::Vec3;
mod detect;
use detect::Detect;
use detect::DetectConfig;

fn main() {
    env_logger::init();
    let scale = 1.0;
    let mut detecting = Detect::new(DetectConfig {
        min_cube_size: scale,
        analyze_pos: Vec3::new(-200.0, 0.0, -200.0),
        analyze_size: 500.0,
        directory: r"images\Active".to_owned(),
        min_cameras: 3,
        result_capture_distance: scale * 10.0,
        sensitivity: 0.001,
        max_cube_number: 10,
        out_folder: r"output".to_owned(),
    });

    detecting.find_target(0, 1);
    detecting.out()
    
}



#[test]
fn test() {
    let scale = 1.0;
    let mut detecting = Detect::new(DetectConfig {
        min_cube_size: scale,
        analyze_pos: Vec3::new(-200.0, 0.0, -200.0),
        analyze_size: 500.0,
        directory: r"images/test".to_owned(),
        min_cameras: 3,
        result_capture_distance: scale * 10.0,
        max_cube_number: 10,
        sensitivity: 0.001,
        out_folder: r"out_test".to_owned(),
    });

    detecting.find_target(0, 1);
    detecting.out();
    let target = &detecting.result;

    for vox in &detecting.voxel {
        assert!(
            (vox.pos - Vec3::new(70.0, 100.0, 0.0)).length() < 15.0,
            "voxel at {} not at the real target position {}",
            target[0].pos,
            Vec3::new(70.0, 100.0, 0.0)
        );
    }


    let _ = Detect::res_compress; // Ensures that res_compress ins't outdated;
    let res_compress = stringify!(self.res_compress);
    assert_eq!(
        target.len(),
        1,
        "{} should reduce target to 1 element but got {}",
        res_compress,
        target.len()
    );
    assert!(
        (target[0].pos - Vec3::new(70.0, 100.0, 0.0)).length() < 10.0,
        "calculated target position {} not at the real target position {}",
        target[0].pos,
        Vec3::new(70.0, 100.0, 0.0)
    );
}
