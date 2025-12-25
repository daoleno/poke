//! ABI decoder implementation using alloy-dyn-abi

use alloy_dyn_abi::{DynSolType, DynSolValue};
use anyhow::{bail, Context, Result};

use crate::domain::abi::{AbiDecoder, AbiRegistry, DecodedArg, DecodedCall, FunctionSignature};

/// ABI decoder implementation using alloy-dyn-abi
pub struct AlloyAbiDecoder {
    registry: AbiRegistry,
}

impl AlloyAbiDecoder {
    /// Create a new decoder with the given registry
    pub fn new(registry: AbiRegistry) -> Self {
        Self { registry }
    }

    /// Get the underlying registry
    pub fn registry(&self) -> &AbiRegistry {
        &self.registry
    }

    /// Set a new registry
    pub fn set_registry(&mut self, registry: AbiRegistry) {
        self.registry = registry;
    }
}

impl AbiDecoder for AlloyAbiDecoder {
    fn decode_calldata(
        &self,
        function: &FunctionSignature,
        data: &[u8],
    ) -> Result<DecodedCall> {
        if data.len() < 4 {
            bail!("calldata too short (need at least 4 bytes for selector)");
        }

        // Verify selector matches
        let selector: [u8; 4] = data[..4].try_into().unwrap();
        if selector != function.selector {
            bail!(
                "selector mismatch: got 0x{}, expected 0x{}",
                hex::encode(selector),
                hex::encode(function.selector)
            );
        }

        let args_data = &data[4..];

        // Parse types from function inputs
        let types: Vec<DynSolType> = function
            .inputs
            .iter()
            .map(|param| {
                param.kind.parse::<DynSolType>().with_context(|| {
                    format!("Failed to parse type '{}' for param '{}'", param.kind, param.name)
                })
            })
            .collect::<Result<Vec<_>>>()?;

        // Decode the arguments
        let decoded_values = if types.is_empty() {
            Vec::new()
        } else {
            // Create a tuple type for decoding all arguments
            let tuple_type = DynSolType::Tuple(types);
            let decoded = tuple_type
                .abi_decode(args_data)
                .context("Failed to decode calldata")?;

            // Extract individual values from the tuple
            match decoded {
                DynSolValue::Tuple(values) => values,
                other => vec![other],
            }
        };

        // Build decoded arguments
        let arguments: Vec<DecodedArg> = function
            .inputs
            .iter()
            .zip(decoded_values.iter())
            .enumerate()
            .map(|(idx, (param, value))| {
                let name = if param.name.trim().is_empty() {
                    format!("arg{}", idx)
                } else {
                    param.name.clone()
                };

                DecodedArg {
                    name,
                    kind: param.kind.clone(),
                    value: format_dyn_sol_value(value),
                }
            })
            .collect();

        Ok(DecodedCall {
            function_name: function.name.clone(),
            signature: function.signature.clone(),
            arguments,
        })
    }

    fn decode_by_selector(
        &self,
        selector: [u8; 4],
        data: &[u8],
    ) -> Result<Option<DecodedCall>> {
        match self.registry.lookup(selector) {
            Some(function) => {
                let decoded = self.decode_calldata(function, data)?;
                Ok(Some(decoded))
            }
            None => Ok(None),
        }
    }
}

/// Format a DynSolValue for display
fn format_dyn_sol_value(value: &DynSolValue) -> String {
    match value {
        DynSolValue::Bool(b) => b.to_string(),
        DynSolValue::Int(i, _) => i.to_string(),
        DynSolValue::Uint(u, _) => {
            let s = u.to_string();
            // For very large numbers, show hex instead
            if s.len() > 20 {
                format!("0x{:x}", u)
            } else {
                s
            }
        }
        DynSolValue::FixedBytes(word, size) => {
            let bytes = &word.as_slice()[..(*size).min(32)];
            format!("0x{}", hex::encode(bytes))
        }
        DynSolValue::Address(addr) => format!("{:?}", addr),
        DynSolValue::Function(func) => format!("0x{}", hex::encode(func.as_slice())),
        DynSolValue::Bytes(bytes) => {
            if bytes.len() <= 32 {
                format!("0x{}", hex::encode(bytes))
            } else {
                format!("0x{}… ({} bytes)", hex::encode(&bytes[..32]), bytes.len())
            }
        }
        DynSolValue::String(s) => {
            if s.len() <= 64 {
                format!("\"{}\"", s)
            } else {
                format!("\"{}…\" ({} chars)", &s[..64], s.len())
            }
        }
        DynSolValue::Array(arr) | DynSolValue::FixedArray(arr) => {
            let max_items = 10;
            let items: Vec<String> = arr
                .iter()
                .take(max_items)
                .map(format_dyn_sol_value)
                .collect();
            if arr.len() > max_items {
                format!("[{}, …] ({} items)", items.join(", "), arr.len())
            } else {
                format!("[{}]", items.join(", "))
            }
        }
        DynSolValue::Tuple(fields) => {
            let items: Vec<String> = fields.iter().map(format_dyn_sol_value).collect();
            format!("({})", items.join(", "))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::abi::ParamSpec;
    use std::path::PathBuf;

    fn make_transfer_function() -> FunctionSignature {
        FunctionSignature {
            selector: [0xa9, 0x05, 0x9c, 0xbb],
            name: "transfer".to_string(),
            signature: "transfer(address,uint256)".to_string(),
            inputs: vec![
                ParamSpec {
                    name: "to".to_string(),
                    kind: "address".to_string(),
                },
                ParamSpec {
                    name: "amount".to_string(),
                    kind: "uint256".to_string(),
                },
            ],
            source: PathBuf::from("test.json"),
        }
    }

    #[test]
    fn test_decode_transfer() {
        let function = make_transfer_function();

        // transfer(0x1234567890123456789012345678901234567890, 1000)
        let calldata = hex::decode(
            "a9059cbb000000000000000000000000123456789012345678901234567890123456789000000000000000000000000000000000000000000000000000000000000003e8"
        ).unwrap();

        let mut registry = AbiRegistry::new();
        registry.insert(function.clone());
        let decoder = AlloyAbiDecoder::new(registry);

        let result = decoder.decode_calldata(&function, &calldata).unwrap();

        assert_eq!(result.function_name, "transfer");
        assert_eq!(result.arguments.len(), 2);
        assert_eq!(result.arguments[0].name, "to");
        assert!(result.arguments[0].value.contains("1234567890"));
        assert_eq!(result.arguments[1].name, "amount");
        assert_eq!(result.arguments[1].value, "1000");
    }

    #[test]
    fn test_selector_mismatch() {
        let function = make_transfer_function();

        // Wrong selector
        let calldata = hex::decode("deadbeef").unwrap();

        let registry = AbiRegistry::new();
        let decoder = AlloyAbiDecoder::new(registry);

        let result = decoder.decode_calldata(&function, &calldata);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("selector mismatch"));
    }
}
