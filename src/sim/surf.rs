
use std::io;
use ::cgmath::Vector3;

/// Represents the surface of a mesh as a point-based model
pub struct Surface {
    points: Vec<Vector3<f32>>
}

impl Surface {
    pub fn from_triangles(positions: &Vec<f32>, indices: &Vec<u32>) -> Surface
    {
        // Collect 3-tuples of Vector3 representing the vertices of each indexed triangle in the mesh
        let triangles = indices.chunks(3)
                               .map(|i|
                                (
                                    Vector3::new(positions[(3*i[0]+0) as usize], positions[(3*i[0]+1) as usize], positions[(3*i[0]+2) as usize]),
                                    Vector3::new(positions[(3*i[1]+0) as usize], positions[(3*i[1]+1) as usize], positions[(3*i[1]+2) as usize]),
                                    Vector3::new(positions[(3*i[2]+0) as usize], positions[(3*i[2]+1) as usize], positions[(3*i[2]+2) as usize])
                                )
                               );

        // Calculate the center point of each triangle to use as a surfel
        let middle_points = triangles.map(|(v0, v1, v2)| (v0 + v1 + v2) / 3.0);

        Surface { points: middle_points.collect() }
    }

    pub fn merge<S>(surfaces: S) -> Surface
    where
        S : IntoIterator<Item = Surface>
    {
        let mut merged_points = Vec::<Vector3<f32>>::new();

        for surf in surfaces {
            merged_points.extend(surf.points);
        }

        Surface { points: merged_points }
    }

    pub fn points(&self) -> &Vec<Vector3<f32>> {
        &self.points
    }

    pub fn dump<S : io::Write>(&self, sink: &mut S) -> io::Result<usize> {
        let mut written : usize = 0;

        written += sink.write("# Surface Model\n".as_bytes())?;
        written += sink.write("# Generated by surf.rs\n\n".as_bytes())?;

        written += sink.write("g surface\n\n".as_bytes())?;

        for &point in self.points.iter() {
            // Write all the points as vertices
            let vertex_line = format!("v {} {} {}\n", point.x, point.y, point.z);
            written += sink.write(vertex_line.as_bytes())?;
        }

        written += sink.write("\n".as_bytes())?;

        // OBJ indices are 1-based, hence +1
        for idx in (0+1)..(self.points.len()+1) {
            // Write points as 1-dimensional faces
            let face_line = format!("f {}\n", idx);
            written += sink.write(face_line.as_bytes())?;
        }

        Ok(written)
    }
}
