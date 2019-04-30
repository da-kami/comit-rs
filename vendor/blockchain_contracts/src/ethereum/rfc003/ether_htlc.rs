use crate::{
    ethereum::ByteCode,
    rfc003::{secret_hash::SecretHash, timestamp::Timestamp},
    DataName, Offset,
};
use ethereum_support::{Address, Bytes, U256};
use regex::bytes::Regex;

#[derive(Debug)]
pub struct EtherHtlc {
    refund_timestamp: Timestamp,
    refund_address: Address,
    redeem_address: Address,
    secret_hash: SecretHash,
}

impl EtherHtlc {
    pub const CONTRACT_CODE_TEMPLATE: &'static str =
        include_str!("./templates/out/ether_contract.asm.hex");
    const EXPIRY_PLACEHOLDER: &'static str = "20000002";
    const REDEEM_ADDRESS_PLACEHOLDER: &'static str = "3000000000000000000000000000000000000003";
    const REFUND_ADDRESS_PLACEHOLDER: &'static str = "4000000000000000000000000000000000000004";
    const SECRET_HASH_PLACEHOLDER: &'static str =
        "1000000000000000000000000000000000000000000000000000000000000001";

    const EXPIRY_REGEX: &'static str = r"\x20\x00\x00\x02";
    const REDEEM_ADDRESS_REGEX: &'static str =
        r"\x30\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x03";
    const REFUND_ADDRESS_REGEX: &'static str =
        r"\x40\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x04";
    const SECRET_HASH_REGEX: &'static str =
        r"\x10\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x00\x01";

    const DEPLOY_HEADER_TEMPLATE: &'static str =
        include_str!("./templates/out/deploy_header.asm.hex");
    const CONTRACT_START_POSITION_PLACEHOLDER: &'static str = "1001";
    const CONTRACT_LENGTH_PLACEHOLDER: &'static str = "2002";

    pub fn new(
        refund_timestamp: Timestamp,
        refund_address: Address,
        redeem_address: Address,
        secret_hash: SecretHash,
    ) -> Self {
        Self {
            refund_timestamp,
            refund_address,
            redeem_address,
            secret_hash,
        }
    }

    pub fn deployment_gas_limit(&self) -> U256 {
        let bytes: Bytes = self.compile_to_hex().into();
        let n_bytes = bytes.0.len();
        let gas_per_byte = 200;

        U256::from(75_000 + n_bytes * gas_per_byte)
    }

    pub fn tx_gas_limit() -> U256 {
        U256::from(100_000)
    }

    pub fn compile_template_to_hex() -> String {
        let code_length = Self::CONTRACT_CODE_TEMPLATE.len() / 2; // In hex, each byte is two chars

        let code_length_as_hex = format!("{:0>4x}", code_length);

        let header_length = Self::DEPLOY_HEADER_TEMPLATE.len() / 2;
        let header_length_as_hex = format!("{:0>4x}", header_length);

        let deploy_header = Self::DEPLOY_HEADER_TEMPLATE
            .to_string()
            .replace(
                Self::CONTRACT_START_POSITION_PLACEHOLDER,
                &header_length_as_hex,
            )
            .replace(Self::CONTRACT_LENGTH_PLACEHOLDER, &code_length_as_hex);

        deploy_header + &Self::CONTRACT_CODE_TEMPLATE.to_string()
    }

    fn get_offset(data_name: DataName, regex: &str) -> Offset {
        let contract = hex::decode(Self::compile_template_to_hex())
            .expect("contract is expected to be hex encoded");

        let re_match = Regex::new(regex)
            .expect("Could not create regex")
            .find(&contract)
            .expect("Could not find regex in hex code");
        Offset::new(
            data_name,
            re_match.start(),
            re_match.end(),
            re_match.end() - re_match.start(),
        )
    }

    pub fn get_all_offsets() -> Vec<Offset> {
        let refund_timestamp = Self::get_offset(DataName::Expiry, Self::EXPIRY_REGEX);
        let redeem_address = Self::get_offset(DataName::RedeemIdentity, Self::REDEEM_ADDRESS_REGEX);
        let refund_address = Self::get_offset(DataName::RefundIdentity, Self::REFUND_ADDRESS_REGEX);
        let secret_hash = Self::get_offset(DataName::SecretHash, Self::SECRET_HASH_REGEX);

        vec![
            secret_hash,
            refund_timestamp,
            redeem_address,
            refund_address,
        ]
    }

    pub fn compile_to_hex(&self) -> ByteCode {
        let refund_timestamp = format!("{:0>8x}", u32::from(self.refund_timestamp));
        let redeem_address = format!("{:x}", self.redeem_address);
        let refund_address = format!("{:x}", self.refund_address);
        let secret_hash = format!("{:x}", self.secret_hash);

        let contract_code = Self::compile_template_to_hex()
            .replace(Self::EXPIRY_PLACEHOLDER, &refund_timestamp)
            .replace(Self::REDEEM_ADDRESS_PLACEHOLDER, &redeem_address)
            .replace(Self::REFUND_ADDRESS_PLACEHOLDER, &refund_address)
            .replace(Self::SECRET_HASH_PLACEHOLDER, &secret_hash);

        ByteCode(contract_code)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn compiled_contract_is_same_length_as_template() {
        let htlc = EtherHtlc::new(
            Timestamp::from(3000000),
            Address::new(),
            Address::new(),
            SecretHash::from_str(
                "1000000000000000000000000000000000000000000000000000000000000001",
            )
            .unwrap(),
        );
        let htlc_hex = htlc.compile_to_hex();
        assert_eq!(
            htlc_hex.0.len(),
            EtherHtlc::CONTRACT_CODE_TEMPLATE.len() + EtherHtlc::DEPLOY_HEADER_TEMPLATE.len(),
            "HTLC is the same length as template plus deploy code"
        );
    }

    #[test]
    fn given_input_data_when_compiled_should_no_longer_contain_placeholders() {
        let htlc = EtherHtlc::new(
            Timestamp::from(2000000000),
            Address::default(),
            Address::default(),
            SecretHash::from_str(
                "0000000000000000000000000000000000000000000000000000000000000000",
            )
            .unwrap(),
        );

        let compiled_code = htlc.compile_to_hex().0;

        assert!(!compiled_code.contains(EtherHtlc::EXPIRY_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::REDEEM_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::REFUND_ADDRESS_PLACEHOLDER));
        assert!(!compiled_code.contains(EtherHtlc::SECRET_HASH_PLACEHOLDER));
    }
}
