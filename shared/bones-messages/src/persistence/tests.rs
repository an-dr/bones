use super::*;

#[test]
fn save_round_trips() {
    let save = Save {
        bytes: b"level=3;hp=42",
    };
    assert_eq!(Save::decode(&save.encode()), Ok(save));
}

#[test]
fn save_round_trips_empty_bytes() {
    let save = Save { bytes: b"" };
    assert_eq!(Save::decode(&save.encode()), Ok(save));
}
