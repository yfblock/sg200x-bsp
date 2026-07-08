//! SD 卡基本信息（RCA / CSD / 容量）及其解析。

/// SD 卡基本信息（在 [`crate::sdmmc::Sdmmc::init`] 之后填充，可由
/// [`crate::sdmmc::Sdmmc::card_info`] 读取）。
#[derive(Debug, Clone, Copy, Default)]
pub struct SdCardInfo {
    /// 卡相对地址 (RCA)：CMD3 返回的高 16 位
    pub rca: u32,
    /// CSD 寄存器原始内容（按 SDHCI Response 寄存器排列：
    /// `csd_raw[0]` = response0 = CSD\[39:8\]，
    /// `csd_raw[1]` = response1 = CSD\[71:40\]，
    /// `csd_raw[2]` = response2 = CSD\[103:72\]，
    /// `csd_raw[3]` = response3 = CSD\[135:104\]，
    /// 高 8 位为 R2 起始位/保留位，CSD\[7:0\]（CRC+stop）已被硬件丢弃）
    pub csd_raw: [u32; 4],
    /// CSD_STRUCTURE 字段：0=v1.0(SDSC)，1=v2.0(SDHC/SDXC)，2=v3.0(SDUC)
    pub csd_structure: u8,
    /// SD 卡容量（字节）；解析失败或未初始化时为 0
    pub capacity_bytes: u64,
}

/// 根据 R2 响应寄存器（response0..response3）解析出 SD 卡的容量等基本信息。
///
/// SDHCI 把 R2 响应的 \[135:8\] 直接放进 RESP\_REG\[127:0\]：
///
/// ```text
/// response3[31:0] = R2[135:104] = (start+trans+rsv) | CSD[127:104]
/// response2[31:0] = R2[103:72]  = CSD[103:72]
/// response1[31:0] = R2[71:40]   = CSD[71:40]
/// response0[31:0] = R2[39:8]    = CSD[39:8]
/// ```
///
/// 即 `RESP_REG[i] = CSD[i + 8]`（CSD\[7:0\] 的 CRC + stop bit 已被硬件丢弃）。
/// 由此推出本函数中各字段的取位方式。
///
/// 三种 CSD 结构：
/// - **v1.0 (SDSC)**：容量 = `(C_SIZE + 1) * 2^(C_SIZE_MULT + 2) * 2^READ_BL_LEN`
/// - **v2.0 (SDHC/SDXC)**：容量 = `(C_SIZE + 1) * 512KiB`，C\_SIZE 为 22 bit
/// - **v3.0 (SDUC)**：容量 = `(C_SIZE + 1) * 512KiB`，C\_SIZE 为 28 bit
pub fn parse_sd_card_info(rca: u32, csd_raw: [u32; 4]) -> SdCardInfo {
    let r0 = csd_raw[0];
    let r1 = csd_raw[1];
    let r2 = csd_raw[2];
    let r3 = csd_raw[3];

    // CSD[127:126] —— RESP_REG[119:118] —— response3 bit[23:22]
    let csd_structure = ((r3 >> 22) & 0x3) as u8;

    let capacity_bytes = match csd_structure {
        0 => {
            // CSD v1.0 (SDSC)
            // READ_BL_LEN: CSD[83:80] = RESP_REG[75:72] = response2[11:8]
            let read_bl_len = (r2 >> 8) & 0xF;
            // C_SIZE: CSD[73:62] (12 bit)，跨 response2/response1
            //   高 2 bit  CSD[73:72] = response2[1:0]
            //   低 10 bit CSD[71:62] = response1[31:22]
            let c_size = ((r2 & 0x3) << 10) | ((r1 >> 22) & 0x3FF);
            // C_SIZE_MULT: CSD[49:47] = response1[9:7]
            let c_size_mult = (r1 >> 7) & 0x7;
            let mult = 1u64 << (c_size_mult + 2);
            let blocknr = (c_size as u64 + 1) * mult;
            let block_len = 1u64 << read_bl_len;
            blocknr * block_len
        }
        1 => {
            // CSD v2.0 (SDHC / SDXC)
            // C_SIZE: CSD[69:48] = response1[29:8]，22 bit
            let c_size = (r1 >> 8) & 0x3F_FFFF;
            (c_size as u64 + 1) * 512 * 1024
        }
        2 => {
            // CSD v3.0 (SDUC)
            // C_SIZE: CSD[75:48] (28 bit)
            //   高 4 bit  CSD[75:72] = response2[3:0]
            //   低 24 bit CSD[71:48] = response1[31:8]
            let c_size = ((r2 & 0xF) << 24) | ((r1 >> 8) & 0xFF_FFFF);
            (c_size as u64 + 1) * 512 * 1024
        }
        _ => 0,
    };

    let _ = r0; // 仅保留作完整存档；CSD v1/v2/v3 容量都不依赖 response0。

    SdCardInfo {
        rca,
        csd_raw,
        csd_structure,
        capacity_bytes,
    }
}
