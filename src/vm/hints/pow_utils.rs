use crate::bigint;
use crate::serde::deserialize_program::ApTracking;
use crate::vm::errors::vm_errors::VirtualMachineError;
use crate::vm::vm_core::VMProxy;
use num_bigint::BigInt;
use num_integer::Integer;
use std::collections::HashMap;

use super::hint_utils::{get_relocatable_from_var_name, insert_value_from_var_name};

/*
Implements hint:
%{ ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1 %}
*/
pub fn pow(
    vm_proxy: &mut VMProxy,
    ids: &HashMap<String, usize>,
    hint_ap_tracking: Option<&ApTracking>,
) -> Result<(), VirtualMachineError> {
    let prev_locs_addr =
        get_relocatable_from_var_name("prev_locs", ids, vm_proxy, hint_ap_tracking)?;
    let prev_locs_exp = vm_proxy.memory.get_integer(&(&prev_locs_addr + 4))?;
    let locs_bit = prev_locs_exp.mod_floor(vm_proxy.prime) & bigint!(1);
    insert_value_from_var_name("locs", locs_bit, ids, vm_proxy, hint_ap_tracking)?;
    Ok(())
}

#[cfg(test)]
mod tests {

    use crate::types::instruction::Register;
    use crate::types::relocatable::MaybeRelocatable;
    use crate::utils::test_utils::*;
    use crate::vm::errors::memory_errors::MemoryError;
    use crate::vm::hints::execute_hint::{get_vm_proxy, BuiltinHintExecutor, HintReference};
    use crate::vm::vm_core::VirtualMachine;
    use crate::vm::vm_memory::memory::Memory;
    use crate::{bigint, vm::runners::builtin_runner::RangeCheckBuiltinRunner};
    use num_bigint::{BigInt, Sign};

    use super::*;

    static HINT_EXECUTOR: BuiltinHintExecutor = BuiltinHintExecutor {};
    use crate::types::hint_executor::HintExecutor;

    #[test]
    fn run_pow_ok() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        for _ in 0..3 {
            vm.segments.add(&mut vm.memory, None);
        }

        //Initialize ap
        vm.run_context.ap = MaybeRelocatable::from((1, 12));

        //Create ids
        let ids = ids!["prev_locs", "locs"];

        //Create references
        vm.references = HashMap::from([
            (
                0,
                HintReference {
                    dereference: true,
                    register: Register::AP,
                    offset1: -5,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: Some(ApTracking {
                        group: 4,
                        offset: 3,
                    }),
                    immediate: None,
                },
            ),
            (
                1,
                HintReference {
                    dereference: true,
                    register: Register::AP,
                    offset1: 0,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: Some(ApTracking {
                        group: 4,
                        offset: 3,
                    }),
                    immediate: None,
                },
            ),
        ]);

        //Insert ids.prev_locs.exp into memory
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 10)),
                &MaybeRelocatable::from(bigint!(3)),
            )
            .unwrap();

        let ap_tracking = ApTracking {
            group: 4,
            offset: 4,
        };
        let mut vm_proxy = get_vm_proxy(&mut vm);
        //Execute the hint
        assert_eq!(
            HINT_EXECUTOR.execute_hint(&mut vm_proxy, hint_code, &ids, &ap_tracking),
            Ok(())
        );

        //Check hint memory inserts
        assert_eq!(
            vm.memory.get(&MaybeRelocatable::from((1, 11))),
            Ok(Some(&MaybeRelocatable::from(bigint!(1))))
        );
    }

    #[test]
    fn run_pow_incorrect_ids() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        for _ in 0..3 {
            vm.segments.add(&mut vm.memory, None);
        }

        //Initialize ap
        vm.run_context.ap = MaybeRelocatable::from((1, 11));

        //Create incorrect ids
        let ids = ids!["locs"];

        let ap_tracking: ApTracking = ApTracking::new();
        let mut vm_proxy = get_vm_proxy(&mut vm);
        //Execute the hint
        assert_eq!(
            HINT_EXECUTOR.execute_hint(&mut vm_proxy, hint_code, &ids, &ap_tracking),
            Err(VirtualMachineError::FailedToGetIds)
        );
    }

    #[test]
    fn run_pow_incorrect_references() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        for _ in 0..3 {
            vm.segments.add(&mut vm.memory, None);
        }

        //Initialize fp
        vm.run_context.fp = MaybeRelocatable::from((1, 11));

        //Create ids
        let ids = ids!["prev_locs", "locs"];

        //Create incorrect references
        vm.references = HashMap::from([
            (0, HintReference::new_simple(-5)),
            // Incorrect reference, offset1 out of range
            (1, HintReference::new_simple(-12)),
        ]);

        let ap_tracking: ApTracking = ApTracking::new();
        let mut vm_proxy = get_vm_proxy(&mut vm);
        //Execute the hint
        assert_eq!(
            HINT_EXECUTOR.execute_hint(&mut vm_proxy, hint_code, &ids, &ap_tracking),
            Err(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 10))
            ))
        );
    }

    #[test]
    fn run_pow_prev_locs_exp_is_not_integer() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        //Initialize fp
        vm.run_context.fp = MaybeRelocatable::from((1, 11));

        //Create ids
        let ids = ids!["prev_locs", "locs"];

        //Create references
        vm.references = HashMap::from([
            (0, HintReference::new_simple(-5)),
            (1, HintReference::new_simple(0)),
        ]);

        //Insert ids.prev_locs.exp into memory as a RelocatableValue
        vm.memory = memory![((1, 10), (1, 11))];
        vm.segments.add(&mut vm.memory, None);
        //Execute the hint
        let mut vm_proxy = get_vm_proxy(&mut vm);
        assert_eq!(
            HINT_EXECUTOR.execute_hint(&mut vm_proxy, hint_code, &ids, &ApTracking::new()),
            Err(VirtualMachineError::ExpectedInteger(
                MaybeRelocatable::from((1, 10))
            ))
        );
    }

    #[test]
    fn run_pow_invalid_memory_insert() {
        let hint_code = "ids.locs.bit = (ids.prev_locs.exp % PRIME) & 1";
        let mut vm = vm_with_range_check!();
        for _ in 0..3 {
            vm.segments.add(&mut vm.memory, None);
        }

        //Initialize ap
        vm.run_context.ap = MaybeRelocatable::from((1, 11));

        //Create ids
        let ids = ids!["prev_locs", "locs"];

        //Create references
        vm.references = HashMap::from([
            (
                0,
                HintReference {
                    dereference: true,
                    register: Register::AP,
                    offset1: -5,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: Some(ApTracking::new()),
                    immediate: None,
                },
            ),
            (
                1,
                HintReference {
                    dereference: true,
                    register: Register::AP,
                    offset1: 0,
                    offset2: 0,
                    inner_dereference: false,
                    ap_tracking_data: Some(ApTracking::new()),
                    immediate: None,
                },
            ),
        ]);

        //Insert ids.prev_locs.exp into memory
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 10)),
                &MaybeRelocatable::from(bigint!(3)),
            )
            .unwrap();

        // Insert ids.locs.bit before the hint execution, so the hint memory.insert fails
        vm.memory
            .insert(
                &MaybeRelocatable::from((1, 11)),
                &MaybeRelocatable::from(bigint!(3)),
            )
            .unwrap();

        let ap_tracking: ApTracking = ApTracking::new();
        let mut vm_proxy = get_vm_proxy(&mut vm);
        //Execute the hint
        assert_eq!(
            HINT_EXECUTOR.execute_hint(&mut vm_proxy, hint_code, &ids, &ap_tracking),
            Err(VirtualMachineError::MemoryError(
                MemoryError::InconsistentMemory(
                    MaybeRelocatable::from((1, 11)),
                    MaybeRelocatable::from(bigint!(3)),
                    MaybeRelocatable::from(bigint!(1))
                )
            ))
        );
    }
}