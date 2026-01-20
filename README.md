# sg200x-bsp

SG2002/SG200x 系列芯片的板级支持包 (BSP)，提供硬件抽象层驱动程序。

## 功能模块

| 模块 | 状态 | 描述 |
|------|------|------|
| pinmux | ✅ 完成 | 引脚复用控制驱动 |
| gpio | ✅ 完成 | GPIO 控制驱动 |
| sdmmc | ✅ 完成 | SD/MMC 控制驱动 |
| i2c | ✅ 完成 | I2C 控制驱动 |
| tpu | 🚧 进行中 | TPU (张量处理单元) 驱动 |
| spi | 📋 计划中 | SPI 控制驱动 |
| mipirx | 📋 计划中 | MIPI RX 控制驱动 |

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sg200x-bsp = "0.1"
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

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
