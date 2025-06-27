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


## Update the Parser
Next, you need to update the parser to recognize your new token. This is done in the [`datex-core/src/compiler/parser.rs`](../../src/compiler/parser.rs) module. The parser is responsible for converting the sequence of tokens recognized by the lexer into an Abstract Syntax Tree (AST).


### Extend the expression grammar
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

### Extend the parser logic
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