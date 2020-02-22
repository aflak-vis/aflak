extern crate obj_exporter as obj;

use obj::{Geometry, ObjSet, Object, Primitive, Shape, Vertex};

pub fn main() {
  let set = ObjSet {
    material_library: None,
    objects: vec![
      Object {
        name: "Square1".to_owned(),
        vertices: vec![
          (-1.0, -1.0, 0.0),
          (1.0, -1.0, 0.0),
          (1.0, 1.0, 0.0),
          (-1.0, 1.0, 0.0),
        ].into_iter()
          .map(|(x, y, z)| Vertex { x, y, z })
          .collect(),
        tex_vertices: vec![],
        normals: vec![],
        geometry: vec![
          Geometry {
            material_name: None,
            shapes: vec![(0, 1, 2), (0, 2, 3)]
              .into_iter()
              .map(|(x, y, z)| Shape {
                primitive: Primitive::Triangle((x, None, None), (y, None, None), (z, None, None)),
                groups: vec![],
                smoothing_groups: vec![],
              })
              .collect(),
          },
        ],
      },
      Object {
        name: "Square2".to_owned(),
        vertices: vec![
          (1.0, -1.0, 0.0),
          (2.0, -1.0, 0.0),
          (2.0, 1.0, 0.0),
          (1.0, 1.0, 0.0),
        ].into_iter()
          .map(|(x, y, z)| Vertex { x, y, z })
          .collect(),
        tex_vertices: vec![],
        normals: vec![],
        geometry: vec![
          Geometry {
            material_name: None,
            shapes: vec![(0, 1, 2), (0, 2, 3)]
              .into_iter()
              .map(|(x, y, z)| Shape {
                primitive: Primitive::Triangle((x, None, None), (y, None, None), (z, None, None)),
                groups: vec![],
                smoothing_groups: vec![],
              })
              .collect(),
          },
        ],
      },
    ],
  };

  obj::export_to_file(&set, "output_multiple.obj").unwrap();
}
