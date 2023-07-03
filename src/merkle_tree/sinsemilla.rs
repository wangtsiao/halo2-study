use halo2_gadgets::{
    ecc::{
        chip::*,
        NonIdentityPoint,
        FixedPoints,
    },
    sinsemilla::{
        chip::{SinsemillaChip, SinsemillaConfig},
        primitives::{self as sinsemilla},
        CommitDomains, HashDomains, Message, MessagePiece
    },
    utilities::lookup_range_check::LookupRangeCheckConfig,
};
use halo2_gadgets::sinsemilla::HashDomain;

use halo2_proofs::{
    circuit::*,
    plonk::*,
    pasta::*,
    pasta::group::ff::PrimeField,
};
use halo2_proofs::pasta::group::{Curve, Group};

// to initialize the global variables, these variables compose some struct.
use lazy_static::lazy_static;

#[derive(Debug, Eq, PartialEq, Clone)]
struct TestFixedBases;

#[derive(Debug, Eq, PartialEq, Clone)]
struct FullWidth(pallas::Affine, &'static [(u64, [pallas::Base; H])]);

#[derive(Debug, Eq, PartialEq, Clone)]
struct BaseField;

#[derive(Debug, Eq, PartialEq, Clone)]
struct Short;

// to create the commit domain of sinsemilla
const PERSONALIZATION: &str = "MerkleCRH";
lazy_static! {
    // the generator point of elliptic curve
    static ref BASE: pallas::Affine = pallas::Point::generator().to_affine();
    // the zs and us below may not related to the merkle circuit.
    static ref ZS_AND_US: Vec<(u64, [pallas::Base; H])> =
        find_zs_and_us(*BASE, NUM_WINDOWS).unwrap();
    static ref ZS_AND_US_SHORT: Vec<(u64, [pallas::Base; H])> =
        find_zs_and_us(*BASE, NUM_WINDOWS_SHORT).unwrap();

    // create the commit domain
    static ref COMMIT_DOMAIN: sinsemilla::CommitDomain = sinsemilla::CommitDomain::new(PERSONALIZATION);
    static ref Q: pallas::Affine = COMMIT_DOMAIN.Q().to_affine();
    static ref R: pallas::Affine = COMMIT_DOMAIN.R().to_affine();
    static ref R_ZS_AND_US: Vec<(u64, [pallas::Base; H])> =
        find_zs_and_us(*R, NUM_WINDOWS).unwrap();
}

impl FullWidth {
    #[allow(dead_code)]
    pub(crate) fn from_pallas_generator() -> Self {
        FullWidth(*BASE, &ZS_AND_US)
    }

    pub(crate) fn from_parts(
        base: pallas::Affine,
        zs_and_us: &'static [(u64, [pallas::Base; H])],
    ) -> Self {
        FullWidth(base, zs_and_us)
    }
}

impl FixedPoint<pallas::Affine> for FullWidth {
    type FixedScalarKind = FullScalar;

    fn generator(&self) -> pallas::Affine {
        self.0
    }

    fn u(&self) -> Vec<[[u8; 32]; H]> {
        self.1
            .iter()
            .map(|(_, us)| {
                [
                    us[0].to_repr(),
                    us[1].to_repr(),
                    us[2].to_repr(),
                    us[3].to_repr(),
                    us[4].to_repr(),
                    us[5].to_repr(),
                    us[6].to_repr(),
                    us[7].to_repr(),
                ]
            })
            .collect()
    }

    fn z(&self) -> Vec<u64> {
        self.1.iter().map(|(z, _)| *z).collect()
    }
}

impl FixedPoint<pallas::Affine> for BaseField {
    type FixedScalarKind = BaseFieldElem;

    fn generator(&self) -> pallas::Affine {
        *BASE
    }

    fn u(&self) -> Vec<[[u8; 32]; H]> {
        ZS_AND_US
            .iter()
            .map(|(_, us)| {
                [
                    us[0].to_repr(),
                    us[1].to_repr(),
                    us[2].to_repr(),
                    us[3].to_repr(),
                    us[4].to_repr(),
                    us[5].to_repr(),
                    us[6].to_repr(),
                    us[7].to_repr(),
                ]
            })
            .collect()
    }

    fn z(&self) -> Vec<u64> {
        ZS_AND_US.iter().map(|(z, _)| *z).collect()
    }
}

impl FixedPoint<pallas::Affine> for Short {
    type FixedScalarKind = ShortScalar;

    fn generator(&self) -> pallas::Affine {
        *BASE
    }

    fn u(&self) -> Vec<[[u8; 32]; H]> {
        ZS_AND_US_SHORT
            .iter()
            .map(|(_, us)| {
                [
                    us[0].to_repr(),
                    us[1].to_repr(),
                    us[2].to_repr(),
                    us[3].to_repr(),
                    us[4].to_repr(),
                    us[5].to_repr(),
                    us[6].to_repr(),
                    us[7].to_repr(),
                ]
            })
            .collect()
    }

    fn z(&self) -> Vec<u64> {
        ZS_AND_US_SHORT.iter().map(|(z, _)| *z).collect()
    }
}

impl FixedPoints<pallas::Affine> for TestFixedBases {
    type FullScalar = FullWidth;
    type ShortScalar = Short;
    type Base = BaseField;
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct TestHashDomain;
impl HashDomains<pallas::Affine> for TestHashDomain {
    #[allow(non_snake_case)]
    fn Q(&self) -> pallas::Affine {
        *Q
    }
}

// This test does not make use of the CommitDomain.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct TestCommitDomain;
impl CommitDomains<pallas::Affine, TestFixedBases, TestHashDomain> for TestCommitDomain {
    fn r(&self) -> FullWidth {
        FullWidth::from_parts(*R, &R_ZS_AND_US)
    }

    fn hash_domain(&self) -> TestHashDomain {
        TestHashDomain
    }
}

#[derive(Default, Copy, Clone)]
struct MyCircuit {
    data: [bool; 10],
}

impl Circuit<pallas::Base> for MyCircuit {
    type Config = (
        EccConfig<TestFixedBases>,
        SinsemillaConfig<TestHashDomain, TestCommitDomain, TestFixedBases>,
    );
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<pallas::Base>) -> Self::Config {
        let advices = [(); 10].map(|_| meta.advice_column());

        // Shared fixed column for loading constants
        let constants = meta.fixed_column();
        meta.enable_constant(constants);

        let lagrange_coeffs = [(); 8].map(|_| meta.fixed_column());
        let table_idx = meta.lookup_table_column();
        let range_check = LookupRangeCheckConfig::configure(
            meta,
            advices[9],
            table_idx
        );

        let ecc_config = EccChip::<TestFixedBases>::configure(
            meta,
            advices,
            lagrange_coeffs,
            range_check
        );

        // fixed columns for the sinsemilla generator lookup table
        let lookup = (
            table_idx,
            meta.lookup_table_column(),
            meta.lookup_table_column(),
        );

        let sinsemilla_config = SinsemillaChip::configure(
            meta,
            advices[..5].try_into().unwrap(),
            advices[2],
            lagrange_coeffs[0],
            lookup,
            range_check,
        );

        (ecc_config, sinsemilla_config)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<pallas::Base>) -> Result<(), Error> {
        let ecc_chip = EccChip::construct(config.0);

        // load the lookup table
        SinsemillaChip::load(config.1.clone(), &mut layouter)?;

        let sinsemilla_chip = SinsemillaChip::construct(config.1);

        let hash_handler = HashDomain::new(
            sinsemilla_chip.clone(),
            ecc_chip.clone(),
            &TestHashDomain
        );

        let field_ele = self.data.into_iter().rev().fold(pallas::Base::zero(), |acc, bit| {
            if bit {
                acc.double() + pallas::Base::one()
            } else {
                acc.double()
            }
        });

        let message_piece = MessagePiece::from_field_elem(
            sinsemilla_chip.clone(),
            layouter.namespace(|| "message"),
            Value::known(field_ele),
            1
        )?;


        let expected_point= {
            let hash_handler = sinsemilla::HashDomain::new(&format!("{}-M", PERSONALIZATION));
            let expected_point = hash_handler.hash_to_point(self.data.into_iter()).unwrap();

            NonIdentityPoint::new(
                ecc_chip.clone(),
                layouter.namespace(|| "expected point"),
                Value::known(expected_point.to_affine())
            )?
        };

        let (result, _) = hash_handler.hash_to_point(
            layouter.namespace(|| "hash to point"),
            Message::from_pieces(sinsemilla_chip.clone(), vec![message_piece.clone()])
        )?;


        result.constrain_equal(
            layouter.namespace(|| "result == expected_point"),
            &expected_point
        )
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::dev::MockProver;
    use crate::merkle_tree::sinsemilla::MyCircuit;

    #[test]
    fn test_circuit() {
        let k = 11;
        let circuit = MyCircuit {
            data: [true, true, false, false, false, false, false, false, false, true]
        };
        let prover = MockProver::run(k, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[cfg(feature = "dev-graph")]
    #[test]
    fn print_sinsemilla_chip() {
        use plotters::prelude::*;

        let root =
            BitMapBackend::new("sinsemilla-hash-layout.png", (1024, 7680)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("SinsemillaHash", ("sans-serif", 60)).unwrap();

        let circuit = MyCircuit {
            data: [true, true, false, false, false, false, false, false, false, false]
        };
        halo2_proofs::dev::CircuitLayout::default()
            .render(11, &circuit, &root)
            .unwrap();
    }
}
