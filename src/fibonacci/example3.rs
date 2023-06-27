use std::marker::PhantomData;
use halo2_proofs::{
    circuit::*,
    plonk::*,
    arithmetic::Field,
};
use halo2_proofs::poly::Rotation;

#[derive(Copy, Clone)]
struct FibonacciConfig {
    advice: Column<Advice>,
    selector: Selector,
    instance: Column<Instance>,
}

#[derive(Copy, Clone)]
struct FibonacciChip<F: Field> {
    config: FibonacciConfig,
    _maker: PhantomData<F>,
}

impl<F: Field> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _maker: PhantomData,
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        let advice = meta.advice_column();
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(advice);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            let a = meta.query_advice(advice, Rotation::cur());
            let b = meta.query_advice(advice, Rotation::next());
            let c = meta.query_advice(advice, Rotation(2));

            let s = meta.query_selector(selector);

            vec![s * (a + b - c)]
        });

        FibonacciConfig {
            advice,
            selector,
            instance,
        }
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, nrows: usize)
        -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "entire fibonacci table",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                self.config.selector.enable(&mut region, 1)?;

                let mut a_cell = region.assign_advice_from_instance(
                    || "f(0)",
                    self.config.instance,
                    0,
                    self.config.advice,
                    0
                )?;

                let mut b_cell = region.assign_advice_from_instance(
                    || "f(1)",
                    self.config.instance,
                    1,
                    self.config.advice,
                    1
                )?;

                for row in 2..nrows {
                    if row < nrows - 2 {
                        self.config.selector.enable(&mut region, row)?;
                    }

                    let c_cell = region.assign_advice(
                        || "next row",
                        self.config.advice,
                        row,
                        || a_cell.value().copied() + b_cell.value()
                    )?;

                    a_cell = b_cell;
                    b_cell = c_cell;
                }

                Ok(b_cell)
            }
        )
    }

    fn expose_public(&self, mut layouter: impl Layouter<F>, cell: &AssignedCell<F, F>, row: usize)
        -> Result<(), Error> {
        layouter.constrain_instance(
            cell.cell(),
            self.config.instance,
            row,
        )
    }
}

#[derive(Copy, Clone, Default)]
struct MyCircuit<F: Field>(PhantomData<F>);


impl<F: Field> Circuit<F> for MyCircuit<F> {
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
        let out_cell = chip.assign_row(
            layouter.namespace(|| "entire table"),
            10
        )?;

        chip.expose_public(
            layouter.namespace(|| "expose public"),
            &out_cell,
            2
        )
    }
}

#[cfg(test)]
mod tests {
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use crate::fibonacci::example3::MyCircuit;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit::default();
        let public_input = vec![
            vec![Fp::from(1), Fp::from(1), Fp::from(55)]
        ];
        let prover = MockProver::run(4, &circuit, public_input).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    #[cfg(feature = "dev-graph")]
    fn test_plot_circuit() {
        // cargo test --all-features --color=always --package halo2_study --lib fibonacci::example3::tests::test_plot_circuit --no-fail-fast -- --format=json --exact -Z unstable-options --show-output
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-3-layout.png", (300, 1024)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 3 Layout", ("sans-serif", 60)).unwrap();

        let circuit = MyCircuit::<Fp>::default();
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}
