use halo2_proofs::{
    circuit::*,
    plonk::*,
    poly::Rotation,
    arithmetic::FieldExt,
};
use crate::is_zero::{IsZeroChip, IsZeroConfig};

/// here is the function
/// ```python
/// def fun(a, b, c):
///     if a == b:
///         return c
///     return a - b
/// ```

#[derive(Clone, Debug)]
struct FunctionConfig<F: FieldExt> {
    col_a: Column<Advice>,
    col_b: Column<Advice>,
    col_c: Column<Advice>,
    selector: Selector,
    a_equal_b: IsZeroConfig<F>,
    output: Column<Advice>,
}

#[derive(Clone)]
struct FunctionChip<F: FieldExt> {
    config: FunctionConfig<F>
}

impl<F: FieldExt> FunctionChip<F> {
    pub fn construct(config: FunctionConfig<F>) -> Self {
        Self {
            config
        }
    }

    pub fn configure(meta: &mut ConstraintSystem<F>) -> FunctionConfig<F> {
        let [col_a, col_b, col_c] = [(); 3].map(|_| meta.advice_column());
        let selector = meta.selector();
        let is_zero_advice_col = meta.advice_column();
        let output = meta.advice_column();

        let a_equal_b = IsZeroChip::configure(
            meta,
            |meta| meta.query_selector(selector),
            |meta| meta.query_advice(col_a, Rotation::cur()) - meta.query_advice(col_b, Rotation::cur()),
            is_zero_advice_col
        );

        meta.create_gate("f(a, b, c) = if a == b {c} else {a - b}", |meta| {
            let s = meta.query_selector(selector);
            let a = meta.query_advice(col_a, Rotation::cur());
            let b = meta.query_advice(col_b, Rotation::cur());
            let c = meta.query_advice(col_c, Rotation::cur());
            let output = meta.query_advice(output, Rotation::cur());

            vec![
                s.clone() * (a_equal_b.expr() * (output.clone() - c)),
                s * (Expression::Constant(F::one()) - a_equal_b.expr()) * (output - (a - b)),
            ]
        });

        FunctionConfig {
            col_a,
            col_b,
            col_c,
            selector,
            a_equal_b,
            output,
        }
    }

    pub fn assign(
        &self,
        mut layouter: impl Layouter<F>,
        a: F,
        b: F,
        c: F,
    ) -> Result<(), Error> {
        let is_zero_chip = IsZeroChip::construct(self.config.a_equal_b.clone());

        layouter.assign_region(
            || "f(a, b, c) = if a=b {c} else {a-b}",
            |mut region| {
                self.config.selector.enable(&mut region, 0)?;
                region.assign_advice(|| "a", self.config.col_a, 0, || Value::known(a))?;
                region.assign_advice(|| "b", self.config.col_b, 0, || Value::known(b))?;
                region.assign_advice(|| "c", self.config.col_c, 0, || Value::known(c))?;

                is_zero_chip.assign(&mut region, 0, Value::known(a-b))?;

                let output = if a==b {c} else {a-b};
                region.assign_advice(||"output", self.config.output, 0, || Value::known(output))?;
                Ok(())
            }
        )
    }
}


#[derive(Default)]
struct MyCircuit<F> {
    a: F,
    b: F,
    c: F
}

impl<F: FieldExt> Circuit<F> for MyCircuit<F> {
    type Config = FunctionConfig<F>;
    type FloorPlanner = SimpleFloorPlanner;

    fn without_witnesses(&self) -> Self {
        MyCircuit::default()
    }

    fn configure(meta: &mut ConstraintSystem<F>) -> Self::Config {
        FunctionChip::configure(meta)
    }

    fn synthesize(&self, config: Self::Config, layouter: impl Layouter<F>) -> Result<(), Error> {
        let chip = FunctionChip::construct(config);

        chip.assign(layouter, self.a, self.b, self.c)
    }
}

#[cfg(test)]
mod tests{
    use halo2_proofs::dev::MockProver;
    use crate::fibonacci::example4::MyCircuit;
    use halo2_proofs::pasta::Fp;

    #[test]
    fn test_circuit() {
        let circuit = MyCircuit {
            a: Fp::from(12),
            b: Fp::from(12),
            c: Fp::from(15),
        };

        let prover = MockProver::run(4, &circuit, vec![]).unwrap();
        prover.assert_satisfied();
    }

    #[test]
    #[cfg(feature = "dev-graph")]
    fn test_plot_circuit() {
        // cargo test --all-features --color=always --package halo2_study --lib fibonacci::example4::tests::test_plot_circuit --no-fail-fast -- --format=json --exact -Z unstable-options --show-output
        use plotters::prelude::*;
        let root = BitMapBackend::new("fib-4-layout.png", (300, 1024)).into_drawing_area();
        root.fill(&WHITE).unwrap();
        let root = root.titled("Fib 4 Layout", ("sans-serif", 60)).unwrap();

        let circuit = MyCircuit::<Fp>::default();
        halo2_proofs::dev::CircuitLayout::default()
            .render(4, &circuit, &root)
            .unwrap();
    }
}
