# sg200x-bsp 架构规范

本文档从当前 `sg200x-bsp` 实现中提炼**架构设计与约束**，供新增/修改驱动与上层集成时遵循。寄存器位域细节另见 [`AGENTS.md`](AGENTS.md)。

---

## 1. 定位与边界

### 1.1 是什么

`sg200x-bsp` 是 **SG2002 / SG200x（CV181x）** 芯片的 `no_std` 硬件抽象层 crate：

- 提供 MMIO 外设驱动（GPIO、I2C、PWM、SD/MMC、DMA、USB、JPU 等）
- 提供 SoC 物理基址常量（`soc`）
- 提供与具体外设无关的底层 helper（`utils`：D-cache 一致性、忙等延时）

### 1.2 不是什么（禁止写入 BSP）

| 禁止项 | 说明 | 应放在 |
|--------|------|--------|
| OS 框架依赖 | `axhal`、`axstd`、`axdriver_*`、`starry-kernel` 等 | 上层 OS / 板级 crate |
| 平台计时 / 基准测试 | `rdtime` 包装、decode 耗时统计、benchmark 报告 | 应用层（如 `examples/helloworld/src/timing.rs`） |
| MMIO 映射（`iomap`） | 虚拟地址安装、页表 | 内核 / 板级启动代码 |
| 网络栈 trait 实现 | `NetDriverOps` 等 | `sg2002-arceos` 等适配层 |
| 全局 `set_mmio_virt_bases` 式单例 | 板级地址由**调用方传入** | 各驱动 `new` / `new_at` 参数 |

BSP 只暴露**中性硬件 API**；OS 适配在 BSP 之上包 wrapper。

---

## 2. 分层结构

```
┌─────────────────────────────────────────────────────────┐
│  应用 / OS（ArceOS、StarryOS、helloworld 等）              │
│  - iomap、virt_to_phys、init.sh、设备节点、benchmark       │
└───────────────────────────┬─────────────────────────────┘
                            │ 传入 MMIO 基址、DMA 地址转换
┌───────────────────────────▼─────────────────────────────┐
│  sg200x-bsp 驱动层                                       │
│  gpio / i2c / pwm / sdmmc / dma / usb / jpu / ethernet … │
│  - 业务逻辑、轮询/中断状态机、协议解析                     │
└───────────────────────────┬─────────────────────────────┘
                            │ tock-registers
┌───────────────────────────▼─────────────────────────────┐
│  regs 子模块 + soc 物理基址                               │
│  register_bitfields! / register_structs!                 │
└───────────────────────────┬─────────────────────────────┘
                            │ MMIO 读写
┌───────────────────────────▼─────────────────────────────┐
│  硬件（CV181x / SG2002）                                  │
└─────────────────────────────────────────────────────────┘

横向：`utils`（cache / delay）被 DMA、USB、JPU 等按需调用，不反向依赖驱动。
```

---

## 3. Crate 组织约束

### 3.1 顶层模块（`lib.rs`）

- 一律 `#![no_std]`
- 按外设划分 `pub mod`；可选功能用 **Cargo feature** 门控（如 `ethernet`、`device-mode`）
- 仅 `ethernet` 等明确需要堆的模块 `extern crate alloc`；其余模块默认无 `alloc`

### 3.2 单外设目录约定

| 文件/目录 | 职责 |
|-----------|------|
| `mod.rs` | 模块文档、公开类型 re-export、驱动主结构体 |
| `regs.rs` | **唯一** MMIO 布局与位域（`tock-registers`） |
| `consts.rs` / `instances.rs` | 实例枚举、FIFO 深度、超时等常量（无 MMIO 读写） |
| 子目录（如 `usb/host/dwc2/`） | 大型外设按控制器/协议拆分；每层仍可有独立 `regs.rs` |

**约束：** 新外设必须先有 `regs.rs`，再在驱动中引用；不得在业务 `.rs` 中散落 magic offset/mask。

### 3.3 `soc` 模块

- **只放物理基址与地址相关常量**（`pub const XXX_BASE: usize`）
- 按地址段分组、带注释；驱动通过 `pub use crate::soc::…` 保持路径兼容
- **禁止**在 `soc` 中写寄存器读写或驱动逻辑

---

## 4. MMIO 访问规范

### 4.1 强制使用 tock-registers

依赖：`tock-registers`（crate 内统一版本，与上层共用 trait 时注意版本一致）。

三层结构：

1. **`regs.rs`** — `register_bitfields!` + `register_structs!`；`ReadOnly` / `ReadWrite` / `WriteOnly` 与手册一致；W1C 语义在注释标明
2. **驱动** — 通过 `RegBlock` 字段访问：`reg.read(FIELD)`、`reg.modify(FIELD::VAL.val(x))`、`reg.write(VALUE32::VAL.val(x))`
3. **拼寄存器值** — 栈上 `LocalRegisterCopy`，再一次性 `.set()`

### 4.2 禁止写法

```rust
// ❌ 裸 volatile（除非遗留代码且注明迁移中）
unsafe { core::ptr::read_volatile(addr.add(0x10)) };

// ❌ 业务代码中的无名 magic 数
regs.xxx.set(0x0000_0005);

// ❌ 在驱动里重复定义 regs.rs 已有的位偏移
let en = (val >> 5) & 1;
```

### 4.3 合法例外（须注释）

- USB 描述符、JPEG 比特流、DMA 缓冲区：**字节流**解析
- RISC-V cache 维护、`fence`：`asm!`（见 `utils::cache`）
- 遗留 `dma/regs.rs` 等 `pub const` 位常量：新字段优先 bitfield，旧代码逐步迁移

---

## 5. 实例化与地址模型

### 5.1 物理基址来源

- 默认值：`crate::soc::*_BASE`（物理地址）
- 适用于 **PA == VA** 的裸机/ArceOS 平台（如 `phys-virt-offset = 0`）

### 5.2 板级传入虚拟基址（推荐模式）

与 GPIO 一致，复杂外设（JPU 等）提供：

```rust
// 默认物理基址（helloworld / 简单平台）
pub fn new() -> Result<Self, E> { ... }

// 板级 iomap 后的虚拟基址（StarryOS 等动态映射平台）
/// # Safety — 调用方保证基址有效映射
pub unsafe fn new_at(...bases: usize, dma_to_phys: Fn) -> Result<Self, E> { ... }
```

**约束：**

- BSP **不**调用 `iomap` / 页表 API
- 需要多个 MMIO 块时，用结构体打包（如 `JpuMmio { jpu_base, top_base, vc_base }`），或分别传参，文档写清每个基址含义

### 5.3 DMA 缓冲区与地址转换

设备 DMA 寄存器写入 **物理地址** 时：

- 提供类型别名或回调，例如 `JpuDmaToPhysFn = fn(usize) -> usize`
- PA==VA 平台可传 `|v| v`
- 上层在 VA≠PA 时传入 `virt_to_phys` 包装（如 StarryOS `cvi_usb_camera.rs`）

**约束：** DMA 前后必须调用 `utils::cache` 做 clean / invalidate（`feature "c906"` 时走 T-Head 指令）。

### 5.4 USB 的全局基址注册（历史模式）

USB 主机栈使用 `set_dwc2_base_virt` / `set_usb_dma_to_phys_fn` 等 **一次性** 原子变量注册板级地址。

- 新代码优先 **显式传参**；若沿用 USB 模式，须在单核初始化阶段完成注册，且文档说明与 `new_at` 等价关系

---

## 6. 错误与类型

### 6.1 错误类型

- 每子系统自有错误枚举（如 `UsbError`、`I2cError`、`EthError`）或 `&'static str`（JPU 等轻量路径）
- **禁止** `std::io::Error`、任意 `String` 错误（无 `alloc` 时不可用）
- 错误语义稳定、可 `Copy`/`Eq` 为佳，便于 `no_std` 上层匹配

### 6.2 结果类型

```rust
pub type UsbResult<T> = Result<T, UsbError>;
```

新模块沿用相同命名：`XxxResult<T>`。

---

## 7. Feature 约束

| Feature | 约束 |
|---------|------|
| `default` | `cv182x-host` + `c906` |
| `c906` | 启用 T-Head D-cache 指令；**非 C906 核必须关闭** |
| `cv182x-host` | USB2 PHY / 控制器 CV182x bring-up 路径 |
| `ethernet` | 编译 `ethernet` 模块，**需要 `alloc`**；不 impl OS 网络 trait |
| `device-mode` / `device-cdc-acm` | USB 从机；与 host 可同时编译，由应用择一 init |
| `usb-force-no-dma` | 仅调试；正常使用**关闭** |

**约束：** 新增 feature 须在 `Cargo.toml` 与 `README.md` 说明；默认 feature 保持可在 SG2002 开发板上开箱可用。

---

## 8. `utils` 边界

| 子模块 | 允许 | 禁止 |
|--------|------|------|
| `cache` | D-cache clean/invalidate、DMA 一致性 | OS 时间源、性能统计 |
| `delay` | NOP 忙等延时 | 依赖 tick 中断的睡眠 |

已移除的 `cpu_time` 类模块：**不得**在 BSP 内恢复平台计时；由上层 `axhal::time` 等提供。

---

## 9. 依赖约束

```toml
# 允许
tock-registers = "0.10"   # MMIO；版本变更需全仓对齐
bit-struct = "0.3"        # 协议结构体（非 MMIO）
log = { default-features = false }  # 可选日志，板级实现 Logger

# 禁止（作为依赖）
axhal, axstd, axdriver, starry-*, tokio, std
```

---

## 10. 上层集成检查清单

集成方（ArceOS / StarryOS / 应用）在调用 BSP 前须完成：

1. **MMIO 映射** — `iomap(phys, size)` 或等效；将虚拟基址传给 `new` / `new_at` / USB `set_*_virt`
2. **DMA 地址** — VA≠PA 时注册 `dma_to_phys`
3. **Cache** — 启用 `c906` feature（SG2002 大核）；DMA 缓冲使用 `utils::cache`
4. **日志** — 若用 `log!`，实现 `log::Log` trait
5. **Pinmux / 时钟** — 使用外设前由板级配置引脚与时钟（BSP 提供 API，不自动猜板型）

BSP **不**假设根文件系统、USB 摄像头已插入、或某块 SD 分区可挂载。

---

## 11. 新外设开发 Checklist

1. [ ] 在 `soc/sg2002.rs` 补充物理基址常量（若手册有新块）
2. [ ] 新增 `src/<module>/regs.rs`：`register_bitfields!` + `register_structs!`
3. [ ] 驱动 `unsafe fn new(base: usize)` 或 `new_at(...)` + 文档 `# Safety`
4. [ ] 业务代码只引用 bitfield 名，无 magic number
5. [ ] DMA 路径：`dcache_clean_range` / `dcache_invalidate_range`
6. [ ] 错误类型 `no_std` 友好；无 `std` / OS crate 依赖
7. [ ] 可选 `alloc` 仅在该模块 `Cargo.toml` feature 下启用
8. [ ] `lib.rs` 模块文档 + `rust,ignore` 最小示例
9. [ ] `cargo check --target riscv64gc-unknown-none-elf`（及所需 features）通过

---

## 12. 参考实现索引

| 模式 | 参考文件 |
|------|----------|
| 单基址 `new` + regs 同文件 | `src/gpio.rs` |
| 多基址 `new_at` + DMA 回调 | `src/jpu/decoder.rs`, `src/jpu/regs.rs` |
| 子模块 regs + 大型状态机 | `src/usb/host/dwc2/` |
| 板级全局 virt 注册 | `src/usb/mod.rs` |
| OS 适配留上层 | `src/ethernet/mod.rs` 模块文档 |
| SoC 仅地址 | `src/soc/sg2002.rs` |
| D-cache / 延时 | `src/utils/cache.rs`, `src/utils/delay.rs` |

---

## 13. 版本与兼容

- Crate 版本遵循 semver；破坏性 API 变更递增主版本
- workspace 依赖声明 `version = "0.7"` 须与 `sg200x-bsp/Cargo.toml` 一致
- 上层若 pin `tock-registers`，须与 BSP **同主版本**，否则 `Writeable` 等 trait 跨 crate 不兼容

---

*本文档描述当前仓库实践；与 [`AGENTS.md`](AGENTS.md) 冲突时，以二者中**更严格**的 MMIO/边界约束为准。*
