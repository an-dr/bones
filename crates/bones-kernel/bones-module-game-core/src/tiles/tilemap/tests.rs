use super::*;

/// A minimal valid TMX map — one tile layer (required by the format) and
/// one "Collision" object layer with two rectangles — small enough to
/// inline, real enough to exercise the actual `tiled` XML parse path.
fn fixture_tmx() -> &'static [u8] {
    br#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="4" height="4" tilewidth="16" tileheight="16" infinite="0" nextlayerid="3" nextobjectid="3">
 <layer id="1" name="Ground" width="4" height="4">
  <data encoding="csv">
0,0,0,0,
0,0,0,0,
0,0,0,0,
0,0,0,0
</data>
 </layer>
 <objectgroup id="2" name="Collision">
  <object id="1" x="16" y="16" width="16" height="16"/>
  <object id="2" x="48" y="32" width="8" height="8"/>
 </objectgroup>
</map>
"#
}

#[test]
fn a_collision_layer_becomes_collision_rects() {
    let rects = load_collision_rects(fixture_tmx()).unwrap();

    assert_eq!(rects.len(), 2);
    assert_eq!(
        rects[0],
        CollisionRect {
            x: 24.0,
            y: 24.0,
            half_w: 8.0,
            half_h: 8.0
        }
    );
    assert_eq!(
        rects[1],
        CollisionRect {
            x: 52.0,
            y: 36.0,
            half_w: 4.0,
            half_h: 4.0
        }
    );
}

#[test]
fn a_map_with_no_collision_layer_yields_no_rects() {
    let tmx = br#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="16" tileheight="16" infinite="0" nextlayerid="2" nextobjectid="1">
 <layer id="1" name="Ground" width="1" height="1">
  <data encoding="csv">
0
</data>
 </layer>
</map>
"#;

    let rects = load_collision_rects(tmx).unwrap();
    assert!(rects.is_empty());
}

#[test]
fn malformed_xml_is_a_structured_error() {
    assert!(load_collision_rects(b"not xml at all").is_err());
}
