const INTERNAL_ERR: &'static str = "`ethabi_derive` internal error";
/// Contract's functions.
#[allow(dead_code, unused_imports, unused_variables)]
pub mod functions {
    use super::INTERNAL_ERR;
    #[derive(Debug, Clone, PartialEq)]
    pub struct AllPools {
        pub param0: substreams::scalar::BigInt,
    }
    impl AllPools {
        const METHOD_ID: [u8; 4] = [65u8, 209u8, 222u8, 151u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            let maybe_data = call.input.get(4..);
            if maybe_data.is_none() {
                return Err("no data to decode".to_string());
            }
            let mut values =
                ethabi::decode(&[ethabi::ParamType::Uint(256usize)], maybe_data.unwrap())
                    .map_err(|e| format!("unable to decode call.input: {:?}", e))?;
            values.reverse();
            Ok(Self {
                param0: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
            })
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                match self.param0.clone().to_bytes_be() {
                    (num_bigint::Sign::Plus, bytes) => bytes,
                    (num_bigint::Sign::NoSign, bytes) => bytes,
                    (num_bigint::Sign::Minus, _) => {
                        panic!("negative numbers are not supported")
                    }
                }
                .as_slice(),
            ))]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<u8>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_address()
                .expect(INTERNAL_ERR)
                .as_bytes()
                .to_vec())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<u8>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for AllPools {
        const NAME: &'static str = "allPools";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<u8>> for AllPools {
        fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct AllPoolsLength {}
    impl AllPoolsLength {
        const METHOD_ID: [u8; 4] = [239u8, 222u8, 78u8, 100u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Ok(Self {})
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<substreams::scalar::BigInt, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<substreams::scalar::BigInt, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Uint(256usize)], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok({
                let mut v = [0 as u8; 32];
                values
                    .pop()
                    .expect("one output data should have existed")
                    .into_uint()
                    .expect(INTERNAL_ERR)
                    .to_big_endian(v.as_mut_slice());
                substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
            })
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<substreams::scalar::BigInt> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for AllPoolsLength {
        const NAME: &'static str = "allPoolsLength";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<substreams::scalar::BigInt> for AllPoolsLength {
        fn output(data: &[u8]) -> Result<substreams::scalar::BigInt, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct ComputePoolAddress {
        pub pool_params: (
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
        ),
        pub curve_params: (
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
        ),
        pub salt: [u8; 32usize],
    }
    impl ComputePoolAddress {
        const METHOD_ID: [u8; 4] = [139u8, 52u8, 0u8, 35u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            let maybe_data = call.input.get(4..);
            if maybe_data.is_none() {
                return Err("no data to decode".to_string());
            }
            let mut values = ethabi::decode(
                &[
                    ethabi::ParamType::Tuple(vec![
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(256usize),
                    ]),
                    ethabi::ParamType::Tuple(vec![
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                    ]),
                    ethabi::ParamType::FixedBytes(32usize),
                ],
                maybe_data.unwrap(),
            )
            .map_err(|e| format!("unable to decode call.input: {:?}", e))?;
            values.reverse();
            Ok(Self {
                pool_params: {
                    let tuple_elements = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_tuple()
                        .expect(INTERNAL_ERR);
                    (
                        tuple_elements[0usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        tuple_elements[1usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        tuple_elements[2usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[3usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[4usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[5usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[6usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[7usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                    )
                },
                curve_params: {
                    let tuple_elements = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_tuple()
                        .expect(INTERNAL_ERR);
                    (
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[0usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[1usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[2usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[3usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                    )
                },
                salt: {
                    let mut result = [0u8; 32];
                    let v = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_fixed_bytes()
                        .expect(INTERNAL_ERR);
                    result.copy_from_slice(&v);
                    result
                },
            })
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[
                ethabi::Token::Tuple(vec![
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.pool_params.0)),
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.pool_params.1)),
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.pool_params.2)),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.pool_params.3.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.pool_params.4.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.pool_params.5.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.pool_params.6.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.pool_params.7.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                ]),
                ethabi::Token::Tuple(vec![
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .0
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .1
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .2
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .3
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                ]),
                ethabi::Token::FixedBytes(self.salt.as_ref().to_vec()),
            ]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<u8>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_address()
                .expect(INTERNAL_ERR)
                .as_bytes()
                .to_vec())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<u8>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for ComputePoolAddress {
        const NAME: &'static str = "computePoolAddress";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<u8>> for ComputePoolAddress {
        fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct DeployPool {
        pub params: (
            Vec<u8>,
            Vec<u8>,
            Vec<u8>,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
        ),
        pub curve_params: (
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
            substreams::scalar::BigInt,
        ),
        pub salt: [u8; 32usize],
    }
    impl DeployPool {
        const METHOD_ID: [u8; 4] = [55u8, 211u8, 251u8, 8u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            let maybe_data = call.input.get(4..);
            if maybe_data.is_none() {
                return Err("no data to decode".to_string());
            }
            let mut values = ethabi::decode(
                &[
                    ethabi::ParamType::Tuple(vec![
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Address,
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(112usize),
                        ethabi::ParamType::Uint(256usize),
                    ]),
                    ethabi::ParamType::Tuple(vec![
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                        ethabi::ParamType::Uint(256usize),
                    ]),
                    ethabi::ParamType::FixedBytes(32usize),
                ],
                maybe_data.unwrap(),
            )
            .map_err(|e| format!("unable to decode call.input: {:?}", e))?;
            values.reverse();
            Ok(Self {
                params: {
                    let tuple_elements = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_tuple()
                        .expect(INTERNAL_ERR);
                    (
                        tuple_elements[0usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        tuple_elements[1usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        tuple_elements[2usize]
                            .clone()
                            .into_address()
                            .expect(INTERNAL_ERR)
                            .as_bytes()
                            .to_vec(),
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[3usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[4usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[5usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[6usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[7usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                    )
                },
                curve_params: {
                    let tuple_elements = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_tuple()
                        .expect(INTERNAL_ERR);
                    (
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[0usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[1usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[2usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                        {
                            let mut v = [0 as u8; 32];
                            tuple_elements[3usize]
                                .clone()
                                .into_uint()
                                .expect(INTERNAL_ERR)
                                .to_big_endian(v.as_mut_slice());
                            substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                        },
                    )
                },
                salt: {
                    let mut result = [0u8; 32];
                    let v = values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_fixed_bytes()
                        .expect(INTERNAL_ERR);
                    result.copy_from_slice(&v);
                    result
                },
            })
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[
                ethabi::Token::Tuple(vec![
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.params.0)),
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.params.1)),
                    ethabi::Token::Address(ethabi::Address::from_slice(&self.params.2)),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.params.3.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.params.4.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.params.5.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.params.6.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self.params.7.clone().to_bytes_be() {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                ]),
                ethabi::Token::Tuple(vec![
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .0
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .1
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .2
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                    ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                        match self
                            .curve_params
                            .3
                            .clone()
                            .to_bytes_be()
                        {
                            (num_bigint::Sign::Plus, bytes) => bytes,
                            (num_bigint::Sign::NoSign, bytes) => bytes,
                            (num_bigint::Sign::Minus, _) => {
                                panic!("negative numbers are not supported")
                            }
                        }
                        .as_slice(),
                    )),
                ]),
                ethabi::Token::FixedBytes(self.salt.as_ref().to_vec()),
            ]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<u8>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_address()
                .expect(INTERNAL_ERR)
                .as_bytes()
                .to_vec())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<u8>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for DeployPool {
        const NAME: &'static str = "deployPool";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<u8>> for DeployPool {
        fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct EulerAccountToPool {
        pub euler_account: Vec<u8>,
    }
    impl EulerAccountToPool {
        const METHOD_ID: [u8; 4] = [34u8, 48u8, 157u8, 32u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            let maybe_data = call.input.get(4..);
            if maybe_data.is_none() {
                return Err("no data to decode".to_string());
            }
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], maybe_data.unwrap())
                .map_err(|e| format!("unable to decode call.input: {:?}", e))?;
            values.reverse();
            Ok(Self {
                euler_account: values
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
            })
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[ethabi::Token::Address(ethabi::Address::from_slice(
                &self.euler_account,
            ))]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<u8>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_address()
                .expect(INTERNAL_ERR)
                .as_bytes()
                .to_vec())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<u8>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for EulerAccountToPool {
        const NAME: &'static str = "eulerAccountToPool";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<u8>> for EulerAccountToPool {
        fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct Evc {}
    impl Evc {
        const METHOD_ID: [u8; 4] = [167u8, 3u8, 84u8, 161u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Ok(Self {})
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<u8>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            let mut values = ethabi::decode(&[ethabi::ParamType::Address], data.as_ref())
                .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_address()
                .expect(INTERNAL_ERR)
                .as_bytes()
                .to_vec())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<u8>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for Evc {
        const NAME: &'static str = "EVC";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<u8>> for Evc {
        fn output(data: &[u8]) -> Result<Vec<u8>, String> {
            Self::output(data)
        }
    }
    #[derive(Debug, Clone, PartialEq)]
    pub struct GetAllPoolsListSlice {
        pub start: substreams::scalar::BigInt,
        pub end: substreams::scalar::BigInt,
    }
    impl GetAllPoolsListSlice {
        const METHOD_ID: [u8; 4] = [50u8, 154u8, 119u8, 178u8];
        pub fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            let maybe_data = call.input.get(4..);
            if maybe_data.is_none() {
                return Err("no data to decode".to_string());
            }
            let mut values = ethabi::decode(
                &[ethabi::ParamType::Uint(256usize), ethabi::ParamType::Uint(256usize)],
                maybe_data.unwrap(),
            )
            .map_err(|e| format!("unable to decode call.input: {:?}", e))?;
            values.reverse();
            Ok(Self {
                start: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                end: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
            })
        }
        pub fn encode(&self) -> Vec<u8> {
            let data = ethabi::encode(&[
                ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                    match self.start.clone().to_bytes_be() {
                        (num_bigint::Sign::Plus, bytes) => bytes,
                        (num_bigint::Sign::NoSign, bytes) => bytes,
                        (num_bigint::Sign::Minus, _) => {
                            panic!("negative numbers are not supported")
                        }
                    }
                    .as_slice(),
                )),
                ethabi::Token::Uint(ethabi::Uint::from_big_endian(
                    match self.end.clone().to_bytes_be() {
                        (num_bigint::Sign::Plus, bytes) => bytes,
                        (num_bigint::Sign::NoSign, bytes) => bytes,
                        (num_bigint::Sign::Minus, _) => {
                            panic!("negative numbers are not supported")
                        }
                    }
                    .as_slice(),
                )),
            ]);
            let mut encoded = Vec::with_capacity(4 + data.len());
            encoded.extend(Self::METHOD_ID);
            encoded.extend(data);
            encoded
        }
        pub fn output_call(
            call: &substreams_ethereum::pb::eth::v2::Call,
        ) -> Result<Vec<Vec<u8>>, String> {
            Self::output(call.return_data.as_ref())
        }
        pub fn output(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
            let mut values = ethabi::decode(
                &[ethabi::ParamType::Array(Box::new(ethabi::ParamType::Address))],
                data.as_ref(),
            )
            .map_err(|e| format!("unable to decode output data: {:?}", e))?;
            Ok(values
                .pop()
                .expect("one output data should have existed")
                .into_array()
                .expect(INTERNAL_ERR)
                .into_iter()
                .map(|inner| {
                    inner
                        .into_address()
                        .expect(INTERNAL_ERR)
                        .as_bytes()
                        .to_vec()
                })
                .collect())
        }
        pub fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            match call.input.get(0..4) {
                Some(signature) => Self::METHOD_ID == signature,
                None => false,
            }
        }
        pub fn call(&self, address: Vec<u8>) -> Option<Vec<Vec<u8>>> {
            use substreams_ethereum::pb::eth::rpc;
            let rpc_calls = rpc::RpcCalls {
                calls: vec![rpc::RpcCall { to_addr: address, data: self.encode() }],
            };
            let responses = substreams_ethereum::rpc::eth_call(&rpc_calls).responses;
            let response = responses
                .get(0)
                .expect("one response should have existed");
            if response.failed {
                return None;
            }
            match Self::output(response.raw.as_ref()) {
                Ok(data) => Some(data),
                Err(err) => {
                    use substreams_ethereum::Function;
                    substreams::log::info!(
                        "Call output for function `{}` failed to decode with error: {}",
                        Self::NAME,
                        err
                    );
                    None
                }
            }
        }
    }
    impl substreams_ethereum::Function for GetAllPoolsListSlice {
        const NAME: &'static str = "getAllPoolsListSlice";
        fn match_call(call: &substreams_ethereum::pb::eth::v2::Call) -> bool {
            Self::match_call(call)
        }
        fn decode(call: &substreams_ethereum::pb::eth::v2::Call) -> Result<Self, String> {
            Self::decode(call)
        }
        fn encode(&self) -> Vec<u8> {
            self.encode()
        }
    }
    impl substreams_ethereum::rpc::RPCDecodable<Vec<Vec<u8>>> for GetAllPoolsListSlice {
        fn output(data: &[u8]) -> Result<Vec<Vec<u8>>, String> {
            Self::output(data)
        }
    }
}
/// Contract's events.
#[allow(dead_code, unused_imports, unused_variables)]
pub mod events {
    use super::INTERNAL_ERR;
    #[derive(Debug, Clone, PartialEq)]
    pub struct PoolDeployed {
        pub asset0: Vec<u8>,
        pub asset1: Vec<u8>,
        pub vault0: Vec<u8>,
        pub vault1: Vec<u8>,
        pub fee_multiplier: substreams::scalar::BigInt,
        pub euler_account: Vec<u8>,
        pub reserve0: substreams::scalar::BigInt,
        pub reserve1: substreams::scalar::BigInt,
        pub price_x: substreams::scalar::BigInt,
        pub price_y: substreams::scalar::BigInt,
        pub concentration_x: substreams::scalar::BigInt,
        pub concentration_y: substreams::scalar::BigInt,
        pub pool: Vec<u8>,
    }
    impl PoolDeployed {
        const TOPIC_ID: [u8; 32] = [
            245u8, 218u8, 36u8, 250u8, 74u8, 116u8, 165u8, 174u8, 76u8, 113u8, 104u8, 87u8, 132u8,
            4u8, 92u8, 132u8, 114u8, 107u8, 181u8, 210u8, 154u8, 242u8, 210u8, 92u8, 102u8, 128u8,
            138u8, 210u8, 134u8, 209u8, 157u8, 76u8,
        ];
        pub fn match_log(log: &substreams_ethereum::pb::eth::v2::Log) -> bool {
            if log.topics.len() != 4usize {
                return false;
            }
            if log.data.len() != 320usize {
                return false;
            }
            return log
                .topics
                .get(0)
                .expect("bounds already checked")
                .as_ref()
                == Self::TOPIC_ID;
        }
        pub fn decode(log: &substreams_ethereum::pb::eth::v2::Log) -> Result<Self, String> {
            let mut values = ethabi::decode(
                &[
                    ethabi::ParamType::Address,
                    ethabi::ParamType::Address,
                    ethabi::ParamType::Address,
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Uint(256usize),
                    ethabi::ParamType::Address,
                ],
                log.data.as_ref(),
            )
            .map_err(|e| format!("unable to decode log.data: {:?}", e))?;
            values.reverse();
            Ok(Self {
                asset0: ethabi::decode(&[ethabi::ParamType::Address], log.topics[1usize].as_ref())
                    .map_err(|e| {
                        format!(
                            "unable to decode param 'asset0' from topic of type 'address': {:?}",
                            e
                        )
                    })?
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
                asset1: ethabi::decode(&[ethabi::ParamType::Address], log.topics[2usize].as_ref())
                    .map_err(|e| {
                        format!(
                            "unable to decode param 'asset1' from topic of type 'address': {:?}",
                            e
                        )
                    })?
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
                fee_multiplier: {
                    let mut v = [0 as u8; 32];
                    ethabi::decode(
                            &[ethabi::ParamType::Uint(256usize)],
                            log.topics[3usize].as_ref(),
                        )
                        .map_err(|e| {
                            format!(
                                "unable to decode param 'fee_multiplier' from topic of type 'uint256': {:?}",
                                e
                            )
                        })?
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                vault0: values
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
                vault1: values
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
                euler_account: values
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
                reserve0: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                reserve1: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                price_x: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                price_y: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                concentration_x: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                concentration_y: {
                    let mut v = [0 as u8; 32];
                    values
                        .pop()
                        .expect(INTERNAL_ERR)
                        .into_uint()
                        .expect(INTERNAL_ERR)
                        .to_big_endian(v.as_mut_slice());
                    substreams::scalar::BigInt::from_unsigned_bytes_be(&v)
                },
                pool: values
                    .pop()
                    .expect(INTERNAL_ERR)
                    .into_address()
                    .expect(INTERNAL_ERR)
                    .as_bytes()
                    .to_vec(),
            })
        }
    }
    impl substreams_ethereum::Event for PoolDeployed {
        const NAME: &'static str = "PoolDeployed";
        fn match_log(log: &substreams_ethereum::pb::eth::v2::Log) -> bool {
            Self::match_log(log)
        }
        fn decode(log: &substreams_ethereum::pb::eth::v2::Log) -> Result<Self, String> {
            Self::decode(log)
        }
    }
}
