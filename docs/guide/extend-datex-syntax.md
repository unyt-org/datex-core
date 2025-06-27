# Extend the DATEX syntax
> This guide explains how to extend the DATEX syntax by giving a step-by-step approach to adding new syntax elements (such as keywords, operators or expressions) and their corresponding functionality.

## Define the Token
If you want to add a new keyword or operator, you need to define it in the `Token` enum. This is done in the [`datex-core/src/compiler/lexer.rs`](../../src/compiler/lexer.rs) file.
Add a new entry to the `Token` enum. For example, if you want to add a new operator called `Is`, you would add:

```rust
pub enum Token {
    // existing tokens...
    #[token("is")] Is,
    // other tokens...
}
```

Make sure to add a new test for your new token in the `tests` section of the lexer to ensure it is recognized correctly by the lexer.
```rust
#[test]
fn test_is_operator() {
    let mut lexer = Token::lexer("a is b");
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::Identifier("a".to_string()))
    );
    assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
    assert_eq!(lexer.next().unwrap(), Ok(Token::Is));
    assert_eq!(lexer.next().unwrap(), Ok(Token::Whitespace));
    assert_eq!(
        lexer.next().unwrap(),
        Ok(Token::Identifier("b".to_string()))
    );
    assert_eq!(lexer.next(), None);
}
```

## Introduce an Instruction Code
Not all new tokens require a new instruction code, but if your new syntax element is a keyword and can not be represented by a set of existing instruction codes, you need to introduce a new instruction code in the [`datex-core/src/global/binary_codes.rs`](../../src/global/binary_codes.rs) file.

```rust
pub enum InstructionCode {
    // existing instruction codes...
    IS, // Checks if two references are identity equal
    // other instruction codes...
}
```

To allow for serialization of the new instruction and it's potential payload, it might be necessary to add a new entry to the `Instruction` enum in the [`datex-core/src/global/protocol_structures/instructions.rs`](../../src/global/protocol_structures/instructions.rs) file:

```rust
pub enum Instruction {
    True, // implicitly true
    Int8(Int8Data), // holds an 8-bit integer

    Is, // New instruction for the `is` operator

    // other instructions...
```

*Note that also the `Display for Instruction` must be updated to include the new instruction code.*

To map `InstructionCode` to the `Instruction` holder, you need to add a new match arm in the `iterate_instruction` method in the [`datex-core/src/parser/body.rs`](../../src/parser/body.rs) file:

```rust
yield match instruction_code {
    // existing instruction codes...
    InstructionCode::IS => Ok(Instruction::Is),
    // other instruction codes...
}
```

## Update the Parser
Next, you need to update the parser to recognize your new token. This is done in the [`datex-core/src/compiler/parser.rs`](../../src/compiler/parser.rs) module. The parser is responsible for converting the sequence of tokens recognized by the lexer into an Abstract Syntax Tree (AST).


### Extend the Expression Grammar
We have to extend the expression grammar to include our expression or operator. This is done by adding a new entry or modifying an existing one in the `DatexExpression` enum.

```rust
pub enum DatexExpression {
    /// Only keywords holding no value, e.g. `null`, `if`, `else`, `while`
    Null,
    /// Custom data type holding a Rust value
    Text(String),
    
    /// Custom data type holding a vector of expressions
    Array(Vec<DatexExpression>),

    /// Custom syntax to represent variable declarations (e.g. `val x = 5`)
    ///                 val           x       DatexExpression<5>
    VariableDeclaration(VariableType, String, Box<DatexExpression>),
}
```

We have to declare an entry here that can hold all parts of our expression if it is more complex than a simple value. 
In our case it's more easy, as we just want to add a new operator `is`, which can be represented as a binary operation, and the holder is already present in the `DatexExpression` enum as `BinaryOperation(BinaryOperator, Box<DatexExpression>, Box<DatexExpression>)`. So here we extend the `BinaryOperator` enum to include our new operator:

```rust
pub enum BinaryOperator {
    // existing operators...
    Is, // New operator
}
```

### Extend the Parser Logic
When extending the parser with new operators (e.g. `is`), **it's essential to add them at the correct precedence level**. Operator precedence determines **the order in which expressions are grouped and evaluated**, especially when multiple operators appear in a row.

Without correct precedence, expressions can be **parsed incorrectly**, leading to **wrong evaluation or misleading decompilation**.

Take this example:

```datex
2 is 4 + 4
```

This should be understood as:

```text
2 is (4 + 4)
    → Is(2, Add(4, 4))
        → false
```

…but if you place the `is` operator at the wrong precedence level (e.g. equal to or higher than `+`), the parser might instead treat it as:

```text
(2 is 4) + 4
    → Add(Is(2, 4), 4)
        → (false + 4)
             → ❌
```

When you introduce a new operator:

1. **Find the correct precedence level** in the parser (e.g., sum, product, comparison).
2. **Add it to that layer** using `.foldl(...)` or `.foldr(...)` as needed.
3. Make sure lower-precedence expressions (like comparisons) are parsed *after* higher-precedence ones (like addition/multiplication).

#### Operator Precedence Table
| Level                                  | What it recognises                                             | Key helpers used                                                      | Notes                                                                                                                                                                      |
| -------------------------------------- | -------------------------------------------------------------- | --------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Script**                             | Whole input file                                               | `statements` or *empty*                                               | Either a non-empty sequence of statements **or** any number of stray semicolons (⇒ an empty script).                                                                       |
| **Statements**                         | `statement ;` … `statement [;]`                                | `expression`, `Semicolon`                                             | Builds a `Vec<Statement>` where every entry tracks whether its terminating semicolon was present. A lone unterminated expression collapses straight to an expression node. |
| **Expression**                         | Top-level value                                                | `tuple` \| `expression_without_tuple`                                 | Wrapper that lets tuples sit beside the rest of the expression grammar.                                                                                                    |
| **Expression (no-tuple)**              | All expressions except tuples                                  | `variable_assignment` \| `equality`                                   | Declared recursively so tuple sub-parses don’t accidentally nest ad infinitum.                                                                                             |
| **Variable assignment / declaration**  | `val x = …`, `ref x = …`, **or** `x = …`                       | `ValKW`, `RefKW`, `Assign`                                            | Emits either `VariableDeclaration` or `VariableAssignment`.                                                                                                                |
| **Equality chain**                     | `a == b`, `a != b`, `a == b == c`, …                           | `sum`, comparison tokens                                              | Supports structural/strict equality (and negations) plus `is`. Multiple comparisons in a row fold left-to-right.                                                           |
| **Sum / Product**                      | `+`, `–` operations; `*`, `/`                                  | `product`, `apply_or_property_access`                                 | Classic left-associative arithmetic precedence.                                                                                                                            |
| **Apply / Property-access chain**      | Function calls and dotted look-ups (`f (x)`, `obj.prop`)       | `atom`, plus `Apply::FunctionCall`/`PropertyAccess`                   | Any number of calls/accesses fold into one `ApplyChain`.                                                                                                                   |
| **Atom**                               | Literals, arrays, objects, parenthesised or nested expressions | `integer`, `decimal`, `text`, `array`, `object`, `wrapped_expression` | Smallest “self-contained” chunk the higher levels operate on.                                                                                                              |
| **Array**                              | `[expr, …]`                                                    | `expression_without_tuple`                                            | Allows 0-N elements, trailing comma OK.                                                                                                                                    |
| **Object**                             | `{key: value, …}`                                              | `key` + value parser                                                  | Keys may be strings, numbers, identifiers or arbitrary expressions `(expr)`.                                                                                               |
| **Tuple**                              | `(a, b)`, `(key: v, …)`                                        | `tuple_entry`                                                         | Three variants ensure **(i)** at least two items, **(ii)** single value plus comma, **(iii)** single `key: value`.                                                         |
| **Key** (for objects/tuples)           | Text, numeric, identifier, or expression                       | `key` selector                                                        | Supports dynamic keys via `(expr)`.                                                                                                                                        |
| **Integer / Decimal / Text / Literal** | Primitive constants                                            | token selectors                                                       | Each maps the raw token into the corresponding `DatexExpression` variant.                                                                                                  |
| **Whitespace**                         | Ignorable gaps                                                 | `Token::Whitespace`                                                   | Centralised so every combinator can use `.padded_by(whitespace.clone())`.                                                                                                  |

The pipeline top-down looks something like this:

```
script
└─ statements
   └─ expression
      ├─ tuple
      │    └─ tuple_entry (value | key:value)
      └─ expression_without_tuple
           ├─ variable_assignment | equality
           │    └─ sum
           │        └─ product
           │            └─ apply/property
           │                └─ atom
           │                    └─ literals / array / object / (expr)
```

For the `is` operator, you would add it to the equality chain section of the `create_parser` method: 
```rust
let equality = sum.clone().foldl(
    choice((
        // Existing operators
        // ...

        // Our new operator
        op(Token::Is) //  is
            .to(binary_op(BinaryOperator::Is)),
    ))
    .then(sum)
    .repeated(),
    |lhs, (op, rhs)| op(Box::new(lhs), Box::new(rhs)),
);
```

If you've introduced a new `DatexExpression` and not a simple operator, you need to extend the parser logic to account for it and map it to the corresponding `DatexExpression` variant and add the token branch handler to the correct level.

```rust
let custom_expression = select! {
    Token::Whatever(s) => DatexExpression::YourExpression(&s)
};

// E.g. if Token::Whatever is an atom, then it must be added to this level:
let atom = choice((
    // Existing atom parsers...
    custom_expression.clone(),
))
.boxed();
```

### Add a Test
Finally, you should add a test to ensure that your new syntax element is parsed correctly. This is done in the [`datex-core/src/compiler/parser.rs`](../../src/compiler/parser.rs) module.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_custom_expression() {
        let input = "whatever";
        let expected = DatexExpression::YourExpression("whatever");
        let result = parse(input);
        assert_eq!(result, expected);
    }
}
```

In the case of the `is` operator, you would add a test like this:

```rust
#[test]
fn test_equal_operators() {
    let src = "5 is 1 + 2";
    let val = parse_unwrap(src);
    assert_eq!(
        val,
        DatexExpression::BinaryOperation(
            BinaryOperator::Is,
            Box::new(DatexExpression::Integer(Integer::from(5))),
            Box::new(DatexExpression::BinaryOperation(
                BinaryOperator::Add,
                Box::new(DatexExpression::Integer(Integer::from(1))),
                Box::new(DatexExpression::Integer(Integer::from(2)))
            ))
        )
    );
}
```

## Extend the Compiler
The next step is to extend the compiler to handle for the new syntax element. This is done in the [`datex-core/src/compiler/bytecode.rs`](../../src/compiler/bytecode.rs) module.

First of all we have to design how we want to represent our new syntax element in the bytecode. For the `is` operator, we will do something similar as we do for addition and other binary operations. The operator will be the first instruction, followed by the (two) operands.

```rust
/// 1 is 2
vec![
    InstructionCode::IS.into(),

    // 1
    InstructionCode::INT_8.into(),
    1,

    // 2
    InstructionCode::INT_8.into(),
    2
];
/// a is b
vec![
    InstructionCode::IS.into(),
    // a
    InstructionCode::GET_SLOT.into(),
    0,
    0,
    0,
    0, // slot address for a

    // b
    InstructionCode::GET_SLOT.into(),
    1,
    0,
    0,
    0, // slot address for b
];
```

Let's start by creating a test case for our new `is` operator in the `tests` section of the bytecode compiler:

```rust
#[test]
fn test_is_operator() {
    init_logger();

    let datex_script = format!("1 is 2");
    let result = compile_and_log(&datex_script);
    assert_eq!(
        result,
        vec![
            InstructionCode::IS.into(),
            InstructionCode::INT_8.into(),
            1,
            InstructionCode::INT_8.into(),
            2
        ]
    );
}
```

*Note that the syntax of the `compile_and_log` call must be valid DATEX syntax (except for now our new `is` operator, which is not yet implemented). Please also note that the `1 is 2` code snippet will most likely throw a compile error later, since the `is` operator will only be valid for reference identity checks and not for value equality, so the example of `1 is 2` is mainly to bring the point across with a not too complex example for this guide.*

To allow the `compile_expression` method to handle our new syntax, we have to add a new match arm for the `BinaryOperation` variant in the `compile_expression` method:

```rust
match ast {
    // existing match arms...
    DatexExpression::Whatever(..whatever) => {
        // Implement serialization logic for your new expression here
        // and add to the compilation_scope buffer
    }
}
```
If your new syntax element requires special serialization, such as representation as multiple instruction codes you can implement it inside of the match by modifing the `compilation_scope` buffer using the utility methods provided by the `CompilationScope` struct:

```rust
compilation_scope.append_binary_code(InstructionCode::WHATEVER);
compilation_scope.insert_value_container(...);
compilation_scope.insert_decimal(...);
// or similar
```

Since the `is` operator is a binary operation, we don't have to do any modification since the `DatexExpression::BinaryOperation` is already handled in the `compile_expression` method and we've already added the `BinaryOperator::Is` to the `BinaryOperator` enum.

Make sure that the test runs successfully and the bytecode is generated correctly.

## Extend the Runtime
Finally, you need to extend the runtime to handle the new syntax element. This is done in the [`datex-core/src/runtime/execution.rs`](../../src/runtime/execution.rs) module.
We'll start by adding a new test that calls the `execute_datex_script_debug_with_result` helper method with a simple `is` expression:

```rust
#[test]
fn test_is() {
    let result = execute_datex_script_debug_with_result("1 is 1");
    assert_eq!(result, true.into());
    assert_structural_eq!(result, ValueContainer::from(true));

    let result = execute_datex_script_debug_with_result("1 is 2");
    assert_eq!(result, false.into());
    assert_structural_eq!(result, ValueContainer::from(false));
}
```

We have to add a new match arm for the `Instruction::Is` instruction holder in the `execute_loop` method. Since out Instruction::Is doesn't hold any payload, we can simply match it and set the active operation to `Instruction::Is` in the `context.scope_stack` as we did for addition and other binary operations. This will allow us to handle the `is` operation in the next iteration of the execution loop, where we will have two operands available.

```rust
let value: ActiveValue = match instruction {
    // simple keywords to Rust / DATEX value
    Instruction::True => true.into(),

    // user defined values with payload to DATEX value
    Instruction::Int8(integer) => Integer::from(integer.0).into(),

    // operations
    Instruction::Is => {
        context.scope_stack.set_active_operation(Instruction::Is);
        ActiveValue::None // we will handle the is operation in the next iteration since we need two operands
    }

    // other instructions...
};
```

Since in this case we need two operands to perform the `is` operation, we will handle it in the next iteration of the execution loop. The `context.scope_stack` will hold the active operation and the operands will be available in the next iteration. So for operators we must also add a match arm for the `Instruction::Is` inside of the `handle_value` method, which will handle the actual operation:

```rust
let res = match operation {
    // existing operations...
    Instruction::Is => {
        let val = active_value_container
            .matches_identity(&value_container); // boolean
        Ok(ValueContainer::from(val)) // return the boolean result as a ValueContainer
    }
    // other operations...
}
```

## Conclusion
You have now successfully extended the DATEX syntax by adding a new keyword or operator, updating the parser, compiler, and runtime to handle it. Please make sure to run all tests to ensure that everything works as expected.

Please run `cargo bench` to ensure that the performance is still acceptable and that no performance regressions have been introduced.


---

<sub>&copy; unyt 2025 • [unyt.org](https://unyt.org)</sub>