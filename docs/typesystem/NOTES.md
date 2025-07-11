# Type system considerations

## Typing
```
var jsexport: any = import(x);
var y = jsexport.x() # error
var z: fun(<any>()) -> any = jsexport.x
z(1, 2, 3)
```

```
if (jsexport match { fun x(): int }) (
    val a = (text)jsexport.x(1, 2, 3) # error
    val b = jsexport.x() # works (int)
)
```

```
val x = 35 as text; # runtime error
val x: text | integer = remote::call() as integer # if text throws
val x: text | integer = remote::call()
```


## Equality concept

* `==` checks that memory is the same, type doesn't matter `User({1,2}) == {1,2}` is true
* `===` checks that values match and type is identical (for all parts of the concatenation)

```
a == b or Ref<a> == Ref<b> or Ref<a> == b -> soft equality comparison

a === b or Ref<a> === Ref<b> or Ref<a> === b makes a value comparison, including strict type compare

Ref<a> is Ref<b> -> identity check, pointer ids must match

a is b -> identity check on values is not allowed
```

```
val x = 12000; -> gets inferred as integer/integer
val y = 12000; -> gets inferred as integer/integer

x == y # true
x === y # true

val x: u8 = 12; -> gets inferred as integer/u8
val y = 12 -> gets inferred as integer/integer

x == y # true
x === y # false


val x: decimal/f32 = 12.0; -> gets inferred as decimal/f32
val y = 12 -> gets inferred as integer/integer

x == y # true
x === y # false

val x: text/application = "xxxxx" # text/application
val x = "xxxxx" # gets inferred as text/plain

x == y # true
x === y # false

val x = [1,2]
val y = [1,2]

x == y # true
x === y # true
```

```
val x = User({1:2})
val y = {1:2}

x == y # true
x === y # false
```

```
x = "test" & {y: 1} & {y:2}
x.y = 10 # x gets "test" & {y: 1} & {y:10}
(x as text).length # 4

x == "test" # true
(x as {y: 1}).y == 1 # true
(x as {y: 2}).y == 2 # true
(x as {y: integer}).y == 2 # ?????

```