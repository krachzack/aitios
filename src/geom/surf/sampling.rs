
use ::geom::tri::Triangle;
use ::geom::vtx::Vertex;


fn sample<I, V>(triangles: I)
    where I : IntoIterator<Item = Triangle<V>>,
        V : Vertex
{
    // 1. make active list of all triangles (logarithmicly binned by area)
    // 2. throw darts
    // 2.1
}
