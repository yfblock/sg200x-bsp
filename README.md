# sg200x-bsp

SG2002/SG200x 系列芯片的板级支持包 (BSP)，提供硬件抽象层驱动程序。

## 功能模块

| 模块 | 状态 | 描述 |
|------|------|------|
| pinmux | ✅ 完成 | 引脚复用控制驱动 |
| gpio | ✅ 完成 | GPIO 控制驱动 |
| sdmmc | ✅ 完成 | SD/MMC 控制驱动 |
| i2c | ✅ 完成 | I2C 控制驱动 |
| pwm | ✅ 完成 | PWM 控制驱动 |
| rstc | ✅ 完成 | 复位控制器驱动 |
| mp | ✅ 完成 | 多处理器启动驱动 |
| tpu | 🚧 进行中 | TPU (张量处理单元) 驱动 |
| spi | 📋 计划中 | SPI 控制驱动 |
| mipirx | 📋 计划中 | MIPI RX 控制驱动 |

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sg200x-bsp = "0.3"
```

## 示例

### GPIO 和 Pinmux

```rust
#![no_std]

use sg200x_bsp::{gpio, pinmux, sdmmc};

// 使用各模块进行硬件操作
```

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
