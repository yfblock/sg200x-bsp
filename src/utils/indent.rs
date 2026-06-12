//! 日志行前缀缩进（按树深度，每级 2 空格）。

/// 返回深度 `depth` 对应的前缀空格串（`depth=0` 为空串）。
///
/// 表内最多 11 级；超出时返回最深一级（22 空格）的缩进。
#[inline]
pub fn log_indent(depth: u8) -> &'static str {
    const T: [&str; 12] = [
        "",
        "  ",
        "    ",
        "      ",
        "        ",
        "          ",
        "            ",
        "              ",
        "                ",
        "                  ",
        "                    ",
        "                      ",
    ];
    T.get(depth as usize).copied().unwrap_or("                      ")
}
