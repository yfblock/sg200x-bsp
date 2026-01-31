//! CIF 类型定义

/// CIF 接口类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifType {
    Csi = 0,
    Sublvds,
    Hispi,
    Ttl,
    BtDmux,
}

/// 输入模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum InputMode {
    Mipi = 0,
    Sublvds,
    Hispi,
    Cmos,
    Bt1120,
    Bt601_19bVhs,
    Bt656_9b,
    Custom0,
    BtDemux,
}

/// RX MAC 时钟
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RxMacClk {
    Clk200M = 0,
    Clk300M,
    Clk400M,
    Clk500M,
    Clk600M,
}

/// 相机 PLL 频率
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CamPllFreq {
    None = 0,
    Freq37P125M,
    Freq25M,
    Freq27M,
    Freq24M,
    Freq26M,
}

/// 原始数据类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum RawDataType {
    Raw8Bit = 0,
    Raw10Bit,
    Raw12Bit,
    Yuv422_8Bit,
    Yuv422_10Bit,
}

/// MIPI HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MipiHdrMode {
    None = 0,
    Vc,
    Dt,
    Dol,
    Manual,
}

/// HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HdrMode {
    None = 0,
    Mode2F,
    Mode3F,
    Dol2F,
    Dol3F,
}

/// LVDS 同步模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LvdsSyncMode {
    Sof = 0,
    Sav,
}

/// LVDS 位端序
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LvdsBitEndian {
    Little = 0,
    Big,
}

/// LVDS VSync 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LvdsVsyncType {
    Normal = 0,
    Share,
    Hconnect,
}

/// LVDS FID 类型
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LvdsFidType {
    None = 0,
    InSav,
}

/// TTL 引脚功能
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlPinFunc {
    Vs = 0,
    Hs,
    Vde,
    Hde,
    D0, D1, D2, D3, D4, D5, D6, D7,
    D8, D9, D10, D11, D12, D13, D14, D15,
}

/// TTL VI 源
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlViSrc {
    Vi0 = 0,
    Vi1,
    Vi2,
}

/// BT Demux 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum BtDemuxMode {
    Disable = 0,
    Demux2,
    Demux3,
    Demux4,
}

/// 时钟边沿
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ClkEdge {
    Up = 0,
    Down,
}

/// 输出 MSB
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum OutputMsb {
    Normal = 0,
    Reverse,
}

/// GPIO 标志
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum GpioFlags {
    ActiveLow = 0,
    ActiveHigh,
}

/// Sub-LVDS 格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SublvdsFmt {
    Bit8 = 0,
    Bit10,
    Bit12,
}

/// Sub-LVDS HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum SublvdsHdr {
    Pat1 = 0,
    Pat2,
}

/// HiSPI 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum HispiMode {
    PktSp = 0,
    StreamSp,
}

/// TTL 传感器格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlSensorFmt {
    Bit8 = 0,
    Bit10,
    Bit12,
    Bit16,
}

/// TTL BT 格式输出
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlBtFmtOut {
    Cbycry = 0,
    Crycby,
    Ycbycr,
    Ycrycb,
}

/// TTL 格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlFmt {
    SyncPat9bBt656 = 0,
    SyncPat17bBt1120,
    Vhs11bBt601,
    Vhs19bBt601,
    Vde11bBt601,
    Vde19bBt601,
    Vsde11bBt601,
    Vsde19bBt601,
    SyncPatSensor = 8,
    VhsSensor = 10,
    VdeSensor = 12,
    VsdeSensor = 14,
    Custom0,
}

/// CSI 格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsiFmt {
    Yuv422_8b = 0,
    Yuv422_10b,
    Raw8,
    Raw10,
    Raw12,
}

/// CSI VSync 生成模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsiVsGenMode {
    Fs = 0,
    Fe,
    FsFe,
}

/// CSI HDR 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsiHdrMode {
    Vc = 0,
    Id,
    Dt,
    Dol,
}

/// CIF 端序
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifEndian {
    Msb = 0,
    Lsb,
}

/// PHY Lane ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum PhyLaneId {
    Lane0 = 0,
    Lane1,
    Lane2,
    Lane3,
    Lane4,
    Lane5,
}

/// Lane ID
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum LaneId {
    Clk = 0,
    Lane0,
    Lane1,
    Lane2,
    Lane3,
}

/// CSI 解码格式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CsiDecodeFmt {
    Yuv422_8 = 0,
    Yuv422_10,
    Raw8,
    Raw10,
    Raw12,
}

/// CIF 时钟边沿
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifClkEdge {
    Rising = 0,
    Falling,
}

/// CIF 时钟方向
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum CifClkDir {
    P0ToP1 = 0,
    P1ToP0,
    Freerun,
}

/// TTL VI 模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum TtlViMode {
    Raw = 1,
    Bt601,
    Bt656,
    Bt1120,
}

/// 图像尺寸
#[derive(Debug, Clone, Copy, Default)]
pub struct ImgSize {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// MCLK PLL 配置
#[derive(Debug, Clone, Copy)]
pub struct MclkPll {
    pub cam: u32,
    pub freq: CamPllFreq,
}

/// DPHY 配置
#[derive(Debug, Clone, Copy)]
pub struct Dphy {
    pub enable: bool,
    pub hs_settle: u8,
}

/// MIPI Demux 信息
#[derive(Debug, Clone, Copy)]
pub struct MipiDemuxInfo {
    pub demux_en: bool,
    pub vc_mapping: [u8; 4],
}

/// MIPI 设备属性
#[derive(Debug, Clone, Copy)]
pub struct MipiDevAttr {
    pub raw_data_type: RawDataType,
    pub lane_id: [i16; 5],
    pub hdr_mode: MipiHdrMode,
    pub data_type: [i16; 2],
    pub pn_swap: [i8; 5],
    pub dphy: Dphy,
    pub demux: MipiDemuxInfo,
}

/// LVDS VSync 类型配置
#[derive(Debug, Clone, Copy)]
pub struct LvdsVsyncTypeS {
    pub sync_type: LvdsVsyncType,
    pub hblank1: u16,
    pub hblank2: u16,
}

/// LVDS FID 类型配置
#[derive(Debug, Clone, Copy)]
pub struct LvdsFidTypeS {
    pub fid: LvdsFidType,
}

/// LVDS 设备属性
#[derive(Debug, Clone, Copy)]
pub struct LvdsDevAttr {
    pub hdr_mode: HdrMode,
    pub sync_mode: LvdsSyncMode,
    pub raw_data_type: RawDataType,
    pub data_endian: LvdsBitEndian,
    pub sync_code_endian: LvdsBitEndian,
    pub lane_id: [i16; 5],
    pub sync_code: [[[i16; 4]; 3]; 4],
    pub vsync_type: LvdsVsyncTypeS,
    pub fid_type: LvdsFidTypeS,
    pub pn_swap: [i8; 5],
}

/// 手动 HDR 属性
#[derive(Debug, Clone, Copy, Default)]
pub struct ManualHdrAttr {
    pub manual_en: bool,
    pub l2s_distance: u16,
    pub lsef_length: u16,
    pub discard_padding_lines: bool,
    pub update: bool,
}

/// TTL 设备属性
#[derive(Debug, Clone, Copy)]
pub struct TtlDevAttr {
    pub vi: TtlViSrc,
    pub func: [i8; 19],
    pub v_bp: u16,
    pub h_bp: u16,
}

/// BT Demux 同步码
#[derive(Debug, Clone, Copy)]
pub struct BtDemuxSync {
    pub sav_vld: u8,
    pub sav_blk: u8,
    pub eav_vld: u8,
    pub eav_blk: u8,
}

/// BT Demux 属性
#[derive(Debug, Clone, Copy)]
pub struct BtDemuxAttr {
    pub func: [i8; 19],
    pub v_fp: u16,
    pub h_fp: u16,
    pub v_bp: u16,
    pub h_bp: u16,
    pub mode: BtDemuxMode,
    pub sync_code_part_a: [u8; 3],
    pub sync_code_part_b: [BtDemuxSync; 4],
    pub yc_exchg: i8,
}

/// 组合设备属性
#[derive(Clone, Copy)]
pub struct ComboDevAttr {
    pub input_mode: InputMode,
    pub mac_clk: RxMacClk,
    pub mclk: MclkPll,
    pub devno: u32,
    pub img_size: ImgSize,
    pub hdr_manu: ManualHdrAttr,
    pub mipi_attr: Option<MipiDevAttr>,
    pub lvds_attr: Option<LvdsDevAttr>,
    pub ttl_attr: Option<TtlDevAttr>,
    pub bt_demux_attr: Option<BtDemuxAttr>,
}

impl Default for ComboDevAttr {
    fn default() -> Self {
        Self {
            input_mode: InputMode::Mipi,
            mac_clk: RxMacClk::Clk400M,
            mclk: MclkPll {
                cam: 0,
                freq: CamPllFreq::None,
            },
            devno: 0,
            img_size: ImgSize::default(),
            hdr_manu: ManualHdrAttr::default(),
            mipi_attr: None,
            lvds_attr: None,
            ttl_attr: None,
            bt_demux_attr: None,
        }
    }
}

/// Sub-LVDS 同步码
#[derive(Debug, Clone, Copy, Default)]
pub struct SublvdsSyncCode {
    pub n0_lef_sav: u16,
    pub n0_lef_eav: u16,
    pub n0_sef_sav: u16,
    pub n0_sef_eav: u16,
    pub n1_lef_sav: u16,
    pub n1_lef_eav: u16,
    pub n1_sef_sav: u16,
    pub n1_sef_eav: u16,
    pub n0_lsef_sav: u16,
    pub n0_lsef_eav: u16,
    pub n1_lsef_sav: u16,
    pub n1_lsef_eav: u16,
}

/// HiSPI 同步码
#[derive(Debug, Clone, Copy, Default)]
pub struct HispiSyncCode {
    pub t1_sol: u16,
    pub t1_eol: u16,
    pub t2_sol: u16,
    pub t2_eol: u16,
    pub t1_sof: u16,
    pub t1_eof: u16,
    pub t2_sof: u16,
    pub t2_eof: u16,
    pub vsync_gen: u16,
}

/// 同步码
#[derive(Debug, Clone, Copy)]
pub struct SyncCode {
    pub norm_bk_sav: u16,
    pub norm_bk_eav: u16,
    pub norm_sav: u16,
    pub norm_eav: u16,
    pub n0_bk_sav: u16,
    pub n0_bk_eav: u16,
    pub n1_bk_sav: u16,
    pub n1_bk_eav: u16,
    pub slvds: Option<SublvdsSyncCode>,
    pub hispi: Option<HispiSyncCode>,
}

impl Default for SyncCode {
    fn default() -> Self {
        Self {
            norm_bk_sav: 0xAB0,
            norm_bk_eav: 0xB60,
            norm_sav: 0x800,
            norm_eav: 0x9D0,
            n0_bk_sav: 0x2B0,
            n0_bk_eav: 0x360,
            n1_bk_sav: 0x6B0,
            n1_bk_eav: 0x760,
            slvds: None,
            hispi: None,
        }
    }
}

/// Sub-LVDS 参数
#[derive(Debug, Clone, Copy)]
pub struct ParamSublvds {
    pub v_front_porch: u16,
    pub lane_num: u16,
    pub hdr_hblank: [u16; 2],
    pub h_size: u16,
    pub hdr_mode: SublvdsHdr,
    pub endian: CifEndian,
    pub wrap_endian: CifEndian,
    pub fmt: SublvdsFmt,
    pub hdr_v_fp: u16,
    pub sync_code: SyncCode,
}

/// HiSPI 参数
#[derive(Debug, Clone, Copy)]
pub struct ParamHispi {
    pub lane_num: u16,
    pub h_size: u16,
    pub mode: HispiMode,
    pub endian: CifEndian,
    pub wrap_endian: CifEndian,
    pub fmt: SublvdsFmt,
    pub sync_code: SyncCode,
}

/// TTL 参数
#[derive(Debug, Clone, Copy)]
pub struct ParamTtl {
    pub fmt: TtlFmt,
    pub sensor_fmt: TtlSensorFmt,
    pub fmt_out: TtlBtFmtOut,
    pub width: u16,
    pub height: u16,
    pub v_bp: u16,
    pub h_bp: u16,
    pub clk_inv: u32,
    pub vi_sel: u32,
    pub vi_from: TtlViSrc,
}

/// CSI 参数
#[derive(Debug, Clone, Copy)]
pub struct ParamCsi {
    pub lane_num: u16,
    pub fmt: CsiFmt,
    pub vs_gen_mode: CsiVsGenMode,
    pub hdr_mode: CsiHdrMode,
    pub data_type: [u16; 2],
    pub decode_type: u16,
    pub vc_mapping: [u8; 4],
}

/// BT Demux 参数
#[derive(Debug, Clone, Copy)]
pub struct ParamBtdemux {
    pub fmt: TtlFmt,
    pub demux: BtDemuxMode,
    pub width: u16,
    pub height: u16,
    pub v_fp: u16,
    pub h_fp: u16,
    pub v_bp: u16,
    pub h_bp: u16,
    pub clk_inv: u32,
    pub sync_code_part_a: [u8; 3],
    pub sync_code_part_b: [BtDemuxSync; 4],
    pub yc_exchg: u8,
}

/// CIF 配置联合体
#[derive(Clone, Copy)]
pub enum CifCfg {
    Sublvds(ParamSublvds),
    Hispi(ParamHispi),
    Ttl(ParamTtl),
    Csi(ParamCsi),
    Btdemux(ParamBtdemux),
}

/// CIF 参数
#[derive(Clone, Copy)]
pub struct CifParam {
    pub cif_type: CifType,
    pub cfg: CifCfg,
    pub hdr_manual: bool,
    pub hdr_shift: u16,
    pub hdr_vsize: u16,
    pub hdr_rm_padding: u16,
    pub info_line_num: u16,
    pub hdr_en: bool,
}

impl Default for CifParam {
    fn default() -> Self {
        Self {
            cif_type: CifType::Csi,
            cfg: CifCfg::Csi(ParamCsi {
                lane_num: 0,
                fmt: CsiFmt::Raw10,
                vs_gen_mode: CsiVsGenMode::Fs,
                hdr_mode: CsiHdrMode::Vc,
                data_type: [0; 2],
                decode_type: 0,
                vc_mapping: [0, 1, 2, 3],
            }),
            hdr_manual: false,
            hdr_shift: 0,
            hdr_vsize: 0,
            hdr_rm_padding: 0,
            info_line_num: 0,
            hdr_en: false,
        }
    }
}
