extern crate obj_exporter as obj;

use obj::{Geometry, ObjSet, Object, Primitive, Shape, TVertex, Vertex};

#[test]
pub fn test_square() {
  let set = ObjSet {
    material_library: None,
    objects: vec![
      Object {
        name: "Square".to_owned(),
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
    ],
  };

  let expected = r#"o Square
v -1.000000 -1.000000 0.000000
v 1.000000 -1.000000 0.000000
v 1.000000 1.000000 0.000000
v -1.000000 1.000000 0.000000
f 1 2 3
f 1 3 4
"#;
  let mut output = Vec::<u8>::new();
  obj::export(&set, &mut output).unwrap();
  assert_eq!(String::from_utf8(output).unwrap(), expected);
}

#[test]
pub fn test_square_with_uv() {
  let set = ObjSet {
    material_library: None,
    objects: vec![
      Object {
        name: "Square".to_owned(),
        vertices: vec![
          (-1.0, -1.0, 0.0),
          (1.0, -1.0, 0.0),
          (1.0, 1.0, 0.0),
          (-1.0, 1.0, 0.0),
        ].into_iter()
          .map(|(x, y, z)| Vertex { x, y, z })
          .collect(),
        tex_vertices: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
          .into_iter()
          .map(|(u, v)| TVertex { u, v, w: 0.0 })
          .collect(),
        normals: vec![],
        geometry: vec![
          Geometry {
            material_name: None,
            shapes: vec![(0, 1, 2), (0, 2, 3)]
              .into_iter()
              .map(|(x, y, z)| Shape {
                primitive: Primitive::Triangle(
                  (x, Some(x), None),
                  (y, Some(y), None),
                  (z, Some(z), None),
                ),
                groups: vec![],
                smoothing_groups: vec![],
              })
              .collect(),
          },
        ],
      },
    ],
  };

  let expected = r#"o Square
v -1.000000 -1.000000 0.000000
v 1.000000 -1.000000 0.000000
v 1.000000 1.000000 0.000000
v -1.000000 1.000000 0.000000
vt 0.000000 0.000000
vt 1.000000 0.000000
vt 1.000000 1.000000
vt 0.000000 1.000000
f 1/1 2/2 3/3
f 1/1 3/3 4/4
"#;

  let mut output = Vec::<u8>::new();
  obj::export(&set, &mut output).unwrap();
  assert_eq!(String::from_utf8(output).unwrap(), expected);
}

#[test]
pub fn test_square_with_uv_and_normals() {
  let set = ObjSet {
    material_library: None,
    objects: vec![
      Object {
        name: "Square".to_owned(),
        vertices: vec![
          (-1.0, -1.0, 0.0),
          (1.0, -1.0, 0.0),
          (1.0, 1.0, 0.0),
          (-1.0, 1.0, 0.0),
        ].into_iter()
          .map(|(x, y, z)| Vertex { x, y, z })
          .collect(),
        tex_vertices: vec![(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)]
          .into_iter()
          .map(|(u, v)| TVertex { u, v, w: 0.0 })
          .collect(),
        normals: vec![
          Vertex {
            x: 0.0,
            y: 0.0,
            z: -1.0,
          },
        ],
        geometry: vec![
          Geometry {
            material_name: None,
            shapes: vec![(0, 1, 2), (0, 2, 3)]
              .into_iter()
              .map(|(x, y, z)| Shape {
                primitive: Primitive::Triangle(
                  (x, Some(x), Some(0)),
                  (y, Some(y), Some(0)),
                  (z, Some(z), Some(0)),
                ),
                groups: vec![],
                smoothing_groups: vec![],
              })
              .collect(),
          },
        ],
      },
    ],
  };

  let expected = r#"o Square
v -1.000000 -1.000000 0.000000
v 1.000000 -1.000000 0.000000
v 1.000000 1.000000 0.000000
v -1.000000 1.000000 0.000000
vt 0.000000 0.000000
vt 1.000000 0.000000
vt 1.000000 1.000000
vt 0.000000 1.000000
vn 0.000000 0.000000 -1.000000
f 1/1/1 2/2/1 3/3/1
f 1/1/1 3/3/1 4/4/1
"#;

  let mut output = Vec::<u8>::new();
  obj::export(&set, &mut output).unwrap();
  assert_eq!(String::from_utf8(output).unwrap(), expected);
}

#[test]
pub fn test_multiple_objects() {
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

  let expected = r#"o Square1
v -1.000000 -1.000000 0.000000
v 1.000000 -1.000000 0.000000
v 1.000000 1.000000 0.000000
v -1.000000 1.000000 0.000000
f 1 2 3
f 1 3 4
o Square2
v 1.000000 -1.000000 0.000000
v 2.000000 -1.000000 0.000000
v 2.000000 1.000000 0.000000
v 1.000000 1.000000 0.000000
f 5 6 7
f 5 7 8
"#;

  let mut output = Vec::<u8>::new();
  obj::export(&set, &mut output).unwrap();
  assert_eq!(String::from_utf8(output).unwrap(), expected);
}

fn create_geometry(groups: &[String], smoothing_groups: &[u32]) -> obj::Geometry {
  Geometry {
    material_name: None,
    shapes: vec![(0, 1, 2), (0, 2, 3)]
      .into_iter()
      .map(|(x, y, z)| Shape {
        primitive: Primitive::Triangle((x, None, None), (y, None, None), (z, None, None)),
        groups: groups.to_owned(),
        smoothing_groups: smoothing_groups.to_owned(),
      })
      .collect(),
  }
}

#[test]
pub fn test_squares_grouped() {
  let set = ObjSet {
    material_library: None,
    objects: vec![
      Object {
        name: "Square".to_owned(),
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
          // default group
          create_geometry(&[], &[]),
          // custom group
          create_geometry(&["group_1".to_owned()], &[1]),
          // default group again
          create_geometry(&["".to_owned()], &[]),
          // two groups
          create_geometry(&["group_1".to_owned(), "group_2".to_owned()], &[1, 2]),
          // The same groups as previous
          create_geometry(&["group_1".to_owned(), "group_2".to_owned()], &[1, 2]),
        ],
      },
    ],
  };

  let expected = r#"o Square
v -1.000000 -1.000000 0.000000
v 1.000000 -1.000000 0.000000
v 1.000000 1.000000 0.000000
v -1.000000 1.000000 0.000000
f 1 2 3
f 1 3 4
g group_1
s 1
f 1 2 3
f 1 3 4
g default
s 0
f 1 2 3
f 1 3 4
g group_1 group_2
s 1 2
f 1 2 3
f 1 3 4
f 1 2 3
f 1 3 4
"#;
  let mut output = Vec::<u8>::new();
  obj::export(&set, &mut output).unwrap();
  assert_eq!(String::from_utf8(output).unwrap(), expected);
}
