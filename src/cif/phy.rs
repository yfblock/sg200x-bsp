//! PHY 层配置

use super::*;

/// MIPI PHY 状态
#[derive(Debug, Clone, Copy)]
pub struct MipiPhyState {
    pub clk_hs_state: bool,
    pub clk_ulps_state: bool,
    pub clk_stop_state: bool,
    pub clk_err_state: bool,
    pub p1_clk_hs_state: bool,
    pub p1_clk_ulps_state: bool,
    pub p1_clk_stop_state: bool,
    pub p1_clk_err_state: bool,
    pub d0_datahs_state: u8,
    pub d1_datahs_state: u8,
    pub d2_datahs_state: u8,
    pub d3_datahs_state: u8,
    pub deskew_state: u8,
    pub p1_deskew_state: u8,
}

impl From<u32> for MipiPhyState {
    fn from(raw: u32) -> Self {
        Self {
            clk_hs_state: (raw & (1 << 0)) != 0,
            clk_ulps_state: (raw & (1 << 1)) != 0,
            clk_stop_state: (raw & (1 << 2)) != 0,
            clk_err_state: (raw & (1 << 3)) != 0,
            p1_clk_hs_state: (raw & (1 << 4)) != 0,
            p1_clk_ulps_state: (raw & (1 << 5)) != 0,
            p1_clk_stop_state: (raw & (1 << 6)) != 0,
            p1_clk_err_state: (raw & (1 << 7)) != 0,
            d0_datahs_state: ((raw >> 8) & 0x7) as u8,
            d1_datahs_state: ((raw >> 12) & 0x7) as u8,
            d2_datahs_state: ((raw >> 16) & 0x7) as u8,
            d3_datahs_state: ((raw >> 20) & 0x7) as u8,
            deskew_state: ((raw >> 24) & 0x3) as u8,
            p1_deskew_state: ((raw >> 26) & 0x3) as u8,
        }
    }
}

/// 设置 Lane ID
pub fn set_lane_id(ctx: &CifCtx, lane: LaneId, select: u32, pn_swap: bool) {
    log::debug!(
        "Set lane ID: lane={:?}, select={}, pn_swap={}",
        lane,
        select,
        pn_swap
    );
    // 实际的寄存器写入操作
}

/// 设置 Lane Deskew
pub fn set_lane_deskew(ctx: &CifCtx, lane: PhyLaneId, phase: u8) {
    log::debug!("Set lane deskew: lane={:?}, phase={}", lane, phase);
    // 实际的寄存器写入操作
}

/// 获取 Lane 数据
pub fn get_lane_data(ctx: &CifCtx, lane: PhyLaneId) -> u8 {
    // 读取 Lane 数据寄存器
    0
}

/// 设置时钟边沿
pub fn set_clk_edge(ctx: &CifCtx, lane: PhyLaneId, edge: CifClkEdge) {
    log::debug!("Set clock edge: lane={:?}, edge={:?}", lane, edge);
    // 实际的寄存器写入操作
}

/// 设置时钟方向
pub fn set_clk_dir(ctx: &CifCtx, dir: CifClkDir) {
    log::debug!("Set clock direction: {:?}", dir);
    // 实际的寄存器写入操作
}

/// 获取 CSI PHY 状态
pub fn get_csi_phy_state(ctx: &CifCtx) -> MipiPhyState {
    // 读取 PHY 状态寄存器
    MipiPhyState::from(0)
}

/// Lane Skew 相位配置
pub const LANE_PHASE_DEFAULT: [i32; 5] = [-1, 0x10, 0x10, 0x10, 0x10];

/// 判断 Lane 是否在 Port1
#[inline]
pub fn lane_is_port1(lane: i16) -> bool {
    lane > 2
}

/// 判断两个 Lane 是否在同一个 Port
#[inline]
pub fn is_same_port(lane1: i16, lane2: i16) -> bool {
    lane_is_port1(lane1) == lane_is_port1(lane2)
}

/// Lane Skew 类型
#[derive(Debug, Clone, Copy)]
pub enum LaneSkewType {
    CrossClk,
    CrossDataNear,
    CrossDataFar,
    Clk,
    Data,
}

impl LaneSkewType {
    /// 获取对应的相位值
    pub fn phase(&self) -> i32 {
        match self {
            Self::CrossClk => -1,
            Self::CrossDataNear => 0x10,
            Self::CrossDataFar => 0x10,
            Self::Clk => -1,
            Self::Data => 0x10,
        }
    }
}
