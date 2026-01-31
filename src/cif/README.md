# CIF (Camera Interface) 驱动模块

本模块提供 SG2002 芯片的相机接口驱动，从 C 代码迁移到 Rust。

## 功能特性

- **MIPI CSI-2 接口**: 支持 1/2/4 Lane，RAW8/RAW10/RAW12/YUV422 格式
- **LVDS/Sub-LVDS/HiSPI 接口**: 支持多种同步模式和 HDR 模式
- **DVP/BT 并行接口**: 支持 BT656/BT601/BT1120/BT Demux
- **PHY 层配置**: Lane 映射、Deskew、时钟方向控制
- **时钟管理**: MAC 时钟、传感器 MCLK 配置
- **HDR 支持**: VC/DT/DOL/Manual 多种 HDR 模式
- **帧捕获**: 视频帧缓冲区管理和捕获接口

## 视频采集架构

SG2002 的视频采集流程：

```
Sensor -> CIF (MIPI/LVDS/TTL) -> ISP -> VI -> Frame Buffer
```

- **CIF (Camera Interface)**: 相机接口层，负责接收传感器的原始数据
- **ISP (Image Signal Processor)**: 图像信号处理器，进行去马赛克、白平衡等处理
- **VI (Video Input)**: 视频输入模块，管理帧缓冲区

CIF 模块的主要职责：
1. 接收传感器的原始数据（通过 MIPI CSI-2、LVDS 或 TTL 接口）
2. 解析数据包格式（如 MIPI CSI-2 的 Data Type）
3. 将数据传递给 ISP 进行处理

## 模块结构

```
cif/
├── mod.rs          # 主模块，CifDev/CifCtx 核心结构
├── types.rs        # 类型定义和枚举
├── regs.rs         # 寄存器地址和底层读写函数
├── drv.rs          # 底层驱动函数（对应 cif_drv.c）
├── mipi.rs         # MIPI CSI-2 配置
├── lvds.rs         # LVDS/Sub-LVDS/HiSPI 配置
├── ttl.rs          # TTL/DVP/BT 并行接口配置
├── phy.rs          # PHY 层配置
├── vip_sys.rs      # VIP 系统控制（时钟/复位）
├── frame.rs        # 帧捕获模块
└── examples.rs     # 使用示例
```

## 使用流程

### 1. 基本 MIPI 配置（配合 GC4653 传感器）

```rust
use sg200x_bsp::cif::*;

// 步骤 1: 初始化 CIF 设备
let mut cif_dev = unsafe { CifDev::new(0) };
unsafe { cif_dev.init() };

// 步骤 2: 配置 MIPI 属性
let mut attr = ComboDevAttr::default();
attr.input_mode = InputMode::Mipi;
attr.devno = 0;
attr.mac_clk = RxMacClk::Clk400M;
attr.img_size = ImgSize {
    x: 0,
    y: 0,
    width: 2560,
    height: 1440,
};
attr.mclk = MclkPll {
    cam: 0,
    freq: CamPllFreq::Freq27M,
};

// MIPI 特定配置
attr.mipi_attr = Some(MipiDevAttr {
    raw_data_type: RawDataType::Raw10Bit,
    lane_id: [2, 1, 3, -1, -1],  // CLK=2, D0=1, D1=3
    hdr_mode: MipiHdrMode::None,
    data_type: [0x2B, 0],  // RAW10 数据类型
    pn_swap: [1, 1, 1, 0, 0],
    dphy: Dphy {
        enable: false,
        hs_settle: 0,
    },
    demux: MipiDemuxInfo {
        demux_en: false,
        vc_mapping: [0, 1, 2, 3],
    },
});

// 步骤 3: 应用配置
cif_dev.set_dev_attr(&attr).unwrap();

// 步骤 4: 使能传感器时钟
cif_dev.enable_sensor_clock(0, true).unwrap();
```

### 2. 视频帧捕获

```rust
use sg200x_bsp::cif::frame::*;

// 创建帧捕获器
let mut capture = unsafe { FrameCapture::new(0) };

// 配置捕获参数
let config = CaptureConfig {
    width: 2560,
    height: 1440,
    format: PixelFormat::Raw10,
    buffer_count: 3,
};
capture.configure(&config)?;

// 分配帧缓冲区（需要提供物理地址）
let phy_addrs = [0x8000_0000, 0x8100_0000, 0x8200_0000];
let sizes = [0x100_0000; 3]; // 每个 16MB
capture.allocate_buffers(&phy_addrs, &sizes)?;

// 开始捕获
capture.start()?;

// 获取一帧
let frame = capture.get_frame(1000)?; // 1000ms 超时

// 处理帧数据
println!("Frame: addr=0x{:08x}, {}x{}", 
    frame.phy_addr, frame.width, frame.height);

// 释放帧
capture.release_frame(&frame)?;

// 停止捕获
capture.stop()?;
```

### 3. 底层驱动使用

```rust
use sg200x_bsp::cif::drv::*;

// 创建驱动上下文
let ctx = unsafe { CifDrvCtx::new(0) };

// 配置 CSI 参数
let csi_param = ParamCsi {
    lane_num: 2,
    fmt: CsiFmt::Raw10,
    vs_gen_mode: CsiVsGenMode::Fs,
    hdr_mode: CsiHdrMode::Vc,
    data_type: [0x2B, 0],
    decode_type: 0x2B,
    vc_mapping: [0, 1, 2, 3],
};

// 配置 CSI
unsafe { cif_config_csi(&ctx, &csi_param) };

// 配置 Lane ID
unsafe {
    cif_set_lane_id(&ctx, LaneId::Clk, 2, true);
    cif_set_lane_id(&ctx, LaneId::Lane0, 1, true);
    cif_set_lane_id(&ctx, LaneId::Lane1, 3, true);
}

// 开始流
unsafe { cif_streaming(&ctx, true, CifType::Csi, 2) };
```

## 关键数据结构

### ComboDevAttr - 组合设备属性

```rust
pub struct ComboDevAttr {
    pub input_mode: InputMode,      // 输入模式（MIPI/LVDS/TTL等）
    pub mac_clk: RxMacClk,          // MAC 时钟（200M-600M）
    pub mclk: MclkPll,              // 传感器主时钟配置
    pub devno: u32,                 // 设备编号
    pub img_size: ImgSize,          // 图像尺寸
    pub mipi_attr: Option<MipiDevAttr>,  // MIPI 属性
    pub lvds_attr: Option<LvdsDevAttr>,  // LVDS 属性
    pub ttl_attr: Option<TtlDevAttr>,    // TTL 属性
    // ...
}
```

### MipiDevAttr - MIPI 设备属性

```rust
pub struct MipiDevAttr {
    pub raw_data_type: RawDataType, // 数据类型（RAW8/10/12, YUV422）
    pub lane_id: [i16; 5],          // Lane ID 映射 [CLK, D0, D1, D2, D3]
    pub hdr_mode: MipiHdrMode,      // HDR 模式
    pub data_type: [i16; 2],        // MIPI Data Type
    pub pn_swap: [i8; 5],           // P/N 交换
    pub dphy: Dphy,                 // DPHY 配置
    pub demux: MipiDemuxInfo,       // Demux 配置
}
```

### FrameBuffer - 帧缓冲区

```rust
pub struct FrameBuffer {
    pub phy_addr: usize,            // 物理地址
    pub vir_addr: Option<*mut u8>,  // 虚拟地址
    pub size: usize,                // 缓冲区大小
    pub width: u32,                 // 图像宽度
    pub height: u32,                // 图像高度
    pub stride: u32,                // 每行字节数
    pub format: PixelFormat,        // 像素格式
}
```

## 寄存器基地址

| 寄存器块 | 基地址 | 说明 |
|---------|--------|------|
| DPHY_TOP | 0x0A0D_0000 | DPHY 顶层控制 |
| DPHY_4L | 0x0A0D_0300 | 4-Lane DPHY |
| DPHY_2L | 0x0A0D_0600 | 2-Lane DPHY |
| SENSOR_MAC0 | 0x0A0C_2000 | MAC0 控制器 |
| SENSOR_MAC1 | 0x0A0C_4000 | MAC1 控制器 |
| SENSOR_MAC_VI | 0x0A0C_6000 | VI MAC 控制器 |
| VIP_SYS | 0x0A0C_8000 | VIP 系统控制 |

## 中断

| 中断号 | 说明 |
|-------|------|
| 22 | CSI MAC0 中断 |
| 23 | CSI MAC1 中断 |

中断状态位：
- Bit 0: ECC 错误
- Bit 1: CRC 错误
- Bit 2: 头部错误
- Bit 3: 字数错误
- Bit 4: FIFO 满

## 实现状态

### ✅ 已完成
- 核心类型定义和枚举
- 模块化结构（按接口类型分离）
- 高层 API 框架（`CifDev`、`CifCtx`）
- MIPI/LVDS/TTL 配置逻辑迁移
- PHY 层配置接口
- VIP 系统时钟和复位控制
- 底层驱动函数（`cif_drv.rs`）
- 帧捕获模块（`frame.rs`）
- 使用示例

### ⚠️ 待完善
- **完整寄存器定义**: 使用 tock-registers 定义完整的寄存器结构体
- **中断处理**: ISR 和中断状态管理的完整实现
- **DMA 配置**: 图像数据 DMA 传输配置
- **VI/ISP 集成**: 与 VI 和 ISP 模块的集成
- **硬件测试**: 在实际硬件上验证

## 注意事项

1. **物理地址**: 帧缓冲区需要使用物理地址，需要从系统内存分配器获取
2. **时钟配置**: 确保在配置 CIF 之前正确配置时钟
3. **Lane 映射**: Lane ID 必须与硬件连接一致
4. **中断处理**: 建议启用中断以检测错误状态
5. **缓冲区大小**: 确保缓冲区足够大以容纳完整的帧数据

## 参考资料

- SG2002 TRM (Technical Reference Manual)
- LicheeRV-Nano-Build FreeRTOS CIF 驱动源码
- MIPI CSI-2 规范
