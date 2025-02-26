use crate::stdlib::prelude::*;

use crate::hint_processor::builtin_hint_processor::keccak_utils::left_pad_u64;
use crate::math_utils::safe_div_usize;
use crate::types::instance_definitions::keccak_instance_def::KeccakInstanceDef;
use crate::types::relocatable::{MaybeRelocatable, Relocatable};
use crate::vm::errors::memory_errors::{InsufficientAllocatedCellsError, MemoryError};
use crate::vm::errors::runner_errors::RunnerError;
use crate::vm::vm_core::VirtualMachine;
use crate::vm::vm_memory::memory::Memory;
use crate::vm::vm_memory::memory_segments::MemorySegmentManager;
use felt::Felt;
use num_integer::div_ceil;
use num_traits::{One, ToPrimitive};

use super::KECCAK_BUILTIN_NAME;

const KECCAK_ARRAY_LEN: usize = 25;

#[derive(Debug, Clone)]
pub struct KeccakBuiltinRunner {
    ratio: u32,
    pub base: usize,
    pub(crate) cells_per_instance: u32,
    pub(crate) n_input_cells: u32,
    verified_addresses: Vec<Relocatable>,
    pub(crate) stop_ptr: Option<usize>,
    pub(crate) included: bool,
    state_rep: Vec<u32>,
    instances_per_component: u32,
}

impl KeccakBuiltinRunner {
    pub(crate) fn new(instance_def: &KeccakInstanceDef, included: bool) -> Self {
        KeccakBuiltinRunner {
            base: 0,
            ratio: instance_def._ratio,
            n_input_cells: instance_def._state_rep.len() as u32,
            cells_per_instance: instance_def.cells_per_builtin(),
            stop_ptr: None,
            verified_addresses: Vec::new(),
            included,
            instances_per_component: instance_def._instance_per_component,
            state_rep: instance_def._state_rep.clone(),
        }
    }

    pub fn initialize_segments(&mut self, segments: &mut MemorySegmentManager) {
        self.base = segments.add().segment_index as usize // segments.add() always returns a positive index
    }

    pub fn initial_stack(&self) -> Vec<MaybeRelocatable> {
        if self.included {
            vec![MaybeRelocatable::from((self.base as isize, 0))]
        } else {
            vec![]
        }
    }

    pub fn base(&self) -> usize {
        self.base
    }

    pub fn ratio(&self) -> u32 {
        self.ratio
    }

    pub fn add_validation_rule(&self, _memory: &mut Memory) {}

    pub fn deduce_memory_cell(
        &self,
        address: Relocatable,
        memory: &Memory,
    ) -> Result<Option<MaybeRelocatable>, RunnerError> {
        let index = address.offset % self.cells_per_instance as usize;
        if index < self.n_input_cells as usize {
            return Ok(None);
        }

        let first_input_addr = (address - index).map_err(|_| RunnerError::KeccakNoFirstInput)?;
        if self.verified_addresses.contains(&first_input_addr) {
            return Ok(None);
        }

        let mut input_felts_u64 = vec![];

        for i in 0..self.n_input_cells {
            let val = match memory.get(&(first_input_addr + i as usize)?) {
                Some(val) => val
                    .as_ref()
                    .get_int_ref()
                    .and_then(|x| x.to_u64())
                    .ok_or(RunnerError::KeccakInputCellsNotU64)?,
                _ => return Ok(None),
            };

            input_felts_u64.push(val)
        }

        if let Some((i, bits)) = self.state_rep.iter().enumerate().next() {
            let val = memory.get_integer((first_input_addr + i)?)?;
            if val.as_ref() >= &(Felt::one() << *bits) {
                return Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                    (first_input_addr + i)?.into(),
                    *bits,
                    val.into_owned(),
                ));
            }

            let len = input_felts_u64.len();
            let mut input_felts_u64 = left_pad_u64(&mut input_felts_u64, KECCAK_ARRAY_LEN - len)
                .try_into()
                .map_err(|_| RunnerError::SliceToArrayError)?;

            keccak::f1600(&mut input_felts_u64);

            return Ok(input_felts_u64
                .get(address.offset - 1)
                .map(|x| Felt::from(*x).into()));
        }
        Ok(None)
    }

    pub fn get_allocated_memory_units(&self, vm: &VirtualMachine) -> Result<usize, MemoryError> {
        let value = safe_div_usize(vm.current_step, self.ratio as usize)
            .map_err(|_| MemoryError::ErrorCalculatingMemoryUnits)?;
        Ok(self.cells_per_instance as usize * value)
    }

    pub fn get_memory_segment_addresses(&self) -> (usize, Option<usize>) {
        (self.base, self.stop_ptr)
    }

    pub fn get_used_cells(&self, segments: &MemorySegmentManager) -> Result<usize, MemoryError> {
        segments
            .get_segment_used_size(self.base())
            .ok_or(MemoryError::MissingSegmentUsedSizes)
    }

    pub fn get_used_cells_and_allocated_size(
        &self,
        vm: &VirtualMachine,
    ) -> Result<(usize, usize), MemoryError> {
        let ratio = self.ratio as usize;
        let min_step = ratio * self.instances_per_component as usize;
        if vm.current_step < min_step {
            Err(
                InsufficientAllocatedCellsError::MinStepNotReached(min_step, KECCAK_BUILTIN_NAME)
                    .into(),
            )
        } else {
            let used = self.get_used_cells(&vm.segments)?;
            let size = self.cells_per_instance as usize
                * safe_div_usize(vm.current_step, ratio).map_err(|_| {
                    InsufficientAllocatedCellsError::CurrentStepNotDivisibleByBuiltinRatio(
                        KECCAK_BUILTIN_NAME,
                        vm.current_step,
                        ratio,
                    )
                })?;
            if used > size {
                return Err(InsufficientAllocatedCellsError::BuiltinCells(
                    KECCAK_BUILTIN_NAME,
                    used,
                    size,
                )
                .into());
            }
            Ok((used, size))
        }
    }

    pub fn get_used_instances(
        &self,
        segments: &MemorySegmentManager,
    ) -> Result<usize, MemoryError> {
        let used_cells = self.get_used_cells(segments)?;
        Ok(div_ceil(used_cells, self.cells_per_instance as usize))
    }

    pub fn final_stack(
        &mut self,
        segments: &MemorySegmentManager,
        pointer: Relocatable,
    ) -> Result<Relocatable, RunnerError> {
        if self.included {
            let stop_pointer_addr =
                (pointer - 1).map_err(|_| RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))?;
            let stop_pointer = segments
                .memory
                .get_relocatable(stop_pointer_addr)
                .map_err(|_| RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))?;
            if self.base as isize != stop_pointer.segment_index {
                return Err(RunnerError::InvalidStopPointerIndex(
                    KECCAK_BUILTIN_NAME,
                    stop_pointer,
                    self.base,
                ));
            }
            let stop_ptr = stop_pointer.offset;
            let num_instances = self.get_used_instances(segments)?;
            let used = num_instances * self.cells_per_instance as usize;
            if stop_ptr != used {
                return Err(RunnerError::InvalidStopPointer(
                    KECCAK_BUILTIN_NAME,
                    Relocatable::from((self.base as isize, used)),
                    Relocatable::from((self.base as isize, stop_ptr)),
                ));
            }
            self.stop_ptr = Some(stop_ptr);
            Ok(stop_pointer_addr)
        } else {
            let stop_ptr = self.base;
            self.stop_ptr = Some(stop_ptr);
            Ok(pointer)
        }
    }

    pub fn get_memory_accesses(
        &self,
        vm: &VirtualMachine,
    ) -> Result<Vec<Relocatable>, MemoryError> {
        let segment_size = vm
            .segments
            .get_segment_size(self.base)
            .ok_or(MemoryError::MissingSegmentUsedSizes)?;

        Ok((0..segment_size)
            .map(|i| (self.base as isize, i).into())
            .collect())
    }

    pub fn get_used_diluted_check_units(&self, diluted_n_bits: u32) -> usize {
        // The diluted cells are:
        // state - 25 rounds times 1600 elements.
        // parity - 24 rounds times 1600/5 elements times 3 auxiliaries.
        // after_theta_rho_pi - 24 rounds times 1600 elements.
        // theta_aux - 24 rounds times 1600 elements.
        // chi_iota_aux - 24 rounds times 1600 elements times 2 auxiliaries.
        // In total 25 * 1600 + 24 * 320 * 3 + 24 * 1600 + 24 * 1600 + 24 * 1600 * 2 = 216640.
        // But we actually allocate 4 virtual columns, of dimensions 64 * 1024, in which we embed the
        // real cells, and we don't free the unused ones.
        // So the real number is 4 * 64 * 1024 = 262144.
        safe_div_usize(262144_usize, diluted_n_bits as usize).unwrap_or(0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hint_processor::builtin_hint_processor::builtin_hint_processor_definition::BuiltinHintProcessor;
    use crate::relocatable;
    use crate::stdlib::collections::HashMap;
    use crate::types::program::Program;
    use crate::utils::test_utils::*;
    use crate::vm::runners::cairo_runner::CairoRunner;
    use crate::vm::vm_memory::memory::Memory;
    use crate::vm::{
        errors::{memory_errors::MemoryError, runner_errors::RunnerError},
        runners::builtin_runner::BuiltinRunner,
        vm_core::VirtualMachine,
    };

    #[cfg(target_arch = "wasm32")]
    use wasm_bindgen_test::*;

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_instances() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true).into();

        let mut vm = vm!();
        vm.segments.segment_used_sizes = Some(vec![1]);

        assert_eq!(builtin.get_used_instances(&vm.segments), Ok(1));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack() {
        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 1))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_stop_pointer() {
        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![992]);

        let pointer = Relocatable::from((2, 2));
        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::InvalidStopPointer(
                KECCAK_BUILTIN_NAME,
                relocatable!(0, 992),
                relocatable!(0, 0)
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_when_not_included() {
        let mut builtin =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), false);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), (0, 0))
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer).unwrap(),
            Relocatable::from((2, 2))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn final_stack_error_non_relocatable() {
        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true);

        let mut vm = vm!();

        vm.segments = segments![
            ((0, 0), (0, 0)),
            ((0, 1), (0, 1)),
            ((2, 0), (0, 0)),
            ((2, 1), 2)
        ];

        vm.segments.segment_used_sizes = Some(vec![0]);

        let pointer = Relocatable::from((2, 2));

        assert_eq!(
            builtin.final_stack(&vm.segments, pointer),
            Err(RunnerError::NoStopPointer(KECCAK_BUILTIN_NAME))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_and_allocated_size_test() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true).into();

        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        let program = Program::from_bytes(
            include_bytes!("../../../../cairo_programs/_keccak.json"),
            Some("main"),
        )
        .unwrap();

        let mut cairo_runner = cairo_runner!(program, "all");

        let mut hint_processor = BuiltinHintProcessor::new_empty();

        let address = cairo_runner.initialize(&mut vm).unwrap();

        cairo_runner
            .run_until_pc(address, &mut vm, &mut hint_processor)
            .unwrap();

        assert_eq!(
            builtin.get_used_cells_and_allocated_size(&vm),
            Ok((0, 1072))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_allocated_memory_units() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::new(10, vec![200; 8]), true).into();

        let mut vm = vm!();
        vm.current_step = 10;

        assert_eq!(builtin.get_allocated_memory_units(&vm), Ok(16));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_segment_addresses() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        assert_eq!(builtin.get_memory_segment_addresses(), (0, None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_missing_segment_used_sizes() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let vm = vm!();

        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Err(MemoryError::MissingSegmentUsedSizes),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses_empty() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_memory_accesses(&vm), Ok(vec![]));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_memory_accesses() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(
            builtin.get_memory_accesses(&vm),
            Ok(vec![
                (builtin.base() as isize, 0).into(),
                (builtin.base() as isize, 1).into(),
                (builtin.base() as isize, 2).into(),
                (builtin.base() as isize, 3).into(),
            ]),
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_missing_segment_used_sizes() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let vm = vm!();

        assert_eq!(
            builtin.get_used_cells(&vm.segments),
            Err(MemoryError::MissingSegmentUsedSizes)
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells_empty() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![0]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(0));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_cells() {
        let builtin: BuiltinRunner =
            KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true).into();
        let mut vm = vm!();

        vm.segments.segment_used_sizes = Some(vec![4]);
        assert_eq!(builtin.get_used_cells(&vm.segments), Ok(4));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stackincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        assert_eq!(
            keccak_builtin.initial_stack(),
            vec![mayberelocatable!(0, 0)]
        )
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn initial_stack_notincluded_test() {
        let keccak_builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), false);
        assert_eq!(keccak_builtin.initial_stack(), Vec::new())
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_valid() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);
        assert_eq!(
            result,
            Ok(Some(MaybeRelocatable::from(Felt::new(
                3086936446498698982_u64
            ))))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_non_reloc_address_err() {
        let memory = memory![
            ((0, 4), 32),
            ((0, 5), 72),
            ((0, 6), 0),
            ((0, 7), 120),
            ((0, 8), 52)
        ];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 1)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_offset_lt_input_cell_length_none() {
        let memory = memory![((0, 4), 32)];
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);
        let result = builtin.deduce_memory_cell(Relocatable::from((0, 2)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_offset_first_addr_error() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];

        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        builtin.verified_addresses.push(Relocatable::from((0, 16)));

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);
        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_expected_integer() {
        let memory = memory![((0, 0), (1, 2))];

        let mut builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        builtin.n_input_cells = 0;
        builtin.cells_per_instance = 100;

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 99)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::Memory(MemoryError::ExpectedInteger(
                (0, 0).into()
            )))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_get_memory_err() {
        let memory = memory![((0, 35), 0)];

        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 15)), &memory);

        assert_eq!(result, Ok(None));
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn deduce_memory_cell_memory_int_larger_than_bits() {
        let memory = memory![
            ((0, 16), 43),
            ((0, 17), 199),
            ((0, 18), 0),
            ((0, 19), 0),
            ((0, 20), 0),
            ((0, 21), 0),
            ((0, 22), 0),
            ((0, 23), 1),
            ((0, 24), 0),
            ((0, 25), 0),
            ((0, 26), 43),
            ((0, 27), 199),
            ((0, 28), 0),
            ((0, 29), 0),
            ((0, 30), 0),
            ((0, 31), 0),
            ((0, 32), 0),
            ((0, 33), 1),
            ((0, 34), 0),
            ((0, 35), 0)
        ];

        let keccak_instance = KeccakInstanceDef::new(2048, vec![1; 8]);
        let builtin = KeccakBuiltinRunner::new(&keccak_instance, true);

        let result = builtin.deduce_memory_cell(Relocatable::from((0, 25)), &memory);

        assert_eq!(
            result,
            Err(RunnerError::IntegerBiggerThanPowerOfTwo(
                (0, 16).into(),
                1,
                43.into()
            ))
        );
    }

    #[test]
    #[cfg_attr(target_arch = "wasm32", wasm_bindgen_test)]
    fn get_used_diluted_check_units_result() {
        let builtin = KeccakBuiltinRunner::new(&KeccakInstanceDef::default(), true);

        let result: usize = builtin.get_used_diluted_check_units(16);

        assert_eq!(result, 16384);
    }
}
