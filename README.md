# sg200x-bsp

SG2002/SG200x 系列芯片的板级支持包 (BSP)，提供硬件抽象层驱动程序。

## 功能模块

| 模块 | 状态 | 描述 |
|------|------|------|
| soc | ✅ 完成 | SoC 外设 MMIO 物理基址（`soc::sg2002::*`），各驱动再导出保持兼容 |
| pinmux | ✅ 完成 | 引脚复用：FMUX 功能选择 + IOBLK 电气配置（含 MIPI 等焊盘的 FMUX 位域） |
| gpio | ✅ 完成 | GPIO 控制驱动 |
| sdmmc | ✅ 完成 | SD/MMC 控制驱动 |
| i2c | ✅ 完成 | I2C 控制驱动 |
| pwm | ✅ 完成 | PWM 控制驱动 |
| rstc | ✅ 完成 | 复位控制器驱动 |
| mp | ✅ 完成 | 多处理器启动驱动 |
| dma | ✅ 完成 | Synopsys DesignWare AXI DMA |
| usb | ✅ 完成 | USB 主机（DWC2）：枚举、拓扑扫描、UVC / Mass Storage 类协议 |
| utils | ✅ 完成 | D-cache / DMA 一致性等通用工具 |
| ethernet | 🔌 可选 | 启用 feature `ethernet` 时编译板载 cvitek-eth（DWMAC + 内部 EPHY），需 `alloc` |

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sg200x-bsp = "0.6"
```

可选功能（在依赖中打开对应 feature，例如 `sg200x-bsp = { version = "0.6", features = ["ethernet"] }`）：

| Feature | 说明 |
|---------|------|
| `cv182x-host` | 默认启用；CV1812H/SG2002 USB PHY/控制器相关 bring-up 路径 |
| `c906` | 默认启用；T-Head C906 非标 D-cache 指令；非 C906 核请关闭 |
| `ethernet` | 板载以太网驱动（需 `alloc`） |
| `usb-force-no-dma` | 调试：USB 主机强制 PIO，仅用于排查 DMA/cache |

## 示例

### Pinmux 与 FMUX

引脚数字功能由 **FMUX**（基地址见 `soc::FMUX_BASE`）的 **FSEL** 位选择；上拉/下拉等由 **IOBLK** 配置（Active Domain 见 `soc::IOBLK_BASE` + `IOBLK_G*_OFFSET`，RTC 域见 `IOBLK_GRTC_BASE`）。**I2C3** 的 SCL/SDA 分别复用在 **SD1_CMD** / **SD1_CLK**（FSEL = 2），可用 `Pinmux::setup_iic3_pins()` 一次配置。

```rust
#![no_std]

use sg200x_bsp::pinmux::{Pinmux, PullConfig, FMUX_UART0_TX};
use sg200x_bsp::soc::{FMUX_BASE, IOBLK_BASE, IOBLK_GRTC_BASE};

let pinmux = unsafe { Pinmux::new(FMUX_BASE, IOBLK_BASE, IOBLK_GRTC_BASE) };

// 将 UART0_TX 保持为默认 UART 功能，并配置上拉
pinmux.set_uart0_tx_func(FMUX_UART0_TX::FSEL::UART0_TX);
pinmux.set_uart0_tx_pull(PullConfig::PullUp);

// I2C3：SD1_CMD → SCL，SD1_CLK → SDA
pinmux.setup_iic3_pins();
```

更底层的寄存器类型（如 `FMUX_SD0_CLK`、`FmuxRegisters`）由 `pinmux` 模块从 `fmux` 子模块再导出，可直接配合 `tock_registers` API 读写。

### I2C

```rust
#![no_std]

use sg200x_bsp::i2c::{I2c, I2cInstance, I2cSpeed};

// 创建 I2C0 驱动实例
let mut i2c = unsafe { I2c::new(I2cInstance::I2c0) };

// 初始化 I2C，使用快速模式 (~400 kbit/s)
i2c.init(I2cSpeed::Fast);

// 写入数据到设备
let slave_addr = 0x50;
let data = [0x00, 0x01, 0x02];
i2c.write(slave_addr, &data).unwrap();

// 从设备读取数据
let mut buf = [0u8; 4];
i2c.read(slave_addr, &mut buf).unwrap();

// 写后读操作 (常用于寄存器读取)
let reg_addr = [0x00];
i2c.write_read(slave_addr, &reg_addr, &mut buf).unwrap();
```

### PWM

```rust
#![no_std]

use sg200x_bsp::pwm::{Pwm, PwmChannel, PwmMode, PwmPolarity};
use sg200x_bsp::soc::PWM0_BASE;

// 创建 PWM0 控制器驱动实例
let mut pwm = unsafe { Pwm::new(PWM0_BASE) };

// 配置通道 0: 1KHz, 50% 占空比
pwm.configure_channel(
    PwmChannel::Channel0,
    1_000,      // 1KHz 频率
    50,         // 50% 占空比
    PwmPolarity::ActiveHigh,
).unwrap();

// 使能 IO 输出并启动
pwm.enable_output(PwmChannel::Channel0);
pwm.start(PwmChannel::Channel0);
```

### 多处理器启动

```rust
#![no_std]

use sg200x_bsp::mp::SecSys;
use sg200x_bsp::soc::SEC_SYS_BASE;

// 启动协处理器 (小核 C906@700MHz)
let sec_sys = unsafe { SecSys::new(SEC_SYS_BASE) };
unsafe {
    sec_sys.start_secondary_core(entry_address);
}
```

## I2C 模块详情

SG2002 芯片共有 6 个 I2C 控制器：

| 实例 | 基地址常量 (`soc::`) | 域 |
|------|----------------------|-----|
| I2C0 | `I2C0_BASE` | Active Domain |
| I2C1 | `I2C1_BASE` | Active Domain |
| I2C2 | `I2C2_BASE` | Active Domain |
| I2C3 | `I2C3_BASE` | Active Domain |
| I2C4 | `I2C4_BASE` | Active Domain |
| RTCSYS_I2C | `RTCSYS_I2C_BASE` | No-die Domain (RTC) |

### 功能特性

- 支持 Master 模式
- 支持 7 位和 10 位地址模式
- 支持标准模式 (~100 kbit/s) 和快速模式 (~400 kbit/s)
- 支持 General Call 和 Start Byte
- 64 x 8bit TX FIFO 和 64 x 8bit RX FIFO
- 支持 DMA 传输

## PWM 模块详情

SG2002 芯片共有 4 个 PWM 控制器，共 16 路 PWM 输出：

| 实例 | 基地址常量 (`soc::`) | 通道 |
|------|----------------------|------|
| PWM0 | `PWM0_BASE` | PWM[0-3] |
| PWM1 | `PWM1_BASE` | PWM[4-7] |
| PWM2 | `PWM2_BASE` | PWM[8-11] |
| PWM3 | `PWM3_BASE` | PWM[12-15] |

### 功能特性

- 支持连续输出模式和固定脉冲数输出模式
- 支持 4 路 PWM 同步输出模式 (可配置相位差)
- 支持极性配置
- 支持动态更新 PWM 参数
- 30 位计数器，支持宽范围频率输出
- 时钟源: 100MHz (默认) 或 148.5MHz
- 最高输出频率: 50MHz，最低输出频率: ~0.093Hz

## 复位控制器模块详情

复位控制器基地址: `soc::RSTC_BASE`（与各驱动 `RSTC_BASE` 再导出同值）

### 功能特性

- 支持软复位控制 (SOFT_RSTN_0 ~ SOFT_RSTN_3)
- 支持 CPU 自动清除软复位 (SOFT_CPUAC_RSTN)
- 支持 CPU 软复位 (SOFT_CPU_RSTN)
- 复位配置为低电平有效

## USB 模块详情

当前 BSP 仅实现 **USB 主机**模式（DWC2 Force Host + 根口枚举）。USB Device / CDC-ACM 暂未包含在本仓库中。

### 代码结构

```
usb/
├── error.rs, setup.rs          # 错误类型与标准 SETUP 包构造
├── host/
│   ├── dwc2/                   # Synopsys DWC2 主机访问层
│   │   ├── regs.rs             # 寄存器/位域（tock-registers）
│   │   ├── controller.rs       # 上电、软复位、FIFO、HPRT0
│   │   ├── isr.rs              # PLIC 中断、GINTMSK/HAINTMSK
│   │   ├── dma.rs              # 共用 DMA 窗口与偏移常量
│   │   ├── channel.rs          # 通道调度原语（ch_xfer、HCCHAR）
│   │   ├── control.rs          # EP0 控制传输、SET_ADDRESS 等
│   │   ├── bulk.rs             # Bulk IN/OUT
│   │   └── isoch.rs            # Isochronous IN（UVC 微帧抓包）
│   ├── enumerate.rs            # 根口连接检查 + 枚举入口
│   └── topology.rs             # Hub 递归扫描，标记 MSC/UVC 候选
└── class/
    ├── uvc.rs                  # UVC 描述符解析、PROBE/COMMIT、抓帧
    └── mass_storage.rs         # MSC BBB + SCSI 命令封装
```

通道约定：**通道 0** 专用于 EP0 控制传输；**通道 5** 专用于 Bulk / Isoch（与 Linux DWC2 HCD 分配习惯一致）。

### 主机 bring-up 顺序

1. 板级设置 MMIO 基址：[`set_dwc2_base_virt`](https://docs.rs/sg200x-bsp/latest/sg200x_bsp/usb/fn.set_dwc2_base_virt.html)；启用 `cv182x-host` 时再 [`set_cv182x_phy_base_virt`](https://docs.rs/sg200x-bsp/latest/sg200x_bsp/usb/fn.set_cv182x_phy_base_virt.html)。
2. 若 VA≠PA，注册 [`set_usb_dma_to_phys_fn`](https://docs.rs/sg200x-bsp/latest/sg200x_bsp/usb/fn.set_usb_dma_to_phys_fn.html) 供 `HCDMA` 地址转换。
3. 实现 `log` crate 的 `Logger`（串口输出）。
4. 调用 `dwc2::dwc2_host_init()`，确认 `HPRT0.CONNSTS` 后执行总线复位。
5. 调用 `host::enumerate_root_port()` 或 `host::topology::enumerate_bus_print_tree()` 完成枚举。

可选：注册 PLIC 中断以启用 Isoch 中断路径：

```rust
axhal::irq::register(
    sg200x_bsp::usb::host::dwc2::DWC2_IRQ_NUM,      // SG2002: 30
    sg200x_bsp::usb::host::dwc2::dwc2_interrupt_handler,
);
```

### UVC 摄像头（Bulk / Isoch）

类驱动在 `usb::class::uvc`：

- `read_configuration_descriptor` + `parse_uvc_video_stream`：解析配置描述符，**优先选择 Bulk IN**，否则回退 **Isoch IN**（记录各 alt 的 `mps_raw`、高带宽 `mult`）。
- `uvc_start_video_stream`：PROBE/COMMIT 协商后启动视频流。
- `uvc_capture_one_frame`：按 `UvcStreamSelection.xfer` 走 `bulk_in` 或 `isoch_in_uframe_batch` 抓一帧 MJPEG。

完整板级示例见 ArceOS 工程 `examples/helloworld`（`usb_camera.rs`）。

## 多处理器模块详情

SG2002 芯片包含以下处理器核心：

| 核心 | 架构 | 频率 | 启动支持 |
|------|------|------|----------|
| 大核 | RISC-V C906 / ARM Cortex-A53 | 1GHz | - |
| 小核 (协处理器) | RISC-V C906 | 700MHz | ✅ |
| 8051 | 8051 | 25MHz | ❌ |

### 功能特性

- 支持启动协处理器 (小核 C906@700MHz)
- 支持设置协处理器启动地址
- 提供便捷函数和底层原始寄存器操作两种方式
- 暂不支持 8051 核心启动

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
