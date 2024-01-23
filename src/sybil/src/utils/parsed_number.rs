use anyhow::Result;

#[derive(Debug)]
/// Used for parsing f64 from strings into integer number with number of decimals
pub struct ParsedNumber {
    pub number: u64,
    pub decimals: u64,
}

impl ParsedNumber {
    pub fn parse(input: &str, supposed_decimals: Option<u64>) -> Result<ParsedNumber> {
        // Split the input into integer and decimal parts
        let mut parts: Vec<&str> = input.split('.').collect();

        // If there is no floating point, add an empty string to the vector in order to avoid errors
        if parts.len() != 2 {
            parts.push("");
        }

        // Number of decimals present in the input
        let actual_decimals = parts[1].len() as u64;

        if let Some(supposed_decimals) = supposed_decimals {
            // Number of decimals that should be added to the number
            let additional_decimals = supposed_decimals.saturating_sub(actual_decimals);

            // Number of decimals that should be removed from the number
            let spare_decimals = actual_decimals.saturating_sub(supposed_decimals);

            // Remove the spare decimals from the number
            let decimal_numbers = parts[1][0..parts[1].len() - spare_decimals as usize].to_string();

            let zeros = "0".repeat(additional_decimals as usize);

            Ok(ParsedNumber {
                number: format!("{}{}{}", parts[0], decimal_numbers, zeros).parse()?,
                decimals: supposed_decimals,
            })
        } else {
            Ok(ParsedNumber {
                number: format!("{}{}", parts[0], parts[1]).parse()?,
                decimals: actual_decimals,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parsed_number_test() {
        let result = ParsedNumber::parse("12345.67", None);
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 1234567);
        assert_eq!(parsed_number.decimals, 2);

        let result = ParsedNumber::parse("12345", None);
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 12345);
        assert_eq!(parsed_number.decimals, 0);

        let result = ParsedNumber::parse("98765.4321", Some(4));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 987654321);
        assert_eq!(parsed_number.decimals, 4);

        let result = ParsedNumber::parse("1.1234", Some(6));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 1123400);
        assert_eq!(parsed_number.decimals, 6);

        let result = ParsedNumber::parse("0.1234", Some(6));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 123400);
        assert_eq!(parsed_number.decimals, 6);

        let result = ParsedNumber::parse("1.1234", Some(2));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 112);
        assert_eq!(parsed_number.decimals, 2);

        let result = ParsedNumber::parse("0.1234", Some(2));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 12);
        assert_eq!(parsed_number.decimals, 2);

        let result = ParsedNumber::parse("0.0", Some(2));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 0);
        assert_eq!(parsed_number.decimals, 2);

        let result = ParsedNumber::parse("1.12", Some(0));
        assert!(result.is_ok());

        let parsed_number = result.unwrap();
        assert_eq!(parsed_number.number, 1);
        assert_eq!(parsed_number.decimals, 0);

        let result = ParsedNumber::parse("invalid_input", None);
        assert!(result.is_err());
    }
}
