# Overview of the Datex Interchange Format (DIF)

## Value Containers

```ts
type DIFValueContainer = DISharedValue | DIFValue

type DIFPointerAddressWithOwnership = string; // e.g. "'mut$12345"
type DIFPointerAddress = string; // e.g. "$12345"

type DISharedValue = { _: DIFPointerAddressWithOwnership }
```

## Values

```ts
type DIFValue =
    | { _: 'c', c?: DIFClassification, v: DIFCoreValueWithTag }
    | DIFCoreValueWithTag

type DIFCoreValueWithTag =
    | DIFDirectRepresentationValue
    | {_: 't', t: DIFValueTag, v: DIFCoreValue}
    | DIFCoreValue

type DIFCoreValue = [CoreLibTypeId, DIFRawValue] // core lib type id, raw value

type DIFClassification = [
    DIFPointerAddress | null, // entity address
    DIFPointerAddress[] | null, // impls
]


type DIFValueTag = [tag: string, is_empty?: true];
```

## Core Values
```ts
type DIFDirectRepresentationValue = boolean | string | number | null;
type DIFRawValue =
    | DIFCoreValueList // list
    | DIFCoreValueMap // map
    | DIFCoreValueRange // range
    | DIFCoreValueCallable // callable
    | Record<string, DIFValueContainer> // StructuralMapWithStringKeys
    | string // text / endpoint / integer
    | number // integer / decimal
    | boolean // booleans
    | null; // null

/**
 * Represents a list of DIFValueContainers.
 */
type DIFCoreValueList = Array<DIFValueContainer>;

/**
 * Represents a map of key-value pairs in DIF, where both keys and values are DIFValueContainers.
 */
type DIFCoreValueMap = Array<[DIFValueContainer, DIFValueContainer]>;

/**
 * Represents a callable definition with hash and optional name
 */
type DIFCoreValueCallable = [string, string|null, boolean|null];

/**
 * Represents a range of values in DIF, defined by a start and end value container, or as a tuple of two value containers.
 */
type DIFCoreValueRange = [DIFValueContainer, DIFValueContainer] | {
    start: DIFValueContainer;
    end: DIFValueContainer;
};


```

## Shared Value Containers
```ts
enum DIFSharedContainerMutability {
    Immutable = 0,
    Mutable = 1,
}

type DIFBaseSharedValueContainer = [
    DIFValueContainer, // value
    DIFSharedContainerMutability, // container mutability
    DIFTypeDefinition, // allowed type
];
```


## Types

```ts
enum CoreLibTypeId {
    null = 1,
    boolean = 2,
    integer = 3,
    integer_u8 = 500,
    integer_u16 = 501,
    integer_u32 = 502,
    integer_u64 = 503,
    integer_u128 = 504,
    integer_i8 = 505,
    integer_i16 = 506,
    integer_i32 = 507,
    integer_i64 = 508,
    integer_i128 = 509,
    integer_ibig = 510,
    decimal = 4,
    decimal_f32 = 511,
    decimal_f64 = 512,
    decimal_dbig = 513,
    text = 5,
    endpoint = 6,
    Unit = 7,
    Never = 8,
    Any = 9,
    List = 10,
    Map = 11,
    Callable = 12,
    Range = 13,
    Type = 14,
}
```