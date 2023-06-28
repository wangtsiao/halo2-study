use std::marker::PhantomData;
use halo2_proofs::{
    circuit::*,
    plonk::*,
    poly::Rotation,
    arithmetic::FieldExt,
};

#[derive(Copy, Clone)]
struct FibonacciConfig {
    col_a: Column<Advice>,
    col_b: Column<Advice>,
    selector: Selector,
    instance: Column<Instance>,
}

#[derive(Copy, Clone)]
struct FibonacciChip<F: FieldExt> {
    config: FibonacciConfig,
    _marker: PhantomData<F>
}


impl<F: FieldExt> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _marker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        let [col_a, col_b] = [(); 2].map(|_| meta.advice_column());
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            let s = meta.query_selector(selector);

            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let next_a = meta.query_advice(col_a, Rotation::next());
            let next_b = meta.query_advice(col_b, Rotation::next());


            vec![s.clone() * (a + b.clone() - next_a.clone()), s * (b + next_a - next_b)]
        });

        FibonacciConfig {
            col_a,
            col_b,
            selector,
            instance
        }
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, nrows: usize)
        -> Result<(AssignedCell<F, F>, AssignedCell<F, F>), Error> {

        layouter.assign_region(
            || "entire fibonacci table",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;

                let mut a_cell = region.assign_advice_from_instance(
                    || "f(0)",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0
                )?;

                let mut b_cell = region.assign_advice_from_instance(
                    || "f(1)",
                    self.config.instance,
                    1,
                    self.config.col_b,
                    0
                )?;

                for row in 1..nrows {
                    if row < nrows - 1 {
                        self.config.selector.enable(&mut region, row)?;
                    }

                    a_cell = region.assign_advice(
                        || "a",
                        self.config.col_a,
                        row,
                        || a_cell.value().copied() + b_cell.value()
                    )?;

                    b_cell = region.assign_advice(
                        || "b",
                        self.config.col_b,
                        row,
                        || b_cell.value().copied() + a_cell.value()
                    )?;
                }

                Ok((a_cell, b_cell))
            }
        )
    }

    fn expose_public(&self, mut layouter: impl Layouter<F>, cell: &AssignedCell<F, F>, row: usize)
        -> Result<(), Error> {
        layouter.constrain_instance(cell.cell(), self.config.instance, row)
    }
}

#[derive(Copy, Clone, Default)]
struct MyCircuit<F: FieldExt>(PhantomData<F>);


impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FibonacciConfig;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FibonacciChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FibonacciChip::construct(config);

        let (_, b_cell) = chip.assign_row(
            layouter.namespace(|| "entire table"),
            5
        )?;

        chip.expose_public(layouter.namespace(|| "out"), &b_cell, 2)
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use crate::fibonacci::example2::MyCircuit;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit(PhantomData);
        let public_input = vec![
            vec![Fp::from(1), Fp::from(1), Fp::from(55)]
        ];
        let prover = MockProver::run(4, &circuit, public_input).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    #[cfg(feature = "dev-graph")]
    fn test_plot_circuit() {
        // cargo test --all-features --color=always --package halo2_study --lib fibonacci::example2::tests::test_plot_circuit --no-fail-fast -- --format=json --exact -Z unstable-options --show-output
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-2-layout.png", (300, 1024)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 2 Layout", ("sans-serif", 60)).unwrap();

        let circuit = MyCircuit::<Fp>::default();
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}
