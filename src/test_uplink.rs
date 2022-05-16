use super::*;

#[test]
fn uplink_partial_eq() {
    let uplink0 = Uplink {
        temperature: Some(22.1),
        co2: Some(9876),
        battery_mv: Some(3809),
        occupancy: Some(Occupancy::OccupiedOrHeat),
        external_digital: Some(false),
    };

    let uplink1 = uplink0.clone();

    assert_eq!(uplink0, uplink1);
}

#[test]
fn test_close() {
    assert!(close(Some(1.0), Some(1.0), 1.0));
    assert!(close(Some(9.8), Some(9.61), 0.4));
    assert!(!close(Some(9.8), Some(9.59), 0.4));
}

#[test]
fn deserialize_00() {
    let expected_output = Uplink {
        occupancy: Some(Occupancy::PendingOrPir),
        ..Uplink::default()
    };

    assert_eq!(
        expected_output,
        Uplink::deserialize(&base64::decode("BQERAQ==").unwrap()).unwrap()
    )
}

#[test]
fn deserialize_01() {
    let expected_output = Uplink {
        temperature: Some(22.0),
        battery_mv: Some(3649),
        ..Uplink::default()
    };

    assert_eq!(
        expected_output,
        Uplink::deserialize(&base64::decode("AQDcAjwHDkE=").unwrap()).unwrap()
    )
}

#[test]
fn deserialize_02() {
    let expected_output = Uplink {
        temperature: Some(24.9),
        battery_mv: Some(3658),
        ..Uplink::default()
    };

    assert_eq!(
        expected_output,
        Uplink::deserialize(&base64::decode("AQD5AjYEAk8FAgcOSg==").unwrap()).unwrap()
    )
}

#[test]
fn deserialize_03() {
    let expected_output = Uplink {
        temperature: Some(21.2),
        battery_mv: Some(3613),
        occupancy: Some(Occupancy::PendingOrPir),
        ..Uplink::default()
    };

    assert_eq!(
        expected_output,
        Uplink::deserialize(&base64::decode("AQDUAigEABQFAAcOHREB").unwrap()).unwrap()
    )
}

#[test]
fn deserialize_no_identifier() {
    assert!(Uplink::deserialize(&[0x20, 0x00, 0x00]).is_err());
}

#[test]
fn deserialize_too_short() {
    assert!(Uplink::deserialize(&[0x06, 0x00]).is_err());
    assert!(Uplink::deserialize(&[0x06, 0x00, 0x00]).is_ok());
}
