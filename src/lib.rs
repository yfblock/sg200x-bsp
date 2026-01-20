//! SG2002 BSP (Board Support Package)
//!
//! 本 crate 提供 SG2002 芯片的底层硬件抽象层驱动程序。
//!
//! # 模块
//!
//! - `pinmux`: 引脚复用控制驱动
//! - `gpio`: GPIO 控制驱动
//! - `sdmmc`: SD/MMC 控制驱动
//! - `i2c`: I2C 控制驱动
//! - `tpu`: TPU (张量处理单元) 驱动
//!
//! # I2C 使用示例
//!
//! ```rust,ignore
//! use sg200x_bsp::i2c::{I2c, I2cInstance, I2cSpeed};
//!
//! // 创建 I2C0 驱动实例
//! let mut i2c = unsafe { I2c::new(I2cInstance::I2c0) };
//!
//! // 初始化 I2C，使用快速模式
//! i2c.init(I2cSpeed::Fast);
//!
//! // 写入数据到设备
//! let slave_addr = 0x50;
//! let data = [0x00, 0x01, 0x02];
//! i2c.write(slave_addr, &data).unwrap();
//!
//! // 从设备读取数据
//! let mut buf = [0u8; 4];
//! i2c.read(slave_addr, &mut buf).unwrap();
//! ```

#![no_std]
#![recursion_limit = "512"]

pub mod gpio;
pub mod pinmux;
pub mod sdmmc;
pub mod tpu;
pub mod i2c;
