use crate::stdlib::rc::Rc;
use datex_core::runtime::RuntimeInternal;
use crate::global::operators::binary::{
    ArithmeticOperator, BitwiseOperator, LogicalOperator,
};
use crate::global::operators::{
    ArithmeticUnaryOperator, AssignmentOperator, BinaryOperator,
    ComparisonOperator, LogicalUnaryOperator, ReferenceUnaryOperator,
    UnaryOperator,
};
use crate::references::reference::Reference;
use crate::runtime::execution::ExecutionError;
use crate::traits::identity::Identity;
use crate::traits::structural_eq::StructuralEq;
use crate::traits::value_eq::ValueEq;
use crate::values::value_container::{OwnedValueKey, ValueContainer};

pub fn set_property(
    runtime_internal: &Option<Rc<RuntimeInternal>>,
    target: &mut ValueContainer,
    key: OwnedValueKey,
    value: ValueContainer,
) -> Result<(), ExecutionError> {
    if let Some(runtime) = runtime_internal {
        target.try_set_property(
            0, // TODO: set correct source id
            &runtime.memory,
            key,
            value
        )?;
        Ok(())
    } else {
        Err(ExecutionError::RequiresRuntime)
    }
}

pub fn handle_unary_reference_operation(
    operator: ReferenceUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    Ok(match operator {
        ReferenceUnaryOperator::CreateRef => {
            ValueContainer::Reference(Reference::from(value_container))
        }
        ReferenceUnaryOperator::CreateRefMut => {
            ValueContainer::Reference(Reference::try_mut_from(value_container)?)
        }
        ReferenceUnaryOperator::Deref => {
            if let ValueContainer::Reference(reference) = value_container {
                reference.value_container()
            } else {
                return Err(ExecutionError::DerefOfNonReference);
            }
        }
    })
}
pub fn handle_unary_logical_operation(
    operator: LogicalUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    unimplemented!(
        "Logical unary operations are not implemented yet: {operator:?}"
    )
}
pub fn handle_unary_arithmetic_operation(
    operator: ArithmeticUnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        ArithmeticUnaryOperator::Minus => Ok((-value_container)?),
        ArithmeticUnaryOperator::Plus => Ok(value_container),
        _ => unimplemented!(
            "Arithmetic unary operations are not implemented yet: {operator:?}"
        ),
    }
}

pub fn handle_unary_operation(
    operator: UnaryOperator,
    value_container: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        UnaryOperator::Reference(reference) => {
            handle_unary_reference_operation(reference, value_container)
        }
        UnaryOperator::Logical(logical) => {
            handle_unary_logical_operation(logical, value_container)
        }
        UnaryOperator::Arithmetic(arithmetic) => {
            handle_unary_arithmetic_operation(arithmetic, value_container)
        }
        _ => {
            core::todo!("#102 Unary instruction not implemented: {operator:?}")
        }
    }
}

pub fn handle_comparison_operation(
    operator: ComparisonOperator,
    lhs: &ValueContainer,
    rhs: &ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ComparisonOperator::StructuralEqual => {
            let val = lhs.structural_eq(rhs);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Equal => {
            let val = lhs.value_eq(rhs);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotStructuralEqual => {
            let val = !lhs.structural_eq(rhs);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::NotEqual => {
            let val = !lhs.value_eq(rhs);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Is => {
            // TODO #103 we should throw a runtime error when one of lhs or rhs is a value
            // instead of a ref. Identity checks using the is operator shall be only allowed
            // for references.
            // @benstre: or keep as always false ? - maybe a compiler check would be better
            let val = lhs.identical(rhs);
            Ok(ValueContainer::from(val))
        }
        ComparisonOperator::Matches => {
            // TODO #407: Fix matches, rhs will always be a type, so actual_type() call is wrong
            let v_type = rhs.actual_container_type(); // Type::try_from(value_container)?;
            let val = v_type.value_matches(lhs);
            Ok(ValueContainer::from(val))
        }
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

pub fn handle_assignment_operation(
    operator: AssignmentOperator,
    lhs: &ValueContainer,
    rhs: ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        AssignmentOperator::AddAssign => Ok((lhs + &rhs)?),
        AssignmentOperator::SubtractAssign => Ok((lhs - &rhs)?),
        _ => {
            unreachable!("Instruction {:?} is not a valid operation", operator);
        }
    }
}

pub fn handle_arithmetic_operation(
    operator: ArithmeticOperator,
    lhs: &ValueContainer,
    rhs: &ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    match operator {
        ArithmeticOperator::Add => Ok((lhs + rhs)?),
        ArithmeticOperator::Subtract => Ok((lhs - rhs)?),
        // ArithmeticOperator::Multiply => {
        //     Ok((active_value_container * &value_container)?)
        // }
        // ArithmeticOperator::Divide => {
        //     Ok((active_value_container / &value_container)?)
        // }
        _ => {
            core::todo!(
                "#408 Implement arithmetic operation for {:?}",
                operator
            );
        }
    }
}

pub fn handle_bitwise_operation(
    operator: BitwiseOperator,
    lhs: &ValueContainer,
    rhs: &ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#409 Implement bitwise operation for {:?}", operator);
    }
}

pub fn handle_logical_operation(
    operator: LogicalOperator,
    lhs: &ValueContainer,
    rhs: &ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    // apply operation to active value
    {
        core::todo!("#410 Implement logical operation for {:?}", operator);
    }
}

pub fn handle_binary_operation(
    operator: BinaryOperator,
    lhs: &ValueContainer,
    rhs: &ValueContainer,
) -> Result<ValueContainer, ExecutionError> {
    match operator {
        BinaryOperator::Arithmetic(arith_op) => {
            handle_arithmetic_operation(arith_op, lhs, rhs)
        }
        BinaryOperator::Bitwise(bitwise_op) => {
            handle_bitwise_operation(bitwise_op, lhs, rhs)
        }
        BinaryOperator::Logical(logical_op) => {
            handle_logical_operation(logical_op, lhs, rhs)
        }
    }
}
