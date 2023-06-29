/// This helper checks that the value witnessed in a given cell is within a given range.
/// Depending on the range, this helper uses either a range-check expression (for small ranges),
/// or a lookup (for large ranges).
///```txt
///        value     |    q_range_check    |   q_lookup  |  table  |
///       ----------------------------------------------------------------
///          v_0     |         1           |      0      |       0       |
///          v_1     |         0           |      1      |       1       |
///```
use halo2_proofs::{
    circuit::*,
    plonk::*,
    arithmetic::FieldExt,
    poly::Rotation,
};

mod table;
use table::RangeCheckTable;

const RANGE_CHECK_BITS: usize = 3;

#[derive(Clone)]
struct RangeCheckConfig<F: FieldExt, const NUM_BITS: usize> {
    value: Column<Advice>,
    q_range_check: Selector,
    q_lookup: Selector,
    table: RangeCheckTable<F, NUM_BITS>
}

impl<F: FieldExt, const NUM_BITS: usize> RangeCheckConfig<F, NUM_BITS> {
    fn configure(
        meta: &mut ConstraintSystem<F>,
        value: Column<Advice>
    ) -> Self {
        let q_range_check = meta.selector();
        let q_lookup = meta.complex_selector();

        let table = RangeCheckTable::configure(meta);


        // Range-check gate
        // for a value v and a range R, check that v < R
        //  v * (1-v) * (2-v) * (R-1-v) = 0
        meta.create_gate("range check", |meta| {
            let v = meta.query_advice(value, Rotation::cur());
            let s = meta.query_selector(q_range_check);

            let range_check = |range: usize, value: Expression<F>| {
                (1..range).fold(value.clone(), |expr, i| {
                    expr * (Expression::Constant(F::from(i as u64))- value.clone())
                })
            };

            Constraints::with_selector(s, [("range check", range_check(1<<NUM_BITS, v))])
        });

        // Range-check lookup
        // check that a value v is contained within a lookup table of table 0...(1<<NUM_BITS)
        meta.lookup(|meta| {
            let q_lookup = meta.query_selector(q_lookup);
            let value = meta.query_advice(value, Rotation::cur());

            vec![
                (q_lookup * value, table.value)
            ]
        });

        Self {
            value,
            q_range_check,
            q_lookup,
            table
        }
    }

    fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        value: Value<Assigned<F>>,
        num_bits: usize,
    ) -> Result<AssignedCell<Assigned<F>, F>, Error> {
        assert!(num_bits <= NUM_BITS);

        if num_bits <= RANGE_CHECK_BITS {
            layouter.assign_region(
                || "assign value for simple range check",
                |mut region| {
                    self.q_range_check.enable(&mut region, 0)?;
                    region.assign_advice(|| "value", self.value, 0, || value)
                }
            )
        } else {
            layouter.assign_region(
                || "assign value for lookup range check",
                |mut region| {
                    self.q_lookup.enable(&mut region, 0)?;
                    region.assign_advice(|| "value", self.value, 0, || value)
                }
            )
        }
    }
}

#[derive(Default)]
struct MyCircuit<F> {
    v: F,
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = RangeCheckConfig<F, 8>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        let v = meta.advice_column();
        RangeCheckConfig::configure(meta, v)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        config.table.assign(&mut layouter)?;

        config.assign(
            layouter.namespace(|| "assign value"),
            Value::known(Assigned::from(self.v)),
            8
        )?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use crate::range_check::example2::MyCircuit;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit {
            v: Fp::from(55)
        };
        let prover = MockProver::run(9, &circuit, vec![]).unwrap();
        prover.assert_satisfied();

        let circuit = MyCircuit {
            v: Fp::from(6)
        };
        let prover = MockProver::run(9, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }
}
