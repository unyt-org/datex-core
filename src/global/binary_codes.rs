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
    CLOSE_AND_STORE = 0x01, // ;
    SCOPE_START = 0x02,  // (
    SCOPE_END = 0x03,    // )
    CACHE_POINT = 0x04,     // cache dxb from this point on
    CACHE_RESET = 0x05,     // reset dxb scope cache

    // primitive / fundamental types 0x10 - 0x2f
    STD_TYPE_TEXT = 0x10,
    STD_TYPE_INT = 0x11,
    STD_TYPE_FLOAT = 0x12,
    STD_TYPE_BOOLEAN = 0x13,
    STD_TYPE_NULL = 0x14,
    STD_TYPE_VOID = 0x15,
    STD_TYPE_BUFFER = 0x16,
    STD_TYPE_CODE_BLOCK = 0x17,
    STD_TYPE_QUANTITY = 0x18,
    STD_TYPE_TIME = 0x19,
    STD_TYPE_URL = 0x1a,

    STD_TYPE_ARRAY = 0x1b,
    STD_TYPE_OBJECT = 0x1c,
    STD_TYPE_SET = 0x1d,
    STD_TYPE_MAP = 0x1e,
    STD_TYPE_TUPLE = 0x1f,

    STD_TYPE_FUNCTION = 0x20,
    STD_TYPE_STREAM = 0x21,
    STD_TYPE_ANY = 0x22,
    STD_TYPE_ASSERTION = 0x23,
    STD_TYPE_TASK = 0x24,
    STD_TYPE_ITERATOR = 0x25,

    // internal variables and other shorthands 0x30 - 0x4f
    VAR_RESULT = 0x30,
    SET_VAR_RESULT = 0x31,
    SET_VAR_RESULT_REFERENCE = 0x32,
    VAR_RESULT_ACTION = 0x33,

    VAR_SUB_RESULT = 0x34,
    SET_VAR_SUB_RESULT = 0x35,
    SET_VAR_SUB_RESULT_REFERENCE = 0x36,
    VAR_SUB_RESULT_ACTION = 0x37,

    VAR_VOID = 0x38,
    SET_VAR_VOID = 0x39,
    SET_VAR_VOID_REFERENCE = 0x3a,
    VAR_VOID_ACTION = 0x3b,

    _VAR_ORIGIN = 0x3c,
    _SET_VAR_ORIGIN = 0x3d,
    _SET_VAR_ORIGIN_REFERENCE = 0x3e,
    _VAR_ORIGIN_ACTION = 0x3f,

    VAR_IT = 0x40,
    SET_VAR_IT = 0x41,
    SET_VAR_IT_REFERENCE = 0x42,
    VAR_IT_ACTION = 0x43,

    VAR_REMOTE = 0x44,

    VAR_REMOTE_ACTION = 0x45,
    VAR_ORIGIN = 0x46,
    VAR_ENDPOINT = 0x47,
    VAR_ENTRYPOINT = 0x48,
    VAR_STD = 0x49,
    // VAR_TIMESTAMP       = 0x4a,
    VAR_META = 0x4b,
    VAR_PUBLIC = 0x4c,
    VAR_THIS = 0x4d,
    VAR_LOCATION = 0x4e,
    VAR_ENV = 0x4f,

    // runtime commands 0x50 - 0x7f
    RETURN = 0x50,         // return
    TEMPLATE = 0x51,       // template
    EXTENDS = 0x52,        // extends
    IMPLEMENTS = 0x53,     // implements
    MATCHES = 0x54,        // matches
    DEBUGGER = 0x55,       // debugger
    JMP = 0x56,            // jmp labelname
    JTR = 0x57,            // jtr labelname
    JFA = 0x58,            // jfa labelname (TODO replace with 0xa)
    COUNT = 0x59,          // count x
    ABOUT = 0x5a,          // about x
    NEW = 0x5b,            // new <x> ()
    DELETE_POINTER = 0x5c, // delete $aa
    COPY = 0x5f,           // copy $aa
    CLONE = 0x60,          // clone $aa
    ORIGIN = 0x61,         // origin $aa
    SUBSCRIBERS = 0x62,    // subscribers $aa
    PLAIN_SCOPE = 0x63,    // scope xy;
    // don't use 0x64 (magic number)
    TRANSFORM = 0x65,             // transform x <Int>
    OBSERVE = 0x66,               // observe x ()=>()
    RUN = 0x67,                   // run xy;
    AWAIT = 0x68,                 // await xy;
    DEFER = 0x69,                 // maybe xy;
    FUNCTION = 0x6a,              // function ()
    ASSERT = 0x6b,                // assert
    ITERATOR = 0x6c,              // iterator ()
    NEXT = 0x6d,                  // next it
    FREEZE = 0x6e,                // freeze
    SEAL = 0x6f,                  // seal
    HAS = 0x70,                   // x has y
    KEYS = 0x71,                  // keys x
    GET_TYPE = 0x72,              // type $aa
    GET = 0x73,                   // get file://..., get @user::34
    RANGE = 0x74,                 // ..
    RESOLVE_RELATIVE_PATH = 0x75, // ./abc
    DO = 0x76,                    // do xy;
    DEFAULT = 0x77,               // x default y
    COLLAPSE = 0x78,              // collapse x
    RESPONSE = 0x79,              // response x
    CLONE_COLLAPSE = 0x88,        // collapse

    // comparators 0x80 - 0x8f
    EQUAL_VALUE = 0x80,     // ==
    NOT_EQUAL_VALUE = 0x81, // ~=
    EQUAL = 0x82,           // ===
    NOT_EQUAL = 0x83,       // ~==
    GREATER = 0x84,         // >
    LESS = 0x85,            // <
    GREATER_EQUAL = 0x86,   // >=
    LESS_EQUAL = 0x87,      // <=

    // logical + algebraic operators 0x90  - 0x9f
    AND = 0x90,       // &
    OR = 0x91,        // |
    ADD = 0x92,       // +
    SUBTRACT = 0x93,  // -
    MULTIPLY = 0x94,  // *
    DIVIDE = 0x95,    // /
    NOT = 0x96,       // !
    MODULO = 0x97,    // %
    POWER = 0x98,     // ^
    INCREMENT = 0x99, // ++
    DECREMENT = 0x9a, // --

    // pointers & variables 0xa0 - 0xbf
    INTERNAL_VAR = 0xa4, // #xyz   0x0000-0x00ff = variables passed on between scopes, 0x0100-0xfdff = normal variables, 0xfe00-0xffff = it variables (#it.0, #it.1, ...) for function arguments
    SET_INTERNAL_VAR = 0xa5, // #aa = ...
    INIT_INTERNAL_VAR = 0xa6, // #aa := ...
    INTERNAL_VAR_ACTION = 0xa7, // #x += ...
    SET_INTERNAL_VAR_REFERENCE = 0xa8, // #x $= ...

    LABEL = 0xa9,        // $x
    SET_LABEL = 0xaa,    // $x = ...,
    INIT_LABEL = 0xab,   // $x := ...
    LABEL_ACTION = 0xac, // $x += ...

    POINTER = 0xad,        // $x
    SET_POINTER = 0xae,    // $aa = ...
    INIT_POINTER = 0xaf,   // $aa := ...
    POINTER_ACTION = 0xb0, // $aa += ...
    CREATE_POINTER = 0xb1, // $$ ()

    CHILD_GET = 0xb2,           // .y
    CHILD_SET = 0xb3,           // .y = a
    CHILD_SET_REFERENCE = 0xb4, // .y $= a
    CHILD_ACTION = 0xb5,        // .y += a, ...
    CHILD_GET_REF = 0xb6,       // ->y

    WILDCARD = 0xb7, // *

    // values 0xc0 - 0xdf
    TEXT = 0xc0,
    INT_8 = 0xc1, // byte
    INT_16 = 0xc2,
    INT_32 = 0xc3,
    INT_64 = 0xc4,
    FLOAT_64 = 0xc5,
    TRUE = 0xc6,
    FALSE = 0xc7,
    NULL = 0xc8,
    VOID = 0xc9,
    BUFFER = 0xca,
    SCOPE_BLOCK_START = 0xcb,
    QUANTITY = 0xcc,
    FLOAT_AS_INT_32 = 0xcd,
    FLOAT_AS_INT_8 = 0xde,
    BIG_INT = 0xdf,

    SHORT_TEXT = 0xce, // string with max. 255 characters

    PERSON_ALIAS = 0xcf,
    PERSON_ALIAS_WILDCARD = 0xd0,
    INSTITUTION_ALIAS = 0xd1,
    INSTITUTION_ALIAS_WILDCARD = 0xd2,
    BOT = 0xd3,
    BOT_WILDCARD = 0xd4,

    ENDPOINT = 0xd5,
    ENDPOINT_WILDCARD = 0xd6,

    URL = 0xd8, //file://... , https://...

    TYPE = 0xd9,          // <type>
    EXTENDED_TYPE = 0xda, // <type/xy()>

    CONJUNCTION = 0xdb, // x&y&z
    DISJUNCTION = 0xdc, // x|y|z

    TIME = 0xdd, // ~2022-10-10~

    // arrays, objects and tuples 0xe0 - 0xef
    ARRAY_START = 0xe0, // array / or array
    ARRAY_END = 0xe1,
    OBJECT_START = 0xe2, // {}
    OBJECT_END = 0xe3,
    TUPLE_START = 0xe4, // (a,b,c)
    TUPLE_END = 0xe5,
    KEY_VALUE_SHORT_TEXT = 0xe6,
    KEY_VALUE_TEXT = 0xe7,   
    // KEY_VALUE_INTEGER = 0xe8, // for array elements
    KEY_VALUE_DYNAMIC = 0xe9, // for object elements with dynamic key
    KEY_PERMISSION = 0xea,       // for object elements with permission prefix
    INTERNAL_OBJECT_SLOT = 0xef, // for object internal slots

    // special instructions 0xf0 - 0xff
    SYNC = 0xf0,      // <==
    STOP_SYNC = 0xf1, // </=

    STREAM = 0xf2,      // << stream
    STOP_STREAM = 0xf3, // </ stream

    EXTEND = 0xf4, // ...

    YEET = 0xf5, // !

    REMOTE = 0xf6, // ::

    _SYNC_SILENT = 0xf7, // <==:
}
