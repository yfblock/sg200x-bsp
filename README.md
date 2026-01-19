# sg200x-bsp

SG2002/SG200x 系列芯片的板级支持包 (BSP)，提供硬件抽象层驱动程序。

## 功能模块

| 模块 | 状态 | 描述 |
|------|------|------|
| pinmux | ✅ 完成 | 引脚复用控制驱动 |
| gpio | ✅ 完成 | GPIO 控制驱动 |
| sdmmc | ✅ 完成 | SD/MMC 控制驱动 |
| tpu | 🚧 进行中 | TPU (张量处理单元) 驱动 |
| spi | 📋 计划中 | SPI 控制驱动 |
| i2c | 📋 计划中 | I2C 控制驱动 |
| mipirx | 📋 计划中 | MIPI RX 控制驱动 |

## 使用方法

在 `Cargo.toml` 中添加依赖：

```toml
[dependencies]
sg200x-bsp = "0.1"
```

## 示例

```rust
#![no_std]

use sg200x_bsp::{gpio, pinmux, sdmmc};

// 使用各模块进行硬件操作
```

## 许可证

本项目采用 MIT 许可证，详见 [LICENSE](LICENSE) 文件。
