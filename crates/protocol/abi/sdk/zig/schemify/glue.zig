//! glue.zig: the module's exports (docs/spec/plugin-abi.md §2): a plugin type in → the
//! `schemify_abi`, `schemify_alloc`, `schemify_on_event` and optional `schemify_call`
//! exports over two static buffers out.
//!
//! Contents: `pub fn exportPlugin(comptime Plugin: type) void`
const numbers = @import("numbers.zig");
const imports = @import("imports.zig");
const Event = @import("event.zig").Event;
const Out = @import("out.zig").Out;
const Code = numbers.Code;

/// Exports the module's entry points for `Plugin`, which declares
/// `pub fn onEvent(ev: Event, out: *Out) void` and, if it contributes `functions`,
/// `pub fn call(fn_id: u32, args: []const u8, result: *Out) Error!void`: a pure function
/// (every import is denied) writing one value; `error.Range` for an unknown `fn_id`.
/// Call it once, from a `comptime` block in the plugin's root file.
pub fn exportPlugin(comptime Plugin: type) void {
    const G = Glue(Plugin);
    @export(&G.abi, .{ .name = "schemify_abi" });
    @export(&G.alloc, .{ .name = "schemify_alloc" });
    @export(&G.onEvent, .{ .name = "schemify_on_event" });
    if (@hasDecl(Plugin, "call")) @export(&G.call, .{ .name = "schemify_call" });
}

// ---- private ----

fn Glue(comptime Plugin: type) type {
    return struct {
        const input_max = if (@hasDecl(Plugin, "schemify_input_max")) Plugin.schemify_input_max else 1 << 20;
        const output_max = if (@hasDecl(Plugin, "schemify_output_max")) Plugin.schemify_output_max else 1 << 20;
        var in: [input_max]u8 = undefined;
        var out: [output_max]u8 = undefined;
        var in_len: u32 = 0;

        fn pack(ptr: usize, len: usize) u64 {
            return @as(u64, ptr) << 32 | len;
        }

        /// The host wrote `len` bytes at `ptr`, and only `alloc` hands out that address.
        fn ours(ptr: u32, len: u32) bool {
            return ptr == @intFromPtr(&in) and len == in_len;
        }

        fn abi() callconv(.c) u32 {
            return numbers.abi_version;
        }

        fn alloc(len: u32) callconv(.c) u32 {
            if (len > in.len) return 0; // the host reports E_SPACE
            in_len = len;
            return @intFromPtr(&in);
        }

        fn onEvent(ptr: u32, len: u32) callconv(.c) u64 {
            if (!ours(ptr, len)) return 0;
            var o: Out = .init(&out);
            if (Event.parse(in[0..len])) |ev| Plugin.onEvent(ev, &o) else |_| o.log(.@"error", "unparsable event");
            if (o.overflow) {
                o = .init(&out);
                o.log(.@"error", "output over schemify_output_max, dropped");
            }
            return if (o.len > 0) pack(@intFromPtr(&out), o.len) else 0;
        }

        fn call(fn_id: u32, ptr: u32, len: u32) callconv(.c) u64 {
            if (!ours(ptr, len)) return pack(0, @intFromEnum(Code.range));
            var o: Out = .init(&out);
            Plugin.call(fn_id, in[0..len], &o) catch |e| return pack(0, @intFromEnum(imports.codeOf(e)));
            if (o.overflow) return pack(0, @intFromEnum(Code.space));
            if (o.len == 0) return pack(0, @intFromEnum(Code.parse));
            return pack(@intFromPtr(&out), o.len);
        }
    };
}
