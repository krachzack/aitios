
use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;

fn sample<I, V>(triangles: I)
    where I : IntoIterator<Item = Triangle<V>>,
        V : Vertex
{

}
