//! imports.zig: the host imports (docs/spec/plugin-abi.md §3, §5) as raw externs and typed
//! wrappers: arguments in → a result or an `Error` out (an import's negative return is a
//! negated ABI code, §8).
//!
//! Contents: `pub const Error`, `pub fn codeOf(err: Error) Code`, `pub const raw` (the
//! externs `table_len` `table_epoch` `read_column` `read_strings` `eval` `log` `fs_read`
//! `fs_list` `fs_write` `fs_remove` `spawn` `fetch` `clipboard_get` `clipboard_set`),
//! `pub fn tableLen`, `pub fn tableEpoch`, `pub fn readColumn`, `pub fn readStrings`,
//! `pub const Strings` (`next`), `pub fn eval`, `pub fn log`, `pub fn fsRead`, `pub fn fsList`,
//! `pub fn fsWrite`, `pub fn fsRemove`, `pub fn spawn`, `pub fn fetch`, `pub fn clipboardGet`,
//! `pub fn clipboardSet`
const std = @import("std");
const numbers = @import("numbers.zig");
const Code = numbers.Code;
const Level = numbers.Level;
const Table = numbers.Table;

/// A failed import or call; `codeOf` gives the ABI number the host saw.
pub const Error = error{ Denied, Gone, Space, Table, Column, Range, Parse, Fuel, Version, Trap, Module };

/// The raw imports, exactly as module "schemify" declares them: pointers are guest
/// addresses, lengths are bytes, and negative returns are `-Code`. The typed wrappers
/// below cover every one; reach for these only for ids outside `Table` or other raw use.
pub const raw = struct {
    pub extern "schemify" fn table_len(table: u32) i32;
    pub extern "schemify" fn table_epoch(table: u32) i32;
    pub extern "schemify" fn read_column(table: u32, column: u32, start: u32, count: u32, dst: [*]u8, dst_len: u32) i32;
    pub extern "schemify" fn read_strings(table: u32, column: u32, start: u32, count: u32, dst: [*]u8, dst_len: u32) i32;
    pub extern "schemify" fn eval(doc: u32, expr: [*]const u8, len: u32) i32;
    pub extern "schemify" fn log(level: u32, text: [*]const u8, len: u32) void;
    pub extern "schemify" fn fs_read(path: [*]const u8, path_len: u32) i32;
    pub extern "schemify" fn fs_list(path: [*]const u8, path_len: u32) i32;
    pub extern "schemify" fn fs_write(path: [*]const u8, path_len: u32, data: [*]const u8, data_len: u32) i32;
    pub extern "schemify" fn fs_remove(path: [*]const u8, path_len: u32) i32;
    pub extern "schemify" fn spawn(argv: [*]const u8, argv_len: u32, stdin: [*]const u8, stdin_len: u32) i32;
    pub extern "schemify" fn fetch(url: [*]const u8, url_len: u32, body: [*]const u8, body_len: u32) i32;
    pub extern "schemify" fn clipboard_get() i32;
    pub extern "schemify" fn clipboard_set(text: [*]const u8, len: u32) i32;
};

/// A `read_strings` buffer: each string is a u32 little-endian length then its bytes.
pub const Strings = struct {
    buf: []const u8,
    at: usize = 0,

    /// The next string, borrowing `buf`; null at the end or at a truncated entry.
    pub fn next(s: *Strings) ?[]const u8 {
        if (s.buf.len - s.at < 4) return null;
        const n = std.mem.readInt(u32, s.buf[s.at..][0..4], .little);
        if (n > s.buf.len - s.at - 4) return null;
        defer s.at += 4 + n;
        return s.buf[s.at + 4 ..][0..n];
    }
};

/// `Error` by `Code` value minus one, in `code_names` order.
const errors = [_]Error{
    error.Denied, error.Gone, error.Space,   error.Table, error.Column, error.Range,
    error.Parse,  error.Fuel, error.Version, error.Trap,  error.Module,
};

/// The ABI code of `err`, for printing or returning to the host.
pub fn codeOf(err: Error) Code {
    for (errors, 1..) |e, c| if (e == err) return @enumFromInt(c);
    unreachable;
}

/// Rows in `table` in the current snapshot.
pub fn tableLen(table: Table) Error!u32 {
    return check(raw.table_len(@intFromEnum(table)));
}

/// `table`'s epoch; it changes whenever any of its rows does, so equal epochs mean the
/// rows read before are still current.
pub fn tableEpoch(table: Table) Error!u32 {
    return check(raw.table_epoch(@intFromEnum(table)));
}

/// Reads rows `start .. start + dst.len` of a fixed-width column into `dst` and returns
/// it. `T` must be the column's element type (see `col`): a string column fails with
/// `error.Column`, a range past the end with `error.Range`.
pub fn readColumn(comptime T: type, table: Table, column: u32, start: u32, dst: []T) Error![]T {
    const bytes = std.mem.sliceAsBytes(dst);
    _ = try check(raw.read_column(@intFromEnum(table), column, start, len32(dst), bytes.ptr, len32(bytes)));
    return dst;
}

/// Reads `count` strings of a string column into `dst`; walk the result with `next`.
/// `error.Space` means `dst` was too small, and the size needed is then in its first
/// 4 bytes (`std.mem.readInt(u32, dst[0..4], .little)`).
pub fn readStrings(table: Table, column: u32, start: u32, count: u32, dst: []u8) Error!Strings {
    const n = try check(raw.read_strings(@intFromEnum(table), column, start, count, dst.ptr, len32(dst)));
    return .{ .buf = dst[0..n] };
}

/// Evaluates `expr` in `doc`'s scope. Inverted: returns a handle at once, and the value
/// arrives later as an `io_done` event carrying that handle.
pub fn eval(doc: u32, expr: []const u8) Error!u32 {
    return check(raw.eval(doc, expr.ptr, len32(expr)));
}

/// Logs `text` immediately, unlike `Out.log`, which logs when the event returns.
pub fn log(level: Level, text: []const u8) void {
    raw.log(@intFromEnum(level), text.ptr, len32(text));
}

// Capability imports: each needs its grant in schemify.project (else `error.Denied`) and
// returns a handle; the result arrives as an `io_done` event with that handle.

/// Reads a file; `io_done` carries its bytes.
pub fn fsRead(path: []const u8) Error!u32 {
    return check(raw.fs_read(path.ptr, len32(path)));
}
/// Lists a directory; `io_done` carries the entries.
pub fn fsList(path: []const u8) Error!u32 {
    return check(raw.fs_list(path.ptr, len32(path)));
}
/// Writes `data` to a file, replacing it.
pub fn fsWrite(path: []const u8, data: []const u8) Error!u32 {
    return check(raw.fs_write(path.ptr, len32(path), data.ptr, len32(data)));
}
/// Removes a file.
pub fn fsRemove(path: []const u8) Error!u32 {
    return check(raw.fs_remove(path.ptr, len32(path)));
}
/// Runs a program: `argv` is line-record values (write them with `Out.putValue`).
/// `io_done`'s status is the exit status and its bytes are stdout.
pub fn spawn(argv: []const u8, stdin: []const u8) Error!u32 {
    return check(raw.spawn(argv.ptr, len32(argv), stdin.ptr, len32(stdin)));
}
/// Fetches `url`; a non-empty `body` makes it a POST.
pub fn fetch(url: []const u8, body: []const u8) Error!u32 {
    return check(raw.fetch(url.ptr, len32(url), body.ptr, len32(body)));
}
/// Reads the clipboard; `io_done` carries its text.
pub fn clipboardGet() Error!u32 {
    return check(raw.clipboard_get());
}
/// Sets the clipboard to `text`.
pub fn clipboardSet(text: []const u8) Error!u32 {
    return check(raw.clipboard_set(text.ptr, len32(text)));
}

// ---- private ----

/// An import's return value: >= 0 is the result, < 0 is a negated code. A code this
/// library does not know (a newer host) comes back as `error.Version`.
fn check(r: i32) Error!u32 {
    if (r >= 0) return @intCast(r);
    const c = -@as(i64, r);
    return if (c <= errors.len) errors[@intCast(c - 1)] else error.Version;
}

fn len32(s: anytype) u32 {
    return @intCast(s.len);
}
