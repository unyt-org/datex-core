# DATEX Formatter

## Parentheses Handling

| Category             | Example       | Expected Output |
| -------------------- | ------------- | --------------- |
| Precedence           | `(1 + 2) * 3` | `(1 + 2) * 3`   |
| Precedence (reverse) | `1 + (2 * 3)` | `1 + 2 * 3`     |
| Associativity        | `(1 + 2) + 3` | `1 + 2 + 3`     |
| Non-associative      | `1 - (2 - 3)` | `1 - (2 - 3)`   |
| Right-associative    | `2 ^ (3 ^ 4)` | `2 ^ 3 ^ 4`     |
| Redundant parens     | `(((x)))`     | `(x)`           |
| KeepAll              | `(((x)))`     | `(((x)))`       |
