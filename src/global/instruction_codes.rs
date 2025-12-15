use core::prelude::rust_2024::*;
use num_enum::TryFromPrimitive;
use strum::Display;
use strum_macros::EnumIter;

#[allow(non_camel_case_types)]
#[derive(
    EnumIter,
    Debug,
    Eq,
    PartialEq,
    TryFromPrimitive,
    Copy,
    Clone,
    Display,
    num_enum::IntoPrimitive,
)]
#[repr(u8)]
pub enum InstructionCode {
    // flow instructions 0x00 - 0x0f
    EXIT = 0x00,
    STATEMENTS,      // statements block
    SHORT_STATEMENTS, // optimized statements block with up to 255 instructions
    UNBOUNDED_STATEMENTS,
    UNBOUNDED_STATEMENTS_END,  // end of statements block (only needed for unbounded blocks)
    CACHE_POINT,     // cache dxb from this point on
    CACHE_RESET,     // reset dxb scope cache

    // internal variables and other shorthands 0x30 - 0x4f
    VAR_RESULT,
    SET_VAR_RESULT,
    SET_VAR_RESULT_REFERENCE,
    VAR_RESULT_ACTION,

    VAR_SUB_RESULT,
    SET_VAR_SUB_RESULT,
    SET_VAR_SUB_RESULT_REFERENCE,
    VAR_SUB_RESULT_ACTION,

    VAR_VOID,
    SET_VAR_VOID,
    SET_VAR_VOID_REFERENCE,
    VAR_VOID_ACTION,

    _VAR_ORIGIN,
    _SET_VAR_ORIGIN,
    _SET_VAR_ORIGIN_REFERENCE,
    _VAR_ORIGIN_ACTION,

    VAR_IT,
    SET_VAR_IT,
    SET_VAR_IT_REFERENCE,
    VAR_IT_ACTION,

    VAR_REMOTE,

    VAR_REMOTE_ACTION,
    VAR_ORIGIN,
    VAR_ENDPOINT,
    VAR_ENTRYPOINT,
    VAR_STD,
    // VAR_TIMESTAMP      ,
    VAR_META,
    VAR_PUBLIC,
    VAR_THIS,
    VAR_LOCATION,
    VAR_ENV,

    APPLY_ZERO,
    APPLY_SINGLE,
    APPLY,

    // runtime commands 0x50 - 0x7f
    RETURN,         // return
    TEMPLATE,       // template
    EXTENDS,        // extends
    IMPLEMENTS,     // implements
    MATCHES,        // matches
    DEBUGGER,       // debugger
    JMP,            // jmp labelname
    JTR,            // jtr labelname
    JFA,            // jfa labelname (TODO replace with 0xa)
    COUNT,          // count x
    ABOUT,          // about x
    NEW,            // new <x> ()
    DELETE_POINTER, // delete $aa
    COPY,           // copy $aa
    CLONE,          // clone $aa
    ORIGIN,         // origin $aa
    SUBSCRIBERS,    // subscribers $aa
    PLAIN_SCOPE,    // scope xy;
    // don't use 0x64 (magic number)

    // comparators 0x80 - 0x8f
    STRUCTURAL_EQUAL,     // ==
    NOT_STRUCTURAL_EQUAL, // !=
    EQUAL,                // ===
    NOT_EQUAL,            // !==
    GREATER,              // >
    LESS,                 // <
    GREATER_EQUAL,        // >=
    LESS_EQUAL,           // <=
    IS,                   // is

    // logical + algebraic operators 0x90  - 0x9f
    AND,       // &
    OR,        // |
    ADD,       // +
    SUBTRACT,  // -
    MULTIPLY,  // *
    DIVIDE,    // /
    NOT,       // !
    MODULO,    // %
    POWER,     // ^
    INCREMENT, // ++
    DECREMENT, // --

    UNARY_PLUS,
    UNARY_MINUS,
    BITWISE_NOT, // ~

    UNION, // | // TODO #428: maybe create a union collection of multiple values, instead of using this as a binary operator?

    // assignment operators
    ASSIGN,          // =
    ADD_ASSIGN,      // +=
    SUBTRACT_ASSIGN, // -=
    MULTIPLY_ASSIGN, // *=
    DIVIDE_ASSIGN,   // /=
    MODULO_ASSIGN,   // %=
    POWER_ASSIGN,    // ^=

    // pointers & variables 0xa0 - 0xbf

    // slots
    GET_SLOT, // #xyz   0x0000-0x00ff = variables passed on between scopes, 0x0100-0xfdff = normal variables, 0xfe00-0xffff = it variables (#it.0, #it.1, ...) for function arguments
    SET_SLOT, // #aa = ...
    ALLOCATE_SLOT, // #aa = ...
    SLOT_ACTION, // #x += ...
    DROP_SLOT, // drop #aa

    LABEL,        // $x
    SET_LABEL,    // $x = ...,
    INIT_LABEL,   // $x := ...
    LABEL_ACTION, // $x += ...

    // Note: fix to sync with RawPointerAddress
    GET_REF = 120u8,          // $x
    GET_INTERNAL_REF = 121u8, // $y, containing globally unique internal id
    GET_LOCAL_REF = 122u8, // $x, containing only the id, origin id is inferred from sender
    GET_OR_INIT_REF, // $aa := ...
    POINTER_ACTION, // $aa += ...
    CREATE_REF,    // &()
    CREATE_REF_MUT, // &mut ()
    SET_REF,       // &aa = ...

    ASSIGN_TO_REF, // *x = 10;

    DEREF, // *x

    CHILD_GET,           // .y
    CHILD_SET,           // .y = a
    CHILD_SET_REFERENCE, // .y $= a
    CHILD_ACTION,        // .y += a, ...
    CHILD_GET_REF,       // ->y

    WILDCARD, // *

    /// type byte codes --> switch to Type Space
    TYPED_VALUE,
    TYPE_EXPRESSION, // type()

    // ...

    // values 0xc0 - 0xdf
    TEXT,
    INT_8, // byte
    INT_16,
    INT_32,
    INT_64,
    INT_128,
    INT_BIG,

    UINT_8, // u8
    UINT_16,
    UINT_32,
    UINT_64,
    UINT_128,

    DECIMAL_F32,
    DECIMAL_F64,
    DECIMAL_BIG,
    DECIMAL_AS_INT_32,
    DECIMAL_AS_INT_16,

    TRUE,
    FALSE,
    NULL,
    VOID,
    BUFFER,
    QUANTITY,

    SHORT_TEXT, // string with max. 255 characters

    PERSON_ALIAS,
    PERSON_ALIAS_WILDCARD,
    INSTITUTION_ALIAS,
    INSTITUTION_ALIAS_WILDCARD,
    BOT,
    BOT_WILDCARD,

    ENDPOINT,
    ENDPOINT_WILDCARD,

    URL, //file://... , https://...

    TIME, // ~2022-10-10~

    // lists and maps 0xe0 - 0xef
    LIST,   // (1,2,3)
    SHORT_LIST, // (1,2,3) - optimized short list with up to 255 elements
    MAP,    // (a:1, b:2)
    SHORT_MAP, // {a:1, b:2} - optimized short map with up to 255 elements

    KEY_VALUE_SHORT_TEXT,
    KEY_VALUE_DYNAMIC, // for object elements with dynamic key
    KEY_PERMISSION,    // for object elements with permission prefix
    INTERNAL_OBJECT_SLOT, // for object internal slots

    // special instructions 0xf0 - 0xff
    SYNC,      // <==
    STOP_SYNC, // </=

    STREAM,      // << stream
    STOP_STREAM, // </ stream


    REMOTE_EXECUTION, // ::

    TRANSFORM,             // transform x <Int>
    OBSERVE,               // observe x ()=>()
    RUN,                   // run xy;
    AWAIT,                 // await xy;
    DEFER,                 // maybe xy;
    FUNCTION,              // function ()
    ASSERT,                // assert
    ITERATOR,              // iterator ()
    NEXT,                  // next it
    FREEZE,                // freeze
    SEAL,                  // seal
    HAS,                   // x has y
    KEYS,                  // keys x
    GET_TYPE,              // type $aa
    GET,                   // get file://..., get @user::34
    RANGE,                 // ..
    RESOLVE_RELATIVE_PATH, // ./abc
    DO,                    // do xy;
    DEFAULT,               // x default y
    COLLAPSE,              // collapse x
    RESPONSE,              // response x
    CLONE_COLLAPSE,        // collapse

}


#[cfg(test)]
mod tests {
    use strum::IntoEnumIterator;
    use super::*;

    #[ignore]
    #[test]
    fn test_instruction_code_values() {
        // print a list of all instruction codes and their values for debugging purposes
        for code in InstructionCode::iter() {
            println!("{:?} = {:2X}", code, code as u8);
        }
    }
}