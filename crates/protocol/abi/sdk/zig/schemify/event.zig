//! event.zig: one event's wire form in → a typed `Event` out (docs/spec/plugin-abi.md §6):
//! one `event` line, then for `io_done` and `rejected` only, a NUL and the payload.
//!
//! Contents: `pub const Event` (`parse(in: []const u8) error{Parse}!Event`)
const std = @import("std");
const numbers = @import("numbers.zig");
const values = @import("values.zig");
const Code = numbers.Code;

/// One event. Strings borrow the input buffer, which is reused after `onEvent` returns:
/// copy what must outlive the event.
pub const Event = union(enum) {
    load,
    unload,
    /// `args` is line-record values; walk them with `values(args)`.
    command: struct { name: []const u8, args: []const u8 },
    doc_opened: u32,
    doc_closed: u32,
    doc_changed: struct { doc: u32, epoch: u32 },
    selection_changed,
    netlist_request: struct { dialect: []const u8, bench: []const u8 },
    run_job: struct { job: u32, lang: []const u8 },
    run_done: u32,
    /// `status` >= 0 is success (the exit status for `spawn`), < 0 a negated `Code`.
    io_done: struct { handle: u32, status: i32, bytes: []const u8 },
    timer: u32,
    saved: u32,
    rejected: struct { code: Code, record: []const u8 },

    /// Parses the wire form: one `event` line, then for `io_done` and `rejected` only, a
    /// NUL and the payload. Anything else, including an unknown kind, is `error.Parse`.
    pub fn parse(in: []const u8) error{Parse}!Event {
        const nul = std.mem.indexOfScalar(u8, in, 0);
        var line = in[0 .. nul orelse in.len];
        const payload: ?[]const u8 = if (nul) |i| in[i + 1 ..] else null;
        if (std.mem.endsWith(u8, line, "\n")) line.len -= 1;
        if (std.mem.indexOfAny(u8, line, "\r\n") != null) return error.Parse;
        var t: Tokens = .{ .rest = line };
        if (!std.mem.eql(u8, t.next(), "event")) return error.Parse;
        const tag = std.meta.stringToEnum(std.meta.Tag(Event), t.next()) orelse return error.Parse;
        const bytes = payload orelse "";
        const ev: Event = switch (tag) {
            .load => .load,
            .unload => .unload,
            .selection_changed => .selection_changed,
            .command => .{ .command = .{
                .name = try t.ident(),
                .args = std.mem.trimStart(u8, t.take(), " \t"),
            } },
            .doc_opened => .{ .doc_opened = try t.int(u32) },
            .doc_closed => .{ .doc_closed = try t.int(u32) },
            .saved => .{ .saved = try t.int(u32) },
            .doc_changed => .{ .doc_changed = .{ .doc = try t.int(u32), .epoch = try t.int(u32) } },
            .netlist_request => .{ .netlist_request = .{ .dialect = try t.ident(), .bench = try t.ident() } },
            .run_job => .{ .run_job = .{ .job = try t.int(u32), .lang = try t.ident() } },
            .run_done => .{ .run_done = try t.int(u32) },
            .timer => .{ .timer = try t.int(u32) },
            .io_done => .{ .io_done = .{ .handle = try t.int(u32), .status = try t.int(i32), .bytes = bytes } },
            .rejected => .{ .rejected = .{ .code = try t.code(), .record = bytes } },
        };
        const wants_payload = tag == .io_done or tag == .rejected;
        if ((payload != null) != wants_payload or t.next().len != 0) return error.Parse;
        return ev;
    }
};

/// Whitespace-separated tokens of an event line (bare text only).
const Tokens = struct {
    rest: []const u8,

    fn next(t: *Tokens) []const u8 {
        const s = std.mem.trimStart(u8, t.rest, " \t");
        const end = std.mem.indexOfAny(u8, s, " \t") orelse s.len;
        t.rest = s[end..];
        return s[0..end];
    }

    fn take(t: *Tokens) []const u8 {
        defer t.rest = "";
        return t.rest;
    }

    fn ident(t: *Tokens) error{Parse}![]const u8 {
        const s = t.next();
        return if (values.bare(s)) s else error.Parse;
    }

    /// Decimal with an optional '-', in `T`'s range; no '+', '_' or radix prefix.
    fn int(t: *Tokens, comptime T: type) error{Parse}!T {
        const s = t.next();
        const neg = s.len > 0 and s[0] == '-';
        const digits = s[@intFromBool(neg)..];
        if (digits.len == 0) return error.Parse;
        var n: i64 = 0;
        for (digits) |c| {
            if (c < '0' or c > '9' or n > 1 << 40) return error.Parse;
            n = n * 10 + (c - '0');
        }
        return std.math.cast(T, if (neg) -n else n) orelse error.Parse;
    }

    fn code(t: *Tokens) error{Parse}!Code {
        const s = t.next();
        for (numbers.code_names, 1..) |name, c| if (std.mem.eql(u8, s, name)) return @enumFromInt(c);
        return error.Parse;
    }
};
