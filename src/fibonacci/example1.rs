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
    col_c: Column<Advice>,
    selector: Selector,
    instance: Column<Instance>,
}

#[derive(Copy, Clone)]
struct FibonacciChip<F: FieldExt> {
    config: FibonacciConfig,
    _maker: PhantomData<F>,
}

impl<F: FieldExt> FibonacciChip<F> {
    fn construct(config: FibonacciConfig) -> Self {
        Self {
            config,
            _maker: PhantomData
        }
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> FibonacciConfig {
        let [col_a, col_b, col_c] = [(); 3].map(|_| meta.advice_column());
        let selector = meta.selector();
        let instance = meta.instance_column();

        meta.enable_equality(col_a);
        meta.enable_equality(col_b);
        meta.enable_equality(col_c);
        meta.enable_equality(instance);

        meta.create_gate("add", |meta| {
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            let s = meta.query_selector(selector);
            vec![s * (a + b - c)]
        });

        FibonacciConfig {
            col_a,
            col_b,
            col_c,
            selector,
            instance,
        }
    }

    fn assign_first_row(&self, mut layouter: impl Layouter<F>)
        -> Result<(AssignedCell<F, F>, AssignedCell<F, F>, AssignedCell<F, F>), Error> {
        layouter.assign_region(
            || "first row",
            |mut region| {

                self.config.selector.enable(&mut region, 0)?;


                let a_cell = region.assign_advice_from_instance(
                    || "f(0)",
                    self.config.instance,
                    0,
                    self.config.col_a,
                    0,
                )?;

                let b_cell = region.assign_advice_from_instance(
                    || "f(1)",
                    self.config.instance,
                    1,
                    self.config.col_b,
                    0,
                )?;

                let c_cell = region.assign_advice(
                    || "a + b",
                    self.config.col_c,
                    0,
                    || a_cell.value().copied() + b_cell.value(),
                )?;

                Ok((a_cell, b_cell, c_cell))
            },
        )
    }

    fn assign_row(&self, mut layouter: impl Layouter<F>, prev_b: &AssignedCell<F, F>, prev_c: &AssignedCell<F, F>)
        -> Result<AssignedCell<F, F>, Error> {
        layouter.assign_region(
            || "next row",
            |mut region| {

                self.config.selector.enable(&mut region, 0)?;

                prev_b.copy_advice(|| "a", &mut region, self.config.col_a, 0)?;
                prev_c.copy_advice(|| "b", &mut region, self.config.col_b, 0)?;

                let c_cell = region.assign_advice(
                    || "c",
                    self.config.col_c,
                    0,
                    || prev_b.value().copied() + prev_c.value(),
                )?;

                Ok(c_cell)
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
        Self::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FibonacciChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, mut layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FibonacciChip::construct(config);

        let (_, mut prev_b, mut prev_c) = chip.assign_first_row(
            layouter.namespace(|| "assign first row")
        )?;

        for _i in 3..10 {
            let c_cell = chip.assign_row(
                layouter.namespace(|| "assign next row"),
                &prev_b,
                &prev_c,
            )?;
            prev_b = prev_c;
            prev_c = c_cell;
        }

        chip.expose_public(
            layouter.namespace(|| "expose public"),
            &prev_c,
            2
        )
    }
}

#[cfg(test)]
mod tests {
    use std::marker::PhantomData;
    use halo2_proofs::dev::MockProver;
    use halo2_proofs::pasta::Fp;
    use crate::fibonacci::example1::MyCircuit;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit(PhantomData);
        let prover = MockProver::run(
            4,
            &circuit,
            vec![vec![Fp::from(1), Fp::from(1), Fp::from(55)]]).unwrap();

        prover.assert_satisfied();
    }

    #[test]
    #[cfg(feature = "dev-graph")]
    fn test_plot_circuit() {
        // cargo test --all-features --color=always --package halo2_study --lib fibonacci::example1::tests::test_plot_circuit --no-fail-fast -- --format=json --exact -Z unstable-options --show-output
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-1-layout.png", (300, 1024)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 1 Layout", ("sans-serif", 60)).unwrap();

        let circuit = MyCircuit::<Fp>(PhantomData);
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}
