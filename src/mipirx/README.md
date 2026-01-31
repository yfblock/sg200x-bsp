# MIPI RX 驱动使用指南

## 概述

MIPI RX (Mobile Industry Processor Interface Receiver) 模块用于接收由 CMOS sensor 传送的视频数据，支持 MIPI D-PHY、Sub-LVDS、HiSPi 等不同的串行视频信号输入。

## 功能特性

- 支持 MIPI D-PHY ver2.1
- 可同时支持 2 路 sensor 输入
- 单一 sensor 最大支持 5M (2688x1944, 2880x1620) @30fps HDR 或 @60fps 线性输入
- 双路 sensor 最大支持 FHD (1920x1080) @60fps HDR 或线性输入
- 单路最多支持 4-Lane MIPI D-PHY 接口，最大支持 1.5Gbps/Lane
- 单路最多支持 4-Lane Sub-LVDS/HiSPi 接口，最大支持 1.5Gbps/Lane
- 支持 RAW8/RAW10/RAW12/RAW16 数据类型
- 支持 YUV422 8-bit / YUV422 10-bit 数据类型
- 最多支持 2 帧 WDR，支持多种 WDR 时序

## 模块结构

```
mipirx/
├── mod.rs      # 主模块，提供统一的驱动接口
├── regs.rs     # 寄存器定义 (使用 tock-registers)
├── types.rs    # 类型定义和枚举
├── phy.rs      # PHY 层配置
├── csi.rs      # CSI 控制器配置
└── README.md   # 本文档
```

## 寄存器基地址

| 模块 | 基地址 | 描述 |
|------|--------|------|
| PHY Top | 0x0A0D_0000 | PHY 顶层控制寄存器 |
| DPHY 4L | 0x0A0D_0300 | 4-Lane DPHY 寄存器 |
| DPHY 2L | 0x0A0D_0600 | 2-Lane DPHY 寄存器 |
| CSI0 | 0x0A0C_2400 | CSI 控制器 0 寄存器 |
| CSI1 | 0x0A0C_4400 | CSI 控制器 1 寄存器 |

## 使用示例

### 1. 基本 MIPI CSI 配置 (单路 4-Lane)

```rust
use sg200x_bsp::mipirx::*;

// 创建 MIPI RX 驱动实例
let mut mipirx = unsafe { MipiRx::new() };

// 初始化
mipirx.init();

// 配置设备属性
let attr = MipiRxDevAttr {
    devno: 0,                       // 设备编号
    sensor_mode: SensorMode::Csi,   // MIPI CSI 模式
    lane_mode: LaneMode::Lane4,     // 4-Lane 模式
    data_type: RawDataType::Raw10,  // RAW10 数据格式
    hdr_mode: HdrMode::None,        // 无 HDR
    lane_id: [0, 1, 2, 3],          // Lane ID 配置
    clk_lane_sel: 0,                // 时钟 Lane 选择
    pn_swap: [false; 5],            // PN 交换配置
    img_width: 1920,                // 图像宽度
    img_height: 1080,               // 图像高度
};

// 应用配置
mipirx.configure(&attr).unwrap();

// 使能接收
mipirx.enable(0);
```

### 2. HDR 模式配置 (VC 模式)

```rust
use sg200x_bsp::mipirx::*;

let mut mipirx = unsafe { MipiRx::new() };
mipirx.init();

// 配置 HDR VC 模式
let attr = MipiRxDevAttr {
    devno: 0,
    sensor_mode: SensorMode::Csi,
    lane_mode: LaneMode::Lane4,
    data_type: RawDataType::Raw10,
    hdr_mode: HdrMode::Vc,          // 使用 VC 模式区分长短曝光
    lane_id: [0, 1, 2, 3],
    clk_lane_sel: 0,
    pn_swap: [false; 5],
    img_width: 1920,
    img_height: 1080,
};

mipirx.configure(&attr).unwrap();
mipirx.enable(0);
```

### 3. 双路 Sensor 配置 (1C2D + 1C2D)

```rust
use sg200x_bsp::mipirx::*;

let mut mipirx = unsafe { MipiRx::new() };
mipirx.init();

// 设置 PHY 为双端口模式
mipirx.set_phy_mode(PhyMode::Mode1C2D_1C2D);

// 配置第一路 (2-Lane)
let attr0 = MipiRxDevAttr {
    devno: 0,
    sensor_mode: SensorMode::Csi,
    lane_mode: LaneMode::Lane2,
    data_type: RawDataType::Raw10,
    hdr_mode: HdrMode::None,
    lane_id: [0, 1, -1, -1],        // 只使用 Lane 0, 1
    clk_lane_sel: 0,
    pn_swap: [false; 5],
    img_width: 1920,
    img_height: 1080,
};
mipirx.configure(&attr0).unwrap();

// 配置第二路 (2-Lane)
let attr1 = MipiRxDevAttr {
    devno: 1,
    sensor_mode: SensorMode::Csi,
    lane_mode: LaneMode::Lane2,
    data_type: RawDataType::Raw10,
    hdr_mode: HdrMode::None,
    lane_id: [2, 3, -1, -1],        // 使用 Lane 2, 3
    clk_lane_sel: 1,
    pn_swap: [false; 5],
    img_width: 1280,
    img_height: 720,
};
mipirx.configure(&attr1).unwrap();

// 使能两路
mipirx.enable(0);
mipirx.enable(1);
```

### 4. Sub-LVDS 模式配置

```rust
use sg200x_bsp::mipirx::*;

let mut mipirx = unsafe { MipiRx::new() };
mipirx.init();

// 配置 Sub-LVDS 属性
let attr = SubLvdsDevAttr {
    devno: 0,
    bit_mode: SubLvdsBitMode::Bit10,    // 10-bit 模式
    lane_enable: 0x0F,                   // 使能 4 个 Lane
    msb_first: true,                     // MSB 优先
    sav_1st: 0x3FF,                      // 同步码第一个符号
    sav_2nd: 0x000,                      // 同步码第二个符号
    sav_3rd: 0x000,                      // 同步码第三个符号
    img_width: 1920,
    img_height: 1080,
};

mipirx.configure_sublvds(&attr).unwrap();
mipirx.enable(0);
```

### 5. 状态查询和错误处理

```rust
use sg200x_bsp::mipirx::*;

let mipirx = unsafe { MipiRx::new() };

// 检查是否有错误
if mipirx.has_error(0) {
    // 获取详细状态
    if let Some(status) = mipirx.get_status(0) {
        if status.ecc_error {
            log::error!("ECC error detected");
        }
        if status.crc_error {
            log::error!("CRC error detected");
        }
        if status.fifo_full {
            log::error!("FIFO full");
        }
    }
    
    // 获取中断状态
    if let Some(int_status) = mipirx.get_interrupt_status(0) {
        log::info!("Interrupt status: {:?}", int_status);
    }
    
    // 清除中断
    mipirx.clear_interrupt(0, 0x1F);
}
```

### 6. 直接访问 PHY 和 CSI 寄存器

```rust
use sg200x_bsp::mipirx::*;

let mut mipirx = unsafe { MipiRx::new() };

// 直接访问 PHY
let phy = mipirx.phy_mut();
phy.set_phy_mode(PhyMode::Mode1C4D);
phy.configure_csi_lane_select(0, [0, 1, 2, 3]);
phy.configure_csi_clk_lane(0, 0, false, 0);

// 直接访问 CSI 控制器
if let Some(csi) = mipirx.csi_mut(0) {
    csi.set_lane_mode(LaneMode::Lane4);
    csi.set_vs_gen_mode(VsGenMode::ByFsFe);
    csi.set_default_vc_mapping();
}
```

## 数据类型说明

### RawDataType

| 类型 | CSI Data Type | 描述 |
|------|---------------|------|
| Raw8 | 0x2A | 8-bit RAW 数据 |
| Raw10 | 0x2B | 10-bit RAW 数据 |
| Raw12 | 0x2C | 12-bit RAW 数据 |
| Raw16 | 0x2E | 16-bit RAW 数据 |
| Yuv422_8bit | 0x1E | YUV422 8-bit |
| Yuv422_10bit | 0x1F | YUV422 10-bit |

### HDR 模式

| 模式 | 描述 |
|------|------|
| None | 无 HDR，线性模式 |
| Vc | 使用 Virtual Channel 区分长短曝光 |
| Id | 使用 ID 区分长短曝光 |
| Dt | 使用 Data Type 区分长短曝光 |
| Dol | Digital Overlap 模式 |
| Manual | 手动配置模式 |

## 注意事项

1. **初始化顺序**: 必须先调用 `init()` 初始化模块，再进行配置。

2. **PHY 模式**: 双路 sensor 时需要先设置 PHY 为 `Mode1C2D_1C2D` 模式。

3. **Lane 配置**: `lane_id` 数组中使用 -1 表示不使用该 Lane。

4. **时钟配置**: 确保时钟 Lane 选择与实际硬件连接一致。

5. **中断处理**: 建议在使能接收前清除所有中断，并根据需要配置中断掩码。

6. **错误处理**: 定期检查 `has_error()` 以检测传输错误。

## 与 CIF 模块的关系

MIPI RX 模块是 CIF (Camera Interface) 的一部分，负责物理层和链路层的处理。完整的视频采集流程为：

```
Sensor -> MIPI RX (PHY + CSI) -> ISP -> VI -> Frame Buffer
```

MIPI RX 模块负责：
1. 接收 sensor 的原始数据（通过 MIPI CSI-2、Sub-LVDS 或 HiSPi 接口）
2. 解析数据包格式（如 MIPI CSI-2 的 Data Type）
3. 将数据传递给 ISP 进行处理

## 参考文档

- SG2002 Technical Reference Manual - Chapter 19.3 MIPI Rx
