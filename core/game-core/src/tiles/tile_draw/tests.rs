use super::*;

/// A minimal valid TMX map with one embedded 2x2 tileset (firstgid 1) and
/// a 2x2 "Ground" layer: (0,0)=gid 1, (1,0)=gid 2, (0,1)=empty, (1,1)=gid 4.
fn fixture_tmx() -> &'static [u8] {
    br#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="2" height="2" tilewidth="16" tileheight="16" infinite="0" nextlayerid="2" nextobjectid="1">
 <tileset firstgid="1" name="grass" tilewidth="16" tileheight="16" tilecount="4" columns="2">
  <image source="grass.png" width="32" height="32"/>
 </tileset>
 <layer id="1" name="Ground" width="2" height="2">
  <data encoding="csv">
1,2,
0,4
</data>
 </layer>
</map>
"#
}

#[test]
fn a_ground_layer_becomes_tile_draws_at_their_grid_position() {
    let draws = load_tile_draws(fixture_tmx(), |name| (name == "grass").then_some(7)).unwrap();

    assert_eq!(draws.len(), 3, "the empty (0,1) cell must not draw, got {draws:?}");
    assert!(draws.contains(&TileDraw {
        sprite_id: 7,
        dst_x: 0,
        dst_y: 0,
        size: 16,
        src_x: 0,
        src_y: 0,
    }));
    assert!(draws.contains(&TileDraw {
        sprite_id: 7,
        dst_x: 16,
        dst_y: 0,
        size: 16,
        src_x: 16,
        src_y: 0,
    }));
    assert!(draws.contains(&TileDraw {
        sprite_id: 7,
        dst_x: 16,
        dst_y: 16,
        size: 16,
        src_x: 16,
        src_y: 16,
    }));
}

#[test]
fn a_tileset_the_resolver_has_no_sprite_for_is_skipped_not_an_error() {
    let draws = load_tile_draws(fixture_tmx(), |_name| None).unwrap();
    assert!(draws.is_empty());
}

#[test]
fn a_map_with_no_ground_layer_yields_no_draws() {
    let tmx = br#"<?xml version="1.0" encoding="UTF-8"?>
<map version="1.10" tiledversion="1.10.2" orientation="orthogonal" renderorder="right-down" width="1" height="1" tilewidth="16" tileheight="16" infinite="0" nextlayerid="2" nextobjectid="1">
 <layer id="1" name="NotGround" width="1" height="1">
  <data encoding="csv">
0
</data>
 </layer>
</map>
"#;

    let draws = load_tile_draws(tmx, |_name| Some(1)).unwrap();
    assert!(draws.is_empty());
}

#[test]
fn malformed_xml_is_a_structured_error() {
    assert!(load_tile_draws(b"not xml at all", |_name| Some(1)).is_err());
}
