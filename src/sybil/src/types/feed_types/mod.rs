pub mod get_asset_data;
pub mod get_multiple_asset_data;
pub mod get_xrc_data;
pub mod rate_data;
pub mod read_contract;
pub mod read_logs;

pub mod big_num_serde {
    use std::str::FromStr;

    use candid::Nat;

    pub fn serialize<S: serde::Serializer>(big_num: &Nat, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&big_num.to_string())
    }

    pub fn deserialize<'de, D: serde::Deserializer<'de>>(d: D) -> Result<Nat, D::Error> {
        let s: &str = serde::de::Deserialize::deserialize(d)?;

        Ok(Nat::from_str(s).unwrap())
    }
}

#[cfg(test)]
mod serde_tests {
    use candid::Nat;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    struct TestStruct {
        #[serde(with = "super::big_num_serde")]
        pub big_num: Nat,
    }

    #[test]
    fn test_big_num_serializer() {
        let test_struct = TestStruct {
            big_num: Nat::from(1234567890),
        };
        let serialized = serde_json::to_string(&test_struct).unwrap();

        assert_eq!(serialized, "{\"big_num\":\"1_234_567_890\"}");
    }

    #[test]
    fn test_big_num_deserializer() {
        let deserialized: TestStruct =
            serde_json::from_str("{\"big_num\":\"1_234_567_890\"}").unwrap();

        assert_eq!(deserialized.big_num, Nat::from(1234567890));
    }
}
