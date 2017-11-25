mod surf;

use self::surf::Surface;
use ::tobj;
use std::fs;
use std::path::Path;

pub fn simulate(model_obj_path: &str) {
    print!("Loading OBJ at {}... ", model_obj_path);
    let (models, _materials) = tobj::load_obj(&Path::new(model_obj_path)).unwrap();
    println!("Ok");
    
    print!("Generating surface models from meshes... ");
    let surfaces = models.iter()
                         .map(|m| (&m.mesh.positions, &m.mesh.indices))
                         .map(|(pos, idx)| Surface::from_triangles(pos, idx));
    println!("Ok");

    print!("Merging surface models... ");
    let surface = Surface::merge(surfaces);
    println!("Ok");

    let surf_dump_file = format!("{}.surfels.obj", model_obj_path);
    print!("Writing surface model to {}... ", surf_dump_file);
    let mut surf_dump_file = fs::File::create(surf_dump_file).unwrap();
    match surface.dump(&mut surf_dump_file) {
        Ok(_) => println!("Ok"),
        Err(_) => println!("Failed")
    }
}
