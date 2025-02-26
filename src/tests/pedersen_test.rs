use crate::stdlib::prelude::*;

use crate::{
    hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor,
    types::program::Program,
    vm::trace::trace_entry::RelocatedTraceEntry,
    vm::{runners::cairo_runner::CairoRunner, vm_core::VirtualMachine},
};

use assert_matches::assert_matches;

#[cfg(target_arch = "wasm32")]
use wasm_bindgen_test::*;

#[test]
#[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
fn pedersen_integration_test() {
    let program = Program::from_bytes(
        include_bytes!("../../cairo_programs/pedersen_test.json"),
        Some("main"),
    )
    .unwrap();
    let mut hint_processor = BuiltinHintProcessor::new_empty();
    let mut cairo_runner = CairoRunner::new(&program, "all", false).unwrap();
    let mut vm = VirtualMachine::new(true);
    let end = cairo_runner.initialize(&mut vm).unwrap();
    assert_matches!(
        cairo_runner.run_until_pc(end, &mut vm, &mut hint_processor),
        Ok(())
    );
    assert!(cairo_runner.relocate(&mut vm) == Ok(()), "Execution failed");

    let python_vm_relocated_trace: Vec<RelocatedTraceEntry> = vec![
        RelocatedTraceEntry {
            pc: 7,
            ap: 25,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 8,
            ap: 26,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 10,
            ap: 27,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 12,
            ap: 28,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 1,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 2,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 3,
            ap: 30,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 5,
            ap: 31,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 6,
            ap: 32,
            fp: 30,
        },
        RelocatedTraceEntry {
            pc: 14,
            ap: 32,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 15,
            ap: 32,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 17,
            ap: 33,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 18,
            ap: 34,
            fp: 25,
        },
        RelocatedTraceEntry {
            pc: 19,
            ap: 35,
            fp: 25,
        },
    ];
    assert_eq!(
        cairo_runner.relocated_trace,
        Some(python_vm_relocated_trace)
    );
}
