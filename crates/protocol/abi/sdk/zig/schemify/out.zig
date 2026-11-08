//! out.zig: output records (docs/spec/plugin-abi.md §7) written into a caller-owned buffer:
//! calls in → line-record text out, values escaped per format spec §2.
//!
//! Contents: `pub const EditMode`, `pub const Out` (`init`, `slice`, `put`, `print`,
//! `putValue`, `contribute`, `contributeScoped`, `edit`, `run`, `vary`, `timer`, `log`)
const std = @import("std");
const numbers = @import("numbers.zig");
const values = @import("values.zig");
const Kind = numbers.Kind;
const Level = numbers.Level;

/// Whether an `edit` waits for the user (`stage`) or applies at once (`commit`).
pub const EditMode = enum { stage, commit };

/// An append-only text buffer: the event's output records, or any text you compose (a
/// log message, an edit's ops). Once full, `overflow` is set and every later append is
/// dropped whole, so the text never ends mid-append.
pub const Out = struct {
    buf: []u8,
    len: usize = 0,
    overflow: bool = false,

    /// An empty buffer writing into `buf`.
    pub fn init(buf: []u8) Out {
        return .{ .buf = buf };
    }

    /// The text written so far; valid until the next append.
    pub fn slice(o: *const Out) []const u8 {
        return o.buf[0..o.len];
    }

    /// Raw text.
    pub fn put(o: *Out, s: []const u8) void {
        if (o.overflow or s.len > o.buf.len - o.len) {
            o.overflow = true;
            return;
        }
        @memcpy(o.buf[o.len..][0..s.len], s);
        o.len += s.len;
    }

    /// Raw text formatted by `std.fmt`.
    pub fn print(o: *Out, comptime fmt: []const u8, args: anytype) void {
        if (o.overflow) return;
        const s = std.fmt.bufPrint(o.buf[o.len..], fmt, args) catch {
            o.overflow = true;
            return;
        };
        o.len += s.len;
    }

    /// One value: bare if legal, else quoted, with `"` `\` LF TAB escaped and other
    /// control bytes as `\u{x}` (lowercase hex, no leading zeros).
    pub fn putValue(o: *Out, v: []const u8) void {
        if (values.bare(v)) return o.put(v);
        o.put("\"");
        for (v) |c| switch (c) {
            '"' => o.put("\\\""),
            '\\' => o.put("\\\\"),
            '\n' => o.put("\\n"),
            '\t' => o.put("\\t"),
            0...8, 0x0b...0x1f, 0x7f => o.print("\\u{{{x}}}", .{c}),
            else => o.put(&.{c}),
        };
        o.put("\"");
    }

    // Records. A body is line-record text, one record per line, written verbatim.

    /// Declares this plugin's whole set of `kind` (immediate mode: it replaces the last).
    pub fn contribute(o: *Out, kind: Kind, body: []const u8) void {
        o.print("contribute {s}\n", .{@tagName(kind)});
        o.verbatim(body);
    }

    /// Like `contribute`, but replaces only the set under `scope`.
    pub fn contributeScoped(o: *Out, kind: Kind, scope: []const u8, body: []const u8) void {
        o.print("contribute {s} ", .{@tagName(kind)});
        o.putValue(scope);
        o.put("\n");
        o.verbatim(body);
    }

    /// Edits `doc` with `ops` as one undo step named `label`.
    pub fn edit(o: *Out, doc: u32, mode: EditMode, label: []const u8, ops: []const u8) void {
        o.print("edit {d} {s} ", .{ doc, @tagName(mode) });
        o.putValue(label);
        o.put("\n");
        o.verbatim(ops);
    }

    /// Runs a testbench; follow with `vary` calls for its sweep variables.
    pub fn run(o: *Out, bench: []const u8, config: []const u8, corner: []const u8) void {
        o.put("run ");
        o.putValue(bench);
        o.put(" ");
        o.putValue(config);
        o.put(" ");
        o.putValue(corner);
        o.put("\n");
    }

    /// Sets `variable` for the `run` written just before; anywhere else it is rejected.
    pub fn vary(o: *Out, variable: []const u8, value: []const u8) void {
        o.put("  vary ");
        o.putValue(variable);
        o.put(" ");
        o.putValue(value);
        o.put("\n");
    }

    /// Asks for a `timer` event with `id` after `ms` milliseconds.
    pub fn timer(o: *Out, id: u32, ms: u32) void {
        o.print("timer {d} {d}\n", .{ id, ms });
    }

    /// Logs `text` once the event returns (`schemify.log` logs at once).
    pub fn log(o: *Out, level: Level, text: []const u8) void {
        o.print("log {s} ", .{@tagName(level)});
        o.putValue(text);
        o.put("\n");
    }

    /// Each line of `body` as `  | line`, an empty one as `  |`; a final newline adds none.
    fn verbatim(o: *Out, body: []const u8) void {
        var lines = std.mem.splitScalar(u8, body, '\n');
        while (lines.next()) |l| {
            if (lines.index == null and l.len == 0) break;
            const line = if (std.mem.endsWith(u8, l, "\r")) l[0 .. l.len - 1] else l;
            o.put(if (line.len > 0) "  | " else "  |");
            o.put(line);
            o.put("\n");
        }
    }
};
