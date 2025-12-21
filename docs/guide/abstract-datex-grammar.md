# Datex grammar abstract
---
> This might not be an exact representation. 

```
Token

Identifier
Number
String

"if" "else" "fn" "let"
";" "," "." "::" ".."
"(" ")" "[" "]" "{" "}"

"+" "-" "*" "/" "!"
"==" "!=" "is"
"="
```

---

```
DxScript
  ::= ";"*
   | Statements

Statements
  ::= Expression (";" Expression)* ";"?

Expression (top level)
  ::= RemoteExecution
   | InnerExpression

RemoteExecution (lowest precedence at 1)
  ::= InnerExpression "::" InnerExpression

InnerExpression (2)
  ::= TypeExpression
   | IfExpression
   | DeclarationOrAssignment
   | FunctionDeclaration
   | ComparisonExpression

IfExpression
  ::= "if" Condition Expression
      ("else" (IfExpression | Expression))?

Condition (3)
  ::= ComparisonExpression

ComparisonExpression (non-associative)
  ::= BinaryExpression
     (("==" | "!=" | "is") BinaryExpression)?

BinaryExpression (left associative) (4)
  ::= ChainExpression
     (("+" | "-" | "*" | "/") ChainExpression)*

ChainExpression (application, property access) (5)
  ::= UnaryExpression
     ( "." Key
     | UnaryExpression )*

UnaryExpression (6)
  ::= ("+" | "-" | "!")* RangeExpression

RangeExpression (7)
  ::= Atom
   | Atom ".." Atom

Atom (highest precedence at 8)
  ::= Literal
   | List
   | Map
   | "(" Statements ")"

Literal
  ::= Number
   | String
   | Identifier

List
  ::= "[" (Expression ("," Expression)*)? "]"

Map
  ::= "{" (Key ":" Expression ("," Key ":" Expression)*)? "}"

Key
  ::= Identifier
   | String
   | "(" Expression ")"

// FunctionDeclaration
//  ::= "fn" "(" ParameterList? ")" Expression

DeclarationOrAssignment
  ::= "let" Identifier "=" Expression
   | Identifier "=" Expression

TypeExpression
  ::= Identifier
```
---
```
Expression
├─ RemoteExecution
│   ├─ InnerExpression
│   └─ InnerExpression
└─ InnerExpression
   ├─ IfExpression
   ├─ DeclarationOrAssignment
   ├─ TypeExpression
   └─ ComparisonExpression
      └─ BinaryExpression
         └─ ChainExpression
            └─ UnaryExpression
               └─ RangeExpression
                  └─ Atom
```
---
```
┌──────────────────────────────────────────────┐
│ Remote execution                             │
│   InnerExpression :: InnerExpression         │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ If / Declaration / Type / Comparison entry   │
│   if … else …                                │
│   let x = …                                  │
│   x = …                                      │
│   TypeExpression                             │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Comparison (non‑associative)                 │
│   ==   !=   is                               │
│   a == b   (but NOT a == b == c)             │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Binary arithmetic (left‑associative)         │
│   +   -   *   /                              │
│   a + b * c - d                              │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Chain / application / access                 │
│   f x y                                      │
│   a.b.c                                      │
│   f x.y z                                    │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Unary                                        │
│   +  -  !                                    │
│   !!!x   -a   +b                             │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Range                                        │
│   a .. b                                     │
└──────────────────────────────────────────────┘
                     ↓
┌──────────────────────────────────────────────┐
│ Atom                                         │
│   literals, lists, maps, (…)                 │
│   42  "hi"  foo  [a,b]  {x:1}                │
└──────────────────────────────────────────────┘
```
