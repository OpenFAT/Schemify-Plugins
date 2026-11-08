//! values.zig: line-record values (format spec §2), text in → unescaped values out, plus
//! the bare-value predicate the event parser and `Out` share. `bare` or
//! "quoted \"escaped\"" with `\n`, `\t`, `\u{hex}`.
//!
//! Contents: `pub fn values(text: []const u8) Values`, `pub const Values` (`next`),
//! `pub fn bare(v: []const u8) bool`, `pub fn isBare(c: u8) bool`
const std = @import("std");

/// An iterator over line-record values.
pub const Values = struct {
    rest: []const u8,

    /// Unescapes the next value into `dst` and returns that part of it; null when none is
    /// left. `error.Parse` (malformed, or longer than `dst`) also ends the iteration.
    pub fn next(v: *Values, dst: []u8) error{Parse}!?[]u8 {
        const s = std.mem.trimStart(u8, v.rest, " \t");
        v.rest = ""; // stop after an error
        if (s.len == 0) return null;
        var n: usize = 0;
        var i: usize = 0;
        if (s[0] == '"') {
            i = 1;
            while (true) : (i += 1) {
                if (i == s.len or s[i] == '\n' or s[i] == '\r') return error.Parse;
                if (s[i] == '"') break;
                if (s[i] != '\\') {
                    try push(dst, &n, &.{s[i]});
                    continue;
                }
                i += 1;
                if (i == s.len) return error.Parse;
                switch (s[i]) {
                    '"', '\\' => try push(dst, &n, &.{s[i]}),
                    'n' => try push(dst, &n, "\n"),
                    't' => try push(dst, &n, "\t"),
                    'u' => {
                        if (i + 1 >= s.len or s[i + 1] != '{') return error.Parse;
                        const close = std.mem.indexOfScalarPos(u8, s, i + 2, '}') orelse return error.Parse;
                        const hex = s[i + 2 .. close];
                        if (hex.len == 0 or hex.len > 6) return error.Parse;
                        for (hex) |c| if (!std.ascii.isHex(c)) return error.Parse;
                        const cp = std.fmt.parseInt(u21, hex, 16) catch return error.Parse;
                        var utf8: [4]u8 = undefined;
                        const len = std.unicode.utf8Encode(cp, &utf8) catch return error.Parse;
                        try push(dst, &n, utf8[0..len]);
                        i = close;
                    },
                    else => return error.Parse,
                }
            }
            i += 1;
        } else {
            if (s[0] == '#' or s[0] == '|') return error.Parse;
            while (i < s.len and isBare(s[i])) i += 1;
            try push(dst, &n, s[0..i]);
        }
        if (i < s.len and !isSpace(s[i])) return error.Parse;
        v.rest = s[i..];
        return dst[0..n];
    }

    fn push(dst: []u8, n: *usize, bytes: []const u8) error{Parse}!void {
        if (bytes.len > dst.len - n.*) return error.Parse;
        @memcpy(dst[n.*..][0..bytes.len], bytes);
        n.* += bytes.len;
    }
};

/// The values of `text` (a command's or function's args), one by one.
pub fn values(text: []const u8) Values {
    return .{ .rest = text };
}

/// True when `v` can be written unquoted.
pub fn bare(v: []const u8) bool {
    if (v.len == 0 or v[0] == '#' or v[0] == '|') return false;
    for (v) |c| if (!isBare(c)) return false;
    return true;
}

/// A bare-value byte (format §2): not whitespace, quote, backslash or control.
pub fn isBare(c: u8) bool {
    return c > 0x20 and c != 0x7f and c != '"' and c != '\\';
}

// ---- private ----

fn isSpace(c: u8) bool {
    return c == ' ' or c == '\t';
}
