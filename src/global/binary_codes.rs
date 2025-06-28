use num_enum::TryFromPrimitive;
use strum::Display;

#[allow(non_camel_case_types)]
#[derive(
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

// x = a * 2;
// b();

// x;
// y;

// SETCURRENTVAR #x;

// ADD;
// VAR a
// INT 2 -> active

// CLOSE_AND_STORE
// APPLY
// VAR b
// TUPLE <-- apply
// SCOPE_START
// asdfasdf
// SCOPE_END <- <actu

pub enum InstructionCode {
    // flow instructions 0x00 - 0x0f
    EXIT = 0x00,
    CLOSE_AND_STORE, // ;
    SCOPE_START,     // (
    SCOPE_END,       // )
    CACHE_POINT,     // cache dxb from this point on
    CACHE_RESET,     // reset dxb scope cache

    // primitive / fundamental types 0x10 - 0x2f
    STD_TYPE_TEXT,
    STD_TYPE_INT,
    STD_TYPE_FLOAT,
    STD_TYPE_BOOLEAN,
    STD_TYPE_NULL,
    STD_TYPE_VOID,
    STD_TYPE_BUFFER,
    STD_TYPE_CODE_BLOCK,
    STD_TYPE_QUANTITY,
    STD_TYPE_TIME,
    STD_TYPE_URL,

    STD_TYPE_ARRAY,
    STD_TYPE_OBJECT,
    STD_TYPE_SET,
    STD_TYPE_MAP,
    STD_TYPE_TUPLE,

    STD_TYPE_FUNCTION,
    STD_TYPE_STREAM,
    STD_TYPE_ANY,
    STD_TYPE_ASSERTION,
    STD_TYPE_TASK,
    STD_TYPE_ITERATOR,

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

    // comparators 0x80 - 0x8f
    STRUCTURAL_EQUAL,      // ==
    NOT_STRUCTURAL_EQUAL,  // !=
    EQUAL,     // ===
    NOT_EQUAL, // !==
    GREATER,          // >
    LESS,             // <
    GREATER_EQUAL,    // >=
    LESS_EQUAL,       // <=
    IS,               // is

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

    // pointers & variables 0xa0 - 0xbf

    // slots
    GET_SLOT, // #xyz   0x0000-0x00ff = variables passed on between scopes, 0x0100-0xfdff = normal variables, 0xfe00-0xffff = it variables (#it.0, #it.1, ...) for function arguments
    UPDATE_SLOT, // #aa = ...
    ALLOCATE_SLOT, // #aa = ...
    SLOT_ACTION, // #x += ...
    DROP_SLOT, // drop #aa

    LABEL,        // $x
    SET_LABEL,    // $x = ...,
    INIT_LABEL,   // $x := ...
    LABEL_ACTION, // $x += ...

    POINTER,        // $x
    SET_POINTER,    // $aa = ...
    INIT_POINTER,   // $aa := ...
    POINTER_ACTION, // $aa += ...
    CREATE_POINTER, // $$ ()

    CHILD_GET,           // .y
    CHILD_SET,           // .y = a
    CHILD_SET_REFERENCE, // .y $= a
    CHILD_ACTION,        // .y += a, ...
    CHILD_GET_REF,       // ->y

    WILDCARD, // *

    // values 0xc0 - 0xdf
    TEXT,
    INT_8, // byte
    INT_16,
    INT_32,
    INT_64,
    INT_128,
    INT_BIG,

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
    SCOPE_BLOCK_START,
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

    TYPE,          // <type>
    EXTENDED_TYPE, // <type/xy()>

    CONJUNCTION, // x&y&z
    DISJUNCTION, // x|y|z

    TIME, // ~2022-10-10~

    // arrays, objects and tuples 0xe0 - 0xef
    ARRAY_START,  // array / or array
    OBJECT_START, // {}
    TUPLE_START,  // (a,b,c)
    KEY_VALUE_SHORT_TEXT,
    KEY_VALUE_DYNAMIC, // for object elements with dynamic key
    KEY_PERMISSION,    // for object elements with permission prefix
    INTERNAL_OBJECT_SLOT, // for object internal slots

    // special instructions 0xf0 - 0xff
    SYNC,      // <==
    STOP_SYNC, // </=

    STREAM,      // << stream
    STOP_STREAM, // </ stream

    EXTEND, // ...

    YEET, // !

    REMOTE, // ::

    _SYNC_SILENT, // <==:
}
