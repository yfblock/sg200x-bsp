//! JPEG 头解析（SOF / DHT / DQT / SOS）。

use super::regs::{FORMAT_400, FORMAT_420, FORMAT_422, FORMAT_224, FORMAT_444};

pub struct JpegHeaderInfo {
    pub width: u32,
    pub height: u32,
    pub num_components: u32,
    pub format: u32,
    pub ecs_offset: usize,
    pub restart_interval: u32,
    pub dc_huff_tbl: [usize; 3],
    pub ac_huff_tbl: [usize; 3],
    pub quant_tbl: [usize; 3],
    pub huff_tables: [HuffTable; 4],
    pub quant_tables: [QuantTable; 4],
    pub huff_table_count: usize,
    pub quant_table_count: usize,
}

pub struct HuffTable {
    pub bits: [u8; 16],
    pub values: [u8; 256],
    pub num_values: usize,
    pub min_codes: [u32; 16],
    pub max_codes: [u32; 16],
    pub ptrs: [u8; 16],
}

impl HuffTable {
    pub fn new() -> Self {
        Self {
            bits: [0; 16],
            values: [0; 256],
            num_values: 0,
            min_codes: [0xFFFF; 16],
            max_codes: [0xFFFF; 16],
            ptrs: [0xFF; 16],
        }
    }

    pub fn sign_extend_16(huff_data: u32) -> u32 {
        if huff_data & 0x8000 != 0 {
            0xFFFF
        } else {
            0
        }
    }

    pub fn sign_extend_8(huff_data: u32) -> u32 {
        if huff_data & 0x80 != 0 {
            0xFFFFFF
        } else {
            0
        }
    }

    pub fn generate(&mut self) {
        let mut ptr_cnt: usize = 0;
        let mut huff_code: u32 = 0;
        let mut data_flag = false;

        for i in 0..16 {
            if self.bits[i] != 0 {
                self.ptrs[i] = ptr_cnt as u8;
                ptr_cnt += self.bits[i] as usize;
                self.min_codes[i] = huff_code;
                self.max_codes[i] = huff_code + (self.bits[i] as u32 - 1);
                data_flag = true;
            } else {
                self.ptrs[i] = 0xFF;
                self.min_codes[i] = 0xFFFF;
                self.max_codes[i] = 0xFFFF;
            }

            if data_flag {
                if self.bits[i] == 0 {
                    huff_code <<= 1;
                } else {
                    huff_code = (self.max_codes[i] + 1) << 1;
                }
            }
        }
    }
}

pub struct QuantTable {
    pub values: [u16; 64],
}

impl QuantTable {
    pub fn new() -> Self {
        Self { values: [0; 64] }
    }
}

impl JpegHeaderInfo {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            num_components: 0,
            format: FORMAT_420,
            ecs_offset: 0,
            restart_interval: 0,
            dc_huff_tbl: [0; 3],
            ac_huff_tbl: [0; 3],
            quant_tbl: [0; 3],
            huff_tables: [HuffTable::new(), HuffTable::new(), HuffTable::new(), HuffTable::new()],
            quant_tables: [
                QuantTable::new(),
                QuantTable::new(),
                QuantTable::new(),
                QuantTable::new(),
            ],
            huff_table_count: 0,
            quant_table_count: 0,
        }
    }
}

pub fn parse_jpeg_header(data: &[u8]) -> Result<JpegHeaderInfo, &'static str> {
    let mut i = 0;
    let mut header_info = JpegHeaderInfo::new();

    while i < data.len().saturating_sub(1) {
        if data[i] == 0xFF {
            let marker = data[i + 1];

            if marker == 0xFF {
                i += 1;
                continue;
            }
            if marker == 0x00 {
                i += 2;
                continue;
            }

            match marker {
                0xC0 | 0xC2 => {
                    if i + 10 >= data.len() {
                        return Err("SOF too short");
                    }

                    header_info.height = ((data[i + 5] as u32) << 8) | (data[i + 6] as u32);
                    header_info.width = ((data[i + 7] as u32) << 8) | (data[i + 8] as u32);
                    header_info.num_components = data[i + 9] as u32;

                    if header_info.num_components == 3 {
                        let comp_start = i + 10;
                        if comp_start + 9 <= data.len() {
                            let h1 = (data[comp_start + 1] >> 4) & 0x0F;
                            let v1 = data[comp_start + 1] & 0x0F;
                            let h2 = (data[comp_start + 4] >> 4) & 0x0F;
                            let v2 = data[comp_start + 4] & 0x0F;

                            header_info.quant_tbl[0] = data[comp_start + 2] as usize;
                            header_info.quant_tbl[1] = data[comp_start + 5] as usize;
                            header_info.quant_tbl[2] = data[comp_start + 8] as usize;

                            header_info.format = if h1 == 2 && v1 == 2 && h2 == 1 && v2 == 1 {
                                FORMAT_420
                            } else if h1 == 2 && v1 == 1 {
                                FORMAT_422
                            } else if h1 == 1 && v1 == 2 {
                                FORMAT_224
                            } else {
                                FORMAT_444
                            };
                        }
                    } else {
                        header_info.format = FORMAT_400;
                    }

                    if i + 3 < data.len() {
                        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                        i += 2 + length;
                        continue;
                    }
                }
                0xC4 => {
                    if i + 3 < data.len() {
                        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                        parse_dht(data, i + 4, i + 2 + length, &mut header_info)?;
                        i += 2 + length;
                        continue;
                    }
                }
                0xDA => {
                    if i + 3 < data.len() {
                        let sos_length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);

                        if i + 5 < data.len() {
                            let num_scan_components = data[i + 4] as usize;
                            let mut comp_offset = i + 5;
                            for comp_idx in 0..num_scan_components.min(3) {
                                if comp_offset + 2 <= data.len() {
                                    let tables = data[comp_offset + 1];
                                    header_info.dc_huff_tbl[comp_idx] =
                                        ((tables >> 4) & 0x0F) as usize;
                                    header_info.ac_huff_tbl[comp_idx] = (tables & 0x0F) as usize;
                                    comp_offset += 2;
                                }
                            }
                        }

                        header_info.ecs_offset = i + 2 + sos_length;
                        return Ok(header_info);
                    }
                }
                0xDB => {
                    if i + 3 < data.len() {
                        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                        parse_dqt(data, i + 4, i + 2 + length, &mut header_info)?;
                        i += 2 + length;
                        continue;
                    }
                }
                0xDD => {
                    if i + 6 <= data.len() {
                        header_info.restart_interval =
                            ((data[i + 4] as u32) << 8) | (data[i + 5] as u32);
                    }
                    if i + 3 < data.len() {
                        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                        i += 2 + length;
                        continue;
                    }
                }
                0xD8 => {
                    i += 2;
                    continue;
                }
                0xD9 => break,
                _ => {
                    if marker >= 0xC0 && i + 3 < data.len() {
                        let length = ((data[i + 2] as usize) << 8) | (data[i + 3] as usize);
                        i += 2 + length;
                        continue;
                    }
                    i += 2;
                    continue;
                }
            }
        }
        i += 1;
    }

    Err("SOS not found")
}

fn parse_dht(
    data: &[u8],
    start: usize,
    end: usize,
    header_info: &mut JpegHeaderInfo,
) -> Result<(), &'static str> {
    let mut offset = start;

    while offset < end && offset + 1 < data.len() {
        let tc_th = data[offset];
        let tc = (tc_th >> 4) & 0x0F;
        let th = tc_th & 0x0F;
        let table_idx: usize = (((th & 1) << 1) | (tc & 1)) as usize;

        let mut num_values = 0;
        for j in 0..16 {
            if offset + 1 + j < data.len() {
                header_info.huff_tables[table_idx].bits[j] = data[offset + 1 + j];
                num_values += data[offset + 1 + j] as usize;
            }
        }

        for j in 0..num_values {
            if offset + 17 + j < data.len() {
                header_info.huff_tables[table_idx].values[j] = data[offset + 17 + j];
            }
        }
        header_info.huff_tables[table_idx].num_values = num_values;
        header_info.huff_tables[table_idx].generate();

        if table_idx >= header_info.huff_table_count {
            header_info.huff_table_count = table_idx + 1;
        }

        offset += 17 + num_values;
    }

    Ok(())
}

fn parse_dqt(
    data: &[u8],
    start: usize,
    end: usize,
    header_info: &mut JpegHeaderInfo,
) -> Result<(), &'static str> {
    let mut offset = start;

    while offset < end && offset + 1 < data.len() {
        let pq_tq = data[offset];
        let tq: usize = (pq_tq & 0x0F) as usize;

        if pq_tq >> 4 == 0 {
            for j in 0..64 {
                if offset + 1 + j < data.len() {
                    header_info.quant_tables[tq].values[j] = data[offset + 1 + j] as u16;
                }
            }
            offset += 1 + 64;
        } else {
            for j in 0..64 {
                if offset + 1 + j * 2 + 1 < data.len() {
                    header_info.quant_tables[tq].values[j] = ((data[offset + 1 + j * 2] as u16) << 8)
                        | (data[offset + 1 + j * 2 + 1] as u16);
                }
            }
            offset += 1 + 128;
        }

        if tq >= header_info.quant_table_count {
            header_info.quant_table_count = tq + 1;
        }
    }

    Ok(())
}
