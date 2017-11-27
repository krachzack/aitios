use ::geom::surf::Surface;
use ::geom::scene::Scene;
use std::fs;
use std::path::Path;

pub fn simulate(scene_obj_file_path: &str) {
    print!("Loading OBJ at {}... ", scene_obj_file_path);
    let scene = Scene::load_from_file(scene_obj_file_path);
    println!("Ok");

    print!("Generating surface models from meshes... ");
    let surface = Surface::from_triangles(&scene.positions, &scene.indices);
    println!("Ok");

    let surf_dump_file = format!("{}.surfels.obj", scene_obj_file_path);
    print!("Writing surface model to {}... ", surf_dump_file);
    let mut surf_dump_file = fs::File::create(surf_dump_file).unwrap();
    match surface.dump(&mut surf_dump_file) {
        Ok(_) => println!("Ok"),
        Err(_) => println!("Failed")
    }
}
