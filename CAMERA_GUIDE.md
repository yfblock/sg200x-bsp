# SG2002 相机驱动使用指南

本指南介绍如何使用 `sg200x-bsp` 中的相机相关驱动模块。

## 模块概览

### 1. `camera` 模块 - 传感器驱动
- **位置**: `src/camera/`
- **功能**: 传感器芯片的 I2C 控制（初始化、曝光、增益等）
- **当前支持**: GC4653 (2560x1440@30fps 线性模式)

### 2. `cif` 模块 - 相机接口驱动
- **位置**: `src/cif/`
- **功能**: MIPI/LVDS/DVP 等物理接口配置
- **支持接口**: MIPI CSI-2, Sub-LVDS, HiSPI, BT656/601/1120

## 完整使用流程

### 步骤 1: 配置 CIF (相机接口)

```rust
use sg200x_bsp::cif::*;

// 创建 CIF 设备
let mut cif_dev = unsafe { CifDev::new(0) };
unsafe { cif_dev.init() };

// 配置 MIPI 属性（与传感器匹配）
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

// MIPI 特定配置（与 GC4653 对应）
attr.mipi_attr = Some(MipiDevAttr {
    raw_data_type: RawDataType::Raw10Bit,
    lane_id: [2, 1, 3, -1, -1],  // CLK=Lane2, D0=Lane1, D1=Lane3
    hdr_mode: MipiHdrMode::None,
    data_type: [0x2B, 0],  // RAW10
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

// 应用配置
cif_dev.set_dev_attr(&attr).unwrap();

// 使能传感器时钟
cif_dev.enable_sensor_clock(0, true).unwrap();
```

### 步骤 2: 初始化传感器 (GC4653)

```rust
use sg200x_bsp::camera::gc4653::*;
use sg200x_bsp::i2c::I2cInstance;

// 创建 GC4653 驱动（使用 I2C3）
let gc4653 = unsafe { Gc4653::new(I2cInstance::I2c3) };

// 探测传感器（读取 Chip ID）
gc4653.probe().unwrap();

// 初始化为 2560x1440@30fps 线性模式
gc4653.init_linear_1440p30().unwrap();

// 可选：调整曝光和增益
gc4653.set_exposure_lines(1500).unwrap();
gc4653.set_gain(2048, 1024).unwrap();  // 2x 模拟增益, 1x 数字增益
```

### 步骤 3: 采集图像数据

**注意**: 当前实现只完成了接口配置层，图像数据采集需要额外的 VI/ISP 驱动支持。

典型的数据流：
```
传感器 → MIPI CSI-2 → CIF (MAC) → VI (Video Input) → ISP → DMA → 内存
  ↑                      ↑                                    ↑
camera 模块           cif 模块                          待实现
```

## 配置参数说明

### MIPI Lane 映射

GC4653 默认配置：
- `lane_id[0]` = 2 (CLK Lane)
- `lane_id[1]` = 1 (Data Lane 0)
- `lane_id[2]` = 3 (Data Lane 1)
- `lane_id[3]` = -1 (未使用)
- `lane_id[4]` = -1 (未使用)

### 数据类型

| 格式 | 数据类型值 | 说明 |
|------|-----------|------|
| RAW8 | 0x2A | 8-bit Bayer RAW |
| RAW10 | 0x2B | 10-bit Bayer RAW (GC4653) |
| RAW12 | 0x2C | 12-bit Bayer RAW |
| YUV422-8 | 0x1E | 8-bit YUV422 |
| YUV422-10 | 0x1F | 10-bit YUV422 |

### MAC 时钟选择

| 时钟 | 频率 | 适用场景 |
|------|------|---------|
| Clk200M | 200MHz | 低带宽传感器 |
| Clk300M | 300MHz | 中等带宽 |
| Clk400M | 400MHz | 标准配置（推荐） |
| Clk500M | 500MHz | 高带宽传感器 |
| Clk600M | 600MHz | 最高带宽 |

## 常见问题

### Q: 如何读取摄像头拍摄的照片？

A: 当前 `camera` 和 `cif` 模块只负责传感器和接口配置，图像数据采集需要：
1. **VI (Video Input) 驱动**: 配置 DMA 通道和缓冲区
2. **ISP 驱动**: 配置图像处理管道
3. **内存管理**: 分配帧缓冲区

建议后续添加 `vi` 和 `isp` 模块来完成完整的采集链路。

### Q: 支持哪些传感器？

A: 当前只实现了 GC4653，但框架支持扩展：
- 在 `src/camera/` 添加新的传感器驱动（如 `imx327.rs`）
- 参考 `gc4653.rs` 的结构实现 I2C 寄存器配置

### Q: 如何调试 MIPI 接口？

A: 可以通过以下方式：
1. 检查 PHY 状态：`phy::get_csi_phy_state()`
2. 检查中断状态：`mipi::check_csi_int_sts()`
3. 查看错误计数：`link.sts_csi.errcnt_*`

## 参考资料

- `sensor/` 目录: 原始 C 代码实现
- `cif/` 目录: 原始 CIF C 代码实现
- `sg2002_trm_cn_v1.02.pdf`: SG2002 技术参考手册
