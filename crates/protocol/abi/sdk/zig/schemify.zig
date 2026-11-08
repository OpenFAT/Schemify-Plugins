//! schemify.zig: the Schemify plugin ABI v1 for Zig 0.16 (docs/spec/plugin-abi.md).
//!
//! The module root: it re-exports the files in `schemify/`. Std only, no WASI. A plugin
//! imports it and exports its glue:
//!
//!     const schemify = @import("schemify");
//!     comptime { schemify.exportPlugin(@This()); }
//!     pub fn onEvent(ev: schemify.Event, out: *schemify.Out) void { ... }
//!
//! Build an executable for wasm32-freestanding with `entry = .disabled` and
//! `rdynamic = true`; the module imports nothing but module "schemify".
//!
//! Buffers are static: an event over `input_max` bytes is refused (the host reports
//! E_SPACE), and output past `output_max` is replaced by one error log. A plugin changes
//! either by declaring `pub const schemify_input_max` / `schemify_output_max`.
//!
//! Contents:
//! - numbers (`schemify/numbers.zig`, generated): `pub const abi_version`, `Code`,
//!   `code_names`, `Table`, `col`, `Level`, `Kind`
//! - imports (`schemify/imports.zig`): `pub const Error`, `codeOf`, `raw`, `tableLen`,
//!   `tableEpoch`, `readColumn`, `readStrings`, `Strings`, `eval`, `log`, `fsRead`, `fsList`,
//!   `fsWrite`, `fsRemove`, `spawn`, `fetch`, `clipboardGet`, `clipboardSet`
//! - events (`schemify/event.zig`): `pub const Event`
//! - values (`schemify/values.zig`): `pub const values`, `Values`
//! - output (`schemify/out.zig`): `pub const EditMode`, `Out`
//! - exports (`schemify/glue.zig`): `pub const exportPlugin`
const numbers = @import("schemify/numbers.zig");
const imports = @import("schemify/imports.zig");
const event = @import("schemify/event.zig");
const line_values = @import("schemify/values.zig");
const out = @import("schemify/out.zig");
const glue = @import("schemify/glue.zig");

pub const abi_version = numbers.abi_version;
pub const Code = numbers.Code;
pub const code_names = numbers.code_names;
pub const Table = numbers.Table;
pub const col = numbers.col;
pub const Level = numbers.Level;
pub const Kind = numbers.Kind;

pub const Error = imports.Error;
pub const codeOf = imports.codeOf;
pub const raw = imports.raw;
pub const tableLen = imports.tableLen;
pub const tableEpoch = imports.tableEpoch;
pub const readColumn = imports.readColumn;
pub const readStrings = imports.readStrings;
pub const Strings = imports.Strings;
pub const eval = imports.eval;
pub const log = imports.log;
pub const fsRead = imports.fsRead;
pub const fsList = imports.fsList;
pub const fsWrite = imports.fsWrite;
pub const fsRemove = imports.fsRemove;
pub const spawn = imports.spawn;
pub const fetch = imports.fetch;
pub const clipboardGet = imports.clipboardGet;
pub const clipboardSet = imports.clipboardSet;

pub const Event = event.Event;
pub const values = line_values.values;
pub const Values = line_values.Values;
pub const EditMode = out.EditMode;
pub const Out = out.Out;
pub const exportPlugin = glue.exportPlugin;
