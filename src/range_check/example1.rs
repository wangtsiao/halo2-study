use std::marker::PhantomData;
use halo2_proofs::{
    circuit::*,
    plonk::*,
    poly::Rotation,
    arithmetic::FieldExt,
};


/// This helper checks that the value witnessed in a given cell is within a given range.
#[derive(Clone, Copy)]
struct RangeCheckConfig<F: FieldExt, const RANGE: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    _marker: PhantomData<F>
}


struct RangeCheckChip<F: FieldExt, const RANGE: usize> {
    config: RangeCheckConfig<F, RANGE>,
}


impl<F: FieldExt, const RANGE: usize> RangeCheckChip<F, RANGE> {
    fn construct(config: RangeCheckConfig<F, RANGE>) -> Self {
        Self {
            config
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> RangeCheckConfig<F, RANGE> {
        let value = meta.advice_column();
        let q_range_check = meta.selector();

        meta.create_gate("range check", |meta| {
            let v = meta.query_advice(value, Rotation::cur());
            let s = meta.query_selector(q_range_check);

            let range_check = |range: usize, value: Expression<F>| {
                (1..range).fold(value.clone(), |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64))- value.clone())
                })
            };

            Constraints::with_selector(s, [("range check", range_check(RANGE, v))])
        });

        RangeCheckConfig {
            value,
            q_range_check,
            _marker: PhantomData,
        }
    }

    fn assign(&self, mut layouter: impl Layouter<F>, value: F) -> Result<(), Error> {
        layouter.assign_region(
            || "assign value",
            |mut region| {
                self.config.q_range_check.enable(&mut region, 0)?;

                region.assign_advice(||"value", self.config.value, 0, || Value::known(value))?;

                Ok(())
            }
        )
    }
}

#[derive(Default, Copy, Clone)]
struct MyCircuit<F> {
    v: F
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = RangeCheckConfig<F, 8>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        RangeCheckChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = RangeCheckChip::construct(config);
        chip.assign(layouter, self.v)
    }
}


#[cfg(test)]
mod tests {
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use crate::range_check::example1::MyCircuit;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit {
            v: Fp::from(2)
        };

        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
}
