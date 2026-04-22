# sg200x-bsp

SG2002/SG200x 系列芯片的板级支持包 (BSP)，提供硬件抽象层驱动程序。

## 功能模块

| 模块 | 状态 | 描述 |
|------|------|------|
| pinmux | ✅ 完成 | 引脚复用：FMUX 功能选择 + IOBLK 电气配置（含 MIPI 等焊盘的 FMUX 位域） |
| gpio | ✅ 完成 | GPIO 控制驱动 |
| sdmmc | ✅ 完成 | SD/MMC 控制驱动 |
| i2c | ✅ 完成 | I2C 控制驱动 |
| pwm | ✅ 完成 | PWM 控制驱动 |
| rstc | ✅ 完成 | 复位控制器驱动 |
| mp | ✅ 完成 | 多处理器启动驱动 |
| dma | ✅ 完成 | Synopsys DesignWare AXI DMA |
| usb | ✅ 完成 | USB 主机（DWC2）及类协议（UVC / Mass Storage 等） |
| utils | ✅ 完成 | D-cache / DMA 一致性等通用工具 |
| ethernet | 🔌 可选 | 启用 feature `ethernet` 时编译板载 cvitek-eth（DWMAC + 内部 EPHY），需 `alloc` |

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sg200x-bsp = "0.5"
```

可选功能（在依赖中打开对应 feature，例如 `sg200x-bsp = { version = "0.5", features = ["ethernet"] }`）：

| Feature | 说明 |
|---------|------|
| `cv182x-host` | 默认启用；CV1812H/SG2002 USB PHY/控制器相关 bring-up 路径 |
| `c906` | 默认启用；T-Head C906 非标 D-cache 指令；非 C906 核请关闭 |
| `ethernet` | 板载以太网驱动（需 `alloc`） |
| `device-mode` / `device-cdc-acm` | USB Device（含最小 CDC-ACM 示例） |
| `usb-force-no-dma` | 调试：USB 主机强制 PIO，仅用于排查 DMA/cache |

## 示例

### Pinmux 与 FMUX

引脚数字功能由 **FMUX**（基地址 `0x0300_1000`，见 `pinmux::FMUX_BASE`）的 **FSEL** 位选择；上拉/下拉等由 **IOBLK** 各组寄存器配置。**I2C3** 的 SCL/SDA 分别复用在 **SD1_CMD** / **SD1_CLK**（FSEL = 2），可用 `Pinmux::setup_iic3_pins()` 一次配置。

```rust
#![no_std]

use sg200x_bsp::pinmux::{Pinmux, PullConfig, FMUX_UART0_TX};

let pinmux = Pinmux::new();

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

use sg200x_bsp::pwm::{Pwm, PwmInstance, PwmChannel, PwmMode, PwmPolarity};

// 创建 PWM0 控制器驱动实例
let mut pwm = unsafe { Pwm::new(PwmInstance::Pwm0) };

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

use sg200x_bsp::mp::start_secondary_core;

// 启动协处理器 (小核 C906@700MHz)
unsafe {
    start_secondary_core(entry_address);
}
```

## I2C 模块详情

SG2002 芯片共有 6 个 I2C 控制器：

| 实例 | 基地址 | 域 |
|------|--------|-----|
| I2C0 | 0x0400_0000 | Active Domain |
| I2C1 | 0x0401_0000 | Active Domain |
| I2C2 | 0x0402_0000 | Active Domain |
| I2C3 | 0x0403_0000 | Active Domain |
| I2C4 | 0x0404_0000 | Active Domain |
| RTCSYS_I2C | 0x0502_B000 | No-die Domain (RTC) |

### 功能特性

- 支持 Master 模式
- 支持 7 位和 10 位地址模式
- 支持标准模式 (~100 kbit/s) 和快速模式 (~400 kbit/s)
- 支持 General Call 和 Start Byte
- 64 x 8bit TX FIFO 和 64 x 8bit RX FIFO
- 支持 DMA 传输

## PWM 模块详情

SG2002 芯片共有 4 个 PWM 控制器，共 16 路 PWM 输出：

| 实例 | 基地址 | 通道 |
|------|--------|------|
| PWM0 | 0x03060000 | PWM[0-3] |
| PWM1 | 0x03061000 | PWM[4-7] |
| PWM2 | 0x03062000 | PWM[8-11] |
| PWM3 | 0x03063000 | PWM[12-15] |

### 功能特性

- 支持连续输出模式和固定脉冲数输出模式
- 支持 4 路 PWM 同步输出模式 (可配置相位差)
- 支持极性配置
- 支持动态更新 PWM 参数
- 30 位计数器，支持宽范围频率输出
- 时钟源: 100MHz (默认) 或 148.5MHz
- 最高输出频率: 50MHz，最低输出频率: ~0.093Hz

## 复位控制器模块详情

复位控制器基地址: 0x03003000

### 功能特性

- 支持软复位控制 (SOFT_RSTN_0 ~ SOFT_RSTN_3)
- 支持 CPU 自动清除软复位 (SOFT_CPUAC_RSTN)
- 支持 CPU 软复位 (SOFT_CPU_RSTN)
- 复位配置为低电平有效

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
