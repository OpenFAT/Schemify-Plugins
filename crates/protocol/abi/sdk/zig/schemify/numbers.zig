//! numbers.zig: the ABI's numbers (codes, tables, columns, levels, kinds), generated from
//! `crates/protocol/abi` (`SCHEMIFY_BLESS=1 cargo test -p schemify-abi --test sdk` rewrites
//! the region; docs/spec/plugin-abi.md §1, §4, §8). Leaf: imports nothing.
//!
//! Contents: `pub const abi_version: u32`, `pub const Code`, `pub const code_names`,
//! `pub const Table`, `pub const col` (one struct per table: docs instances wires labels nets
//! inst_params paths name_map violations doc_vars bench_specs bench_outputs results selection
//! ui_nodes tabs tool status options camera dialog dialog_rows; columns `handle` `kind` `dirty`
//! `path` `cell` `doc` `id` `x` `y` `orient` `name` `x1` `y1` `x2` `y2` `net` `text` `width`
//! `global` `pin_count` `inst_row` `value` `source` `impl` `parent` `emitted` `code` `severity`
//! `object` `param` `expr` `min` `max` `job` `output` `pass` `owner` `title` `active` `symbol`
//! `selected` `message` `scale` `purpose` `note` `label` `editable`), `pub const Level`,
//! `pub const Kind`

// BEGIN GENERATED
pub const abi_version: u32 = 1;

/// Error codes (ABI §8); imports return them negated.
pub const Code = enum(u32) {
    denied = 1,
    gone = 2,
    space = 3,
    table = 4,
    column = 5,
    range = 6,
    parse = 7,
    fuel = 8,
    version = 9,
    trap = 10,
    module = 11,
};
/// Wire names of `Code`, indexed by `code - 1`.
pub const code_names = [_][]const u8{
    "E_DENIED",
    "E_GONE",
    "E_SPACE",
    "E_TABLE",
    "E_COLUMN",
    "E_RANGE",
    "E_PARSE",
    "E_FUEL",
    "E_VERSION",
    "E_TRAP",
    "E_MODULE",
};

/// Snapshot tables (ABI §4).
pub const Table = enum(u32) {
    docs,
    instances,
    wires,
    labels,
    nets,
    inst_params,
    paths,
    name_map,
    violations,
    doc_vars,
    bench_specs,
    bench_outputs,
    results,
    selection,
    ui_nodes,
    tabs,
    tool,
    status,
    options,
    camera,
    dialog,
    dialog_rows,
};

/// Column ids per table: `col.nets.pin_count`.
pub const col = struct {
    pub const docs = struct {
        pub const handle: u32 = 0; // u32
        pub const kind: u32 = 1; // u8
        pub const dirty: u32 = 2; // u8
        pub const path: u32 = 3; // str
        pub const cell: u32 = 4; // str
    };
    pub const instances = struct {
        pub const doc: u32 = 0; // u32
        pub const id: u32 = 1; // u32
        pub const x: u32 = 2; // i32
        pub const y: u32 = 3; // i32
        pub const orient: u32 = 4; // u8
        pub const cell: u32 = 5; // str
        pub const name: u32 = 6; // str
    };
    pub const wires = struct {
        pub const doc: u32 = 0; // u32
        pub const id: u32 = 1; // u32
        pub const x1: u32 = 2; // i32
        pub const y1: u32 = 3; // i32
        pub const x2: u32 = 4; // i32
        pub const y2: u32 = 5; // i32
        pub const net: u32 = 6; // u32
    };
    pub const labels = struct {
        pub const doc: u32 = 0; // u32
        pub const id: u32 = 1; // u32
        pub const x: u32 = 2; // i32
        pub const y: u32 = 3; // i32
        pub const net: u32 = 4; // u32
        pub const text: u32 = 5; // str
    };
    pub const nets = struct {
        pub const name: u32 = 0; // str
        pub const width: u32 = 1; // u16
        pub const global: u32 = 2; // u8
        pub const pin_count: u32 = 3; // u32
    };
    pub const inst_params = struct {
        pub const inst_row: u32 = 0; // u32
        pub const name: u32 = 1; // str
        pub const value: u32 = 2; // str
        pub const source: u32 = 3; // u8
    };
    pub const paths = struct {
        pub const path: u32 = 0; // str
        pub const cell: u32 = 1; // str
        pub const impl: u32 = 2; // str
        pub const parent: u32 = 3; // u32
    };
    pub const name_map = struct {
        pub const path: u32 = 0; // str
        pub const emitted: u32 = 1; // str
    };
    pub const violations = struct {
        pub const code: u32 = 0; // str
        pub const severity: u32 = 1; // u8
        pub const object: u32 = 2; // u32
        pub const param: u32 = 3; // str
    };
    pub const doc_vars = struct {
        pub const name: u32 = 0; // str
        pub const expr: u32 = 1; // str
        pub const value: u32 = 2; // str
    };
    pub const bench_specs = struct {
        pub const name: u32 = 0; // str
        pub const expr: u32 = 1; // str
        pub const min: u32 = 2; // str
        pub const max: u32 = 3; // str
    };
    pub const bench_outputs = struct {
        pub const name: u32 = 0; // str
        pub const expr: u32 = 1; // str
        pub const min: u32 = 2; // str
        pub const max: u32 = 3; // str
    };
    pub const results = struct {
        pub const job: u32 = 0; // u32
        pub const output: u32 = 1; // str
        pub const value: u32 = 2; // str
        pub const pass: u32 = 3; // u8
    };
    pub const selection = struct {
        pub const doc: u32 = 0; // u32
        pub const id: u32 = 1; // u32
    };
    pub const ui_nodes = struct {
        pub const id: u32 = 0; // str
        pub const kind: u32 = 1; // u8
        pub const parent: u32 = 2; // str
        pub const owner: u32 = 3; // u16
    };
    pub const tabs = struct {
        pub const doc: u32 = 0; // u32
        pub const title: u32 = 1; // str
        pub const active: u32 = 2; // u8
        pub const dirty: u32 = 3; // u8
        pub const symbol: u32 = 4; // u8
        pub const text: u32 = 5; // u8
    };
    pub const tool = struct {
        pub const name: u32 = 0; // str
        pub const owner: u32 = 1; // u32
    };
    pub const status = struct {
        pub const x: u32 = 0; // i32
        pub const y: u32 = 1; // i32
        pub const selected: u32 = 2; // u32
        pub const path: u32 = 3; // str
        pub const message: u32 = 4; // str
    };
    pub const options = struct {
        pub const name: u32 = 0; // str
        pub const value: u32 = 1; // str
    };
    pub const camera = struct {
        pub const x: u32 = 0; // i32
        pub const y: u32 = 1; // i32
        pub const scale: u32 = 2; // u32
    };
    pub const dialog = struct {
        pub const kind: u32 = 0; // str
        pub const purpose: u32 = 1; // str
        pub const title: u32 = 2; // str
        pub const text: u32 = 3; // str
        pub const note: u32 = 4; // str
    };
    pub const dialog_rows = struct {
        pub const label: u32 = 0; // str
        pub const value: u32 = 1; // str
        pub const editable: u32 = 2; // u8
    };
};

/// Log levels.
pub const Level = enum(u32) {
    @"error",
    warn,
    info,
    debug,
};

/// Contribution kinds (ABI §1); `@tagName` is the wire name.
pub const Kind = enum {
    ui,
    commands,
    keymap,
    annotations,
    functions,
    dialects,
    runners,
    checks,
    constraint_types,
    options,
};
// END GENERATED
