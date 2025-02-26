use super::{
    bitwise_instance_def::BitwiseInstanceDef, ec_op_instance_def::EcOpInstanceDef,
    ecdsa_instance_def::EcdsaInstanceDef, keccak_instance_def::KeccakInstanceDef,
    pedersen_instance_def::PedersenInstanceDef, poseidon_instance_def::PoseidonInstanceDef,
    range_check_instance_def::RangeCheckInstanceDef,
};

#[derive(Debug, PartialEq)]
pub(crate) struct BuiltinsInstanceDef {
    pub(crate) output: bool,
    pub(crate) pedersen: Option<PedersenInstanceDef>,
    pub(crate) range_check: Option<RangeCheckInstanceDef>,
    pub(crate) ecdsa: Option<EcdsaInstanceDef>,
    pub(crate) bitwise: Option<BitwiseInstanceDef>,
    pub(crate) ec_op: Option<EcOpInstanceDef>,
    pub(crate) keccak: Option<KeccakInstanceDef>,
    pub(crate) poseidon: Option<PoseidonInstanceDef>,
}

impl BuiltinsInstanceDef {
    pub(crate) fn plain() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: false,
            pedersen: None,
            range_check: None,
            ecdsa: None,
            bitwise: None,
            ec_op: None,
            keccak: None,
            poseidon: None,
        }
    }

    pub(crate) fn small() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: None,
            ec_op: None,
            keccak: None,
            poseidon: None,
        }
    }

    pub(crate) fn dex() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: None,
            ec_op: None,
            keccak: None,
            poseidon: None,
        }
    }

    pub(crate) fn perpetual_with_bitwise() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::new(32, 1)),
            range_check: Some(RangeCheckInstanceDef::new(16, 8)),
            ecdsa: Some(EcdsaInstanceDef::new(2048)),
            bitwise: Some(BitwiseInstanceDef::new(64)),
            ec_op: Some(EcOpInstanceDef::new(1024)),
            keccak: None,
            poseidon: None,
        }
    }

    pub(crate) fn bitwise() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::new(256, 1)),
            range_check: Some(RangeCheckInstanceDef::default()),
            ecdsa: Some(EcdsaInstanceDef::new(1024)),
            bitwise: Some(BitwiseInstanceDef::new(8)),
            ec_op: None,
            keccak: None,
            poseidon: None,
        }
    }

    pub(crate) fn recursive() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::new(256, 1)),
            range_check: Some(RangeCheckInstanceDef::default()),
            ecdsa: None,
            bitwise: Some(BitwiseInstanceDef::new(16)),
            ec_op: None,
            keccak: Some(KeccakInstanceDef::new(2048, vec![200; 8])),
            poseidon: None,
        }
    }

    pub(crate) fn all() -> BuiltinsInstanceDef {
        BuiltinsInstanceDef {
            output: true,
            pedersen: Some(PedersenInstanceDef::default()),
            range_check: Some(RangeCheckInstanceDef::default()),
            ecdsa: Some(EcdsaInstanceDef::default()),
            bitwise: Some(BitwiseInstanceDef::default()),
            ec_op: Some(EcOpInstanceDef::default()),
            keccak: None,
            poseidon: Some(PoseidonInstanceDef::default()), // Added for testing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_plain() {
        let builtins = BuiltinsInstanceDef::plain();
        assert!(!builtins.output);
        assert!(builtins.pedersen.is_none());
        assert!(builtins.range_check.is_none());
        assert!(builtins.ecdsa.is_none());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_small() {
        let builtins = BuiltinsInstanceDef::small();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_some());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_dex() {
        let builtins = BuiltinsInstanceDef::dex();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_some());
        assert!(builtins.bitwise.is_none());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_perpetual_with_bitwise() {
        let builtins = BuiltinsInstanceDef::perpetual_with_bitwise();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_some());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_bitwise() {
        let builtins = BuiltinsInstanceDef::bitwise();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_recursive() {
        let builtins = BuiltinsInstanceDef::recursive();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_none());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_none());
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_builtins_all() {
        let builtins = BuiltinsInstanceDef::all();
        assert!(builtins.output);
        assert!(builtins.pedersen.is_some());
        assert!(builtins.range_check.is_some());
        assert!(builtins.ecdsa.is_some());
        assert!(builtins.bitwise.is_some());
        assert!(builtins.ec_op.is_some());
    }
}
