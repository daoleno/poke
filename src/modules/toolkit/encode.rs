//! ABI encode command for encoding function calldata

use alloy::primitives::keccak256;
use alloy_dyn_abi::{DynSolType, DynSolValue};
use super::ToolResult;
use crate::core::{Action, NotifyLevel};

/// Encode function call data from signature and arguments
pub fn encode(input: Option<String>) -> Action {
    let Some(input) = input else {
        return Action::Notify(
            "Usage: :encode <function_sig> <arg1> <arg2> ...".into(),
            NotifyLevel::Warn,
        );
    };

    let parts: Vec<&str> = input.trim().split_whitespace().collect();
    if parts.is_empty() {
        return Action::Notify(
            "Usage: :encode transfer(address,uint256) 0x123... 1000".into(),
            NotifyLevel::Warn,
        );
    }

    let signature = parts[0];
    let args = &parts[1..];

    match encode_calldata(signature, args) {
        Ok(calldata) => {
            let result = ToolResult::new("ABI Encode")
                .add("signature", signature)
                .add("calldata", format!("0x{}", hex::encode(&calldata)));
            result.into_action()
        }
        Err(e) => Action::Notify(format!("Encoding error: {}", e), NotifyLevel::Error),
    }
}

/// Parse function signature and encode calldata
fn encode_calldata(signature: &str, args: &[&str]) -> Result<Vec<u8>, String> {
    // Parse function signature to extract name and parameter types
    let (param_types, normalized_sig) = parse_function_signature(signature)?;

    // Check argument count matches parameter count
    if args.len() != param_types.len() {
        return Err(format!(
            "Argument count mismatch: expected {} arguments, got {}",
            param_types.len(),
            args.len()
        ));
    }

    // Compute function selector (first 4 bytes of keccak256(signature))
    let hash = keccak256(normalized_sig.as_bytes());
    let selector = &hash[..4];

    // Parse and encode arguments
    let mut calldata = selector.to_vec();

    if !param_types.is_empty() {
        let values = parse_arguments(&param_types, args)?;
        // Wrap values in a tuple for proper encoding
        let tuple_value = DynSolValue::Tuple(values);
        let encoded = tuple_value.abi_encode_params();
        calldata.extend_from_slice(&encoded);
    }

    Ok(calldata)
}

/// Parse function signature to extract parameter types
fn parse_function_signature(signature: &str) -> Result<(Vec<DynSolType>, String), String> {
    // Normalize: remove spaces
    let normalized = signature.replace(" ", "");

    // Find opening and closing parentheses
    let open_paren = normalized
        .find('(')
        .ok_or_else(|| "Invalid function signature: missing '('".to_string())?;
    let close_paren = normalized
        .rfind(')')
        .ok_or_else(|| "Invalid function signature: missing ')'".to_string())?;

    if close_paren <= open_paren {
        return Err("Invalid function signature: malformed parentheses".to_string());
    }

    // Extract parameter types string
    let params_str = &normalized[open_paren + 1..close_paren];

    // Parse parameter types
    let param_types = if params_str.is_empty() {
        Vec::new()
    } else {
        let type_strings: Vec<&str> = params_str.split(',').collect();
        let mut types = Vec::new();

        for type_str in type_strings {
            let ty = DynSolType::parse(type_str)
                .map_err(|e| format!("Failed to parse type '{}': {}", type_str, e))?;
            types.push(ty);
        }

        types
    };

    Ok((param_types, normalized))
}

/// Parse argument values according to their types
fn parse_arguments(types: &[DynSolType], args: &[&str]) -> Result<Vec<DynSolValue>, String> {
    let mut values = Vec::new();

    for (i, (ty, arg)) in types.iter().zip(args.iter()).enumerate() {
        let value = parse_value(ty, arg).map_err(|e| {
            format!("Failed to parse argument {} (type {:?}): {}", i + 1, ty, e)
        })?;
        values.push(value);
    }

    Ok(values)
}

/// Parse a single value according to its type
fn parse_value(ty: &DynSolType, arg: &str) -> Result<DynSolValue, String> {
    match ty {
        DynSolType::Address => {
            // Parse address
            let addr = arg.trim().to_lowercase();
            let addr = addr.strip_prefix("0x").unwrap_or(&addr);

            if addr.len() != 40 || !addr.chars().all(|c| c.is_ascii_hexdigit()) {
                return Err("Invalid address: expected 40 hex characters".to_string());
            }

            let bytes = hex::decode(addr).map_err(|e| format!("Invalid hex: {}", e))?;
            let mut array = [0u8; 20];
            array.copy_from_slice(&bytes);

            Ok(DynSolValue::Address(alloy::primitives::Address::from(array)))
        }

        DynSolType::Bool => {
            // Parse boolean
            let value = match arg.to_lowercase().as_str() {
                "true" | "1" => true,
                "false" | "0" => false,
                _ => return Err(format!("Invalid bool: expected true/false, got '{}'", arg)),
            };
            Ok(DynSolValue::Bool(value))
        }

        DynSolType::Int(size) => {
            // Parse signed integer
            let value = if arg.starts_with("0x") || arg.starts_with("0X") {
                // Hex format - parse as U256 then convert
                let hex_str = &arg[2..];
                let bytes = parse_hex_to_bytes(hex_str, 32)?;
                let mut array = [0u8; 32];
                array.copy_from_slice(&bytes);
                alloy::primitives::I256::from_be_bytes(array)
            } else {
                // Decimal format
                arg.parse::<alloy::primitives::I256>()
                    .map_err(|e| format!("Invalid integer: {}", e))?
            };
            Ok(DynSolValue::Int(value, *size))
        }

        DynSolType::Uint(size) => {
            // Parse unsigned integer
            let value = if arg.starts_with("0x") || arg.starts_with("0X") {
                // Hex format
                let hex_str = &arg[2..];
                let bytes = parse_hex_to_bytes(hex_str, 32)?;
                let mut array = [0u8; 32];
                array.copy_from_slice(&bytes);
                alloy::primitives::U256::from_be_bytes(array)
            } else {
                // Decimal format
                arg.parse::<alloy::primitives::U256>()
                    .map_err(|e| format!("Invalid unsigned integer: {}", e))?
            };
            Ok(DynSolValue::Uint(value, *size))
        }

        DynSolType::Bytes => {
            // Parse dynamic bytes
            let hex_str = arg.strip_prefix("0x").unwrap_or(arg);
            let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;
            Ok(DynSolValue::Bytes(bytes))
        }

        DynSolType::FixedBytes(size) => {
            // Parse fixed bytes
            let hex_str = arg.strip_prefix("0x").unwrap_or(arg);
            let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;

            if bytes.len() != *size {
                return Err(format!(
                    "Invalid bytes length: expected {} bytes, got {}",
                    size,
                    bytes.len()
                ));
            }

            Ok(DynSolValue::FixedBytes(
                alloy::primitives::FixedBytes::from_slice(&bytes),
                *size,
            ))
        }

        DynSolType::String => {
            // Parse string (remove quotes if present)
            let s = if (arg.starts_with('"') && arg.ends_with('"'))
                || (arg.starts_with('\'') && arg.ends_with('\''))
            {
                &arg[1..arg.len() - 1]
            } else {
                arg
            };
            Ok(DynSolValue::String(s.to_string()))
        }

        DynSolType::Array(inner_ty) => {
            // Parse array - expect format: [val1,val2,val3]
            let array_str = arg.trim();
            if !array_str.starts_with('[') || !array_str.ends_with(']') {
                return Err("Array must be enclosed in brackets: [val1,val2,...]".to_string());
            }

            let inner_str = &array_str[1..array_str.len() - 1];
            if inner_str.is_empty() {
                return Ok(DynSolValue::Array(Vec::new()));
            }

            let elements: Vec<&str> = inner_str.split(',').map(|s| s.trim()).collect();
            let mut values = Vec::new();

            for elem in elements {
                values.push(parse_value(inner_ty, elem)?);
            }

            Ok(DynSolValue::Array(values))
        }

        DynSolType::FixedArray(inner_ty, size) => {
            // Parse fixed array - expect format: [val1,val2,val3]
            let array_str = arg.trim();
            if !array_str.starts_with('[') || !array_str.ends_with(']') {
                return Err("Array must be enclosed in brackets: [val1,val2,...]".to_string());
            }

            let inner_str = &array_str[1..array_str.len() - 1];
            let elements: Vec<&str> = inner_str.split(',').map(|s| s.trim()).collect();

            if elements.len() != *size {
                return Err(format!(
                    "Fixed array size mismatch: expected {} elements, got {}",
                    size,
                    elements.len()
                ));
            }

            let mut values = Vec::new();
            for elem in elements {
                values.push(parse_value(inner_ty, elem)?);
            }

            Ok(DynSolValue::FixedArray(values))
        }

        DynSolType::Tuple(types) => {
            // Parse tuple - expect format: (val1,val2,val3)
            let tuple_str = arg.trim();
            if !tuple_str.starts_with('(') || !tuple_str.ends_with(')') {
                return Err("Tuple must be enclosed in parentheses: (val1,val2,...)".to_string());
            }

            let inner_str = &tuple_str[1..tuple_str.len() - 1];
            let elements: Vec<&str> = inner_str.split(',').map(|s| s.trim()).collect();

            if elements.len() != types.len() {
                return Err(format!(
                    "Tuple size mismatch: expected {} elements, got {}",
                    types.len(),
                    elements.len()
                ));
            }

            let mut values = Vec::new();
            for (ty, elem) in types.iter().zip(elements.iter()) {
                values.push(parse_value(ty, elem)?);
            }

            Ok(DynSolValue::Tuple(values))
        }

        _ => Err(format!("Unsupported type: {:?}", ty)),
    }
}

/// Parse hex string to bytes with padding
fn parse_hex_to_bytes(hex_str: &str, expected_size: usize) -> Result<Vec<u8>, String> {
    let bytes = hex::decode(hex_str).map_err(|e| format!("Invalid hex: {}", e))?;

    if bytes.len() > expected_size {
        return Err(format!(
            "Hex value too large: expected max {} bytes, got {}",
            expected_size,
            bytes.len()
        ));
    }

    // Pad with zeros on the left
    let mut padded = vec![0u8; expected_size];
    padded[expected_size - bytes.len()..].copy_from_slice(&bytes);

    Ok(padded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_transfer() {
        // transfer(address,uint256)
        let sig = "transfer(address,uint256)";
        let args = vec!["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0", "1000000"];

        let result = encode_calldata(sig, &args).unwrap();
        let hex_result = hex::encode(&result);

        // Selector: a9059cbb
        assert!(hex_result.starts_with("a9059cbb"));
    }

    #[test]
    fn test_encode_no_args() {
        // balanceOf() - though typically has args, testing no-arg case
        let sig = "totalSupply()";
        let args = vec![];

        let result = encode_calldata(sig, &args).unwrap();
        // Should be just 4 bytes (selector)
        assert_eq!(result.len(), 4);
    }

    #[test]
    fn test_parse_bool() {
        let ty = DynSolType::Bool;
        assert!(matches!(parse_value(&ty, "true"), Ok(DynSolValue::Bool(true))));
        assert!(matches!(parse_value(&ty, "false"), Ok(DynSolValue::Bool(false))));
        assert!(matches!(parse_value(&ty, "1"), Ok(DynSolValue::Bool(true))));
        assert!(matches!(parse_value(&ty, "0"), Ok(DynSolValue::Bool(false))));
    }

    #[test]
    fn test_parse_string() {
        let ty = DynSolType::String;
        assert!(matches!(
            parse_value(&ty, "\"hello\""),
            Ok(DynSolValue::String(s)) if s == "hello"
        ));
        assert!(matches!(
            parse_value(&ty, "world"),
            Ok(DynSolValue::String(s)) if s == "world"
        ));
    }

    #[test]
    fn test_parse_array() {
        let ty = DynSolType::Array(Box::new(DynSolType::Uint(256)));
        let result = parse_value(&ty, "[1,2,3]");
        assert!(result.is_ok());
    }

    #[test]
    fn test_argument_count_mismatch() {
        let sig = "transfer(address,uint256)";
        let args = vec!["0x742d35Cc6634C0532925a3b844Bc9e7595f0bEb0"];

        let result = encode_calldata(sig, &args);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Argument count mismatch"));
    }
}
