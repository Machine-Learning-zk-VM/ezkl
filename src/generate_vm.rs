use crate::graph::{GraphCircuit, GraphWitness};
use halo2_proofs::dev::{CellValue, InstanceValue};
use halo2curves::bn256::Fr;
#[cfg(not(target_arch = "wasm32"))]
use itertools::Itertools;
#[cfg(not(target_arch = "wasm32"))]
use log::info;
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::fs::File;
#[cfg(not(target_arch = "wasm32"))]
use std::io::Write;
use std::path::PathBuf;

const POWDR_TEMPLATE: &str = include_str!("powdr_template.asm");

enum Arg {
    Mem((usize, usize), usize),
    Constant(Fr),
}

impl Display for Arg {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Arg::Mem(_, addr) => write!(f, "{}", addr),
            Arg::Constant(_) => panic!(),
        }
    }
}

/// Generates the assembly program
pub(crate) fn generate_program(
    compiled_circuit_path: PathBuf,
    data_path: PathBuf,
) -> Result<(), Box<dyn Error>> {
    // mock should catch any issues by default so we set it to safe
    let mut circuit = GraphCircuit::load(compiled_circuit_path)?;

    let data = GraphWitness::from_path(data_path)?;

    circuit.load_graph_witness(&data)?;

    let public_inputs = circuit.prepare_public_inputs(&data)?;

    info!("Mock proof");

    let prover = halo2_proofs::dev::MockProver::run(
        circuit.settings().run_args.logrows,
        &circuit,
        vec![public_inputs],
    )
    .map_err(Box::<dyn Error>::from)?;

    assert_eq!("[Column { index: 0, column_type: Advice }, Column { index: 1, column_type: Advice }, Column { index: 2, column_type: Advice }, Column { index: 0, column_type: Fixed }, Column { index: 0, column_type: Instance }]", format!("{:?}", prover.permutation().columns));
    println!("Permutation columns: {:?}", prover.permutation().columns);
    println!("Permutation mappings: [");
    for m in &prover.permutation().mapping {
        let m = m.iter().take(20).collect::<Vec<_>>();
        println!("  {:?},", m);
    }
    println!("]");
    println!("Permutation aux: [");
    for m in &prover.permutation().aux {
        let m = m.iter().take(20).collect::<Vec<_>>();
        println!("  {:?},", m);
    }
    println!("]");

    // See this algorithm: https://zcash.github.io/halo2/design/proving-system/permutation.html
    let aux = &prover.permutation().aux;
    println!("Building Unique representatives...");
    let mut unique_representatives = aux.iter().flat_map(|a| a.iter()).collect::<Vec<_>>();
    unique_representatives
        .sort_by(|(col1, row1), (col2, row2)| row1.cmp(row2).then_with(|| col1.cmp(col2)));
    println!(
        "unique_representatives[:20]: {:?}",
        &unique_representatives[..20]
    );
    println!("Building representative to mem address...");
    let representative_to_mem_address = unique_representatives
        .iter()
        .enumerate()
        .map(|(i, r)| (*r, i))
        .collect::<BTreeMap<_, _>>();
    println!("Num fixed columns: {}", prover.fixed().len());
    for (i, col) in prover.fixed().iter().enumerate() {
        let mut counter = 0;
        for v in col.iter() {
            if let CellValue::Assigned(_) = v {
                counter += 1;
            }
        }
        println!("Fixed column {}: {} items", i, counter);
    }
    let mut representative_to_instance = BTreeMap::new();
    for (i, rep) in aux[4].iter().enumerate() {
        if let InstanceValue::Assigned(value) = prover.instance[0][i] {
            println!("Instance ({}, {}) {} -> {:?}", rep.0, rep.1, i, value);
            assert!(representative_to_instance.insert(*rep, value).is_none());
        }
    }
    println!("Num instance values: {}", representative_to_instance.len());
    let mut representative_to_fixed = BTreeMap::new();
    for (i, rep) in aux[3].iter().enumerate() {
        if let CellValue::Assigned(value) = prover.fixed()[0][i] {
            println!("Fixed ({}, {}) {} -> {:?}", rep.0, rep.1, i, value);
            assert!(representative_to_fixed.insert(*rep, value).is_none());
        }
    }
    println!("Num fixed values: {}", representative_to_fixed.len());
    println!("Building args...");
    let args = aux
        .iter()
        .map(|col| {
            col.iter()
                .map(|rep| {
                    if let Some(v) = representative_to_instance.get(rep) {
                        Arg::Constant(*v)
                    } else if let Some(v) = representative_to_fixed.get(rep) {
                        Arg::Constant(*v)
                    } else {
                        Arg::Mem(*rep, representative_to_mem_address[rep])
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();

    let selectors = &prover.selectors;
    // 12 x 131072
    println!(
        "Selectors shape: {} x {}",
        selectors.len(),
        selectors[0].len()
    );

    // I'm guessing these are the names...
    let names = [
        "add", "sub", "dot", "cumprod", "sum", "neg", "mult", "iszero", "identity", "isbool",
        "div_128", "relu",
    ];
    let mut mem = vec![];
    let mut rep_to_addr = BTreeMap::new();

    let mut get_addr = |arg: &Arg| {
        if let Arg::Mem(rep, _) = arg {
            if let Some(addr) = rep_to_addr.get(rep) {
                *addr
            } else {
                let addr = mem.len();
                rep_to_addr.insert(*rep, addr);

                let (col, row) = *rep;
                assert!(col < 3);
                let v = if let CellValue::Assigned(v) = prover.advice[col][row] {
                    v
                } else {
                    panic!()
                };
                mem.push(v);

                addr
            }
        } else {
            panic!()
        }
    };

    let mut program = vec![];

    for row in 0..selectors[0].len() {
        let active_gates = selectors.iter().positions(|x| x[row]).collect::<Vec<_>>();
        assert!(active_gates.len() <= 1);
        if active_gates.len() == 1 {
            let active_gate = active_gates[0];
            let name = names[active_gate];

            match name {
                "add" => {
                    program.push(format!(
                        "add {}, {}, {};",
                        get_addr(&args[0][row]),
                        get_addr(&args[1][row]),
                        get_addr(&args[2][row]),
                    ));
                }
                "sub" => todo!(),
                "dot" => {
                    program.push(format!(
                        "dot {}, {}, {}, {};",
                        get_addr(&args[0][row]),
                        get_addr(&args[1][row]),
                        get_addr(&args[2][row - 1]),
                        get_addr(&args[2][row]),
                    ));
                }
                "cumprod" => todo!(),
                "sum" => todo!(),
                "neg" => todo!(),
                "mult" => {
                    program.push(format!(
                        "mult {}, {}, {};",
                        get_addr(&args[0][row]),
                        get_addr(&args[1][row]),
                        get_addr(&args[2][row]),
                    ));
                }
                "iszero" => todo!(),
                "identity" => todo!(),
                "isbool" => todo!(),
                "div_128" => {
                    program.push(format!(
                        "div_128 {}, {};",
                        get_addr(&args[0][row]),
                        get_addr(&args[1][row]),
                    ));
                }
                "relu" => {
                    program.push(format!(
                        "relu {}, {};",
                        get_addr(&args[0][row]),
                        get_addr(&args[1][row]),
                    ));
                }
                &_ => panic!(),
            };
        }
    }
    let program = program.join("\n");
    let vm = POWDR_TEMPLATE.replace("{{program}}", &program);

    let mut file = File::create("vm.asm").unwrap();
    writeln!(file, "{}", vm).unwrap();
    file.flush().unwrap();

    let mut file = File::create("memory.csv").unwrap();
    writeln!(file, "main.mem").unwrap();

    let to_str = |mut x: Fr| {
        let neg = x > x.neg();
        if neg {
            x = x.neg();
        }
        let hex_str = format!("{:?}", x);
        let trimmed = hex_str.trim_start_matches("0x");
        let x = i128::from_str_radix(trimmed, 16).unwrap();
        format!("{}{}", if neg { "-" } else { "" }, x)
    };

    for v in mem.iter() {
        writeln!(file, "{}", to_str(*v)).unwrap();
    }
    for _ in mem.len()..65536 {
        writeln!(file, "0").unwrap();
    }
    file.flush().unwrap();

    Ok(())
}
