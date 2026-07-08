# AGENTS.md — sg200x-bsp

SG2002/SG200x 板级支持包：`no_std` Rust 驱动，MMIO 访问统一用 **tock-registers**（`Cargo.toml` 已依赖 `tock-registers = "0.9"`）。

## 寄存器访问规范（必读）

**新增或修改 MMIO 代码时，优先且尽量只用 tock-registers，不要手写 magic number 读写。**

### 三层结构

1. **`*/regs.rs`** — 寄存器定义（每个外设或子模块一份）
   - `register_bitfields!`：位域、枚举值、W1C 语义在注释里写清
   - `register_structs!`：偏移布局，`ReadOnly` / `ReadWrite` / `WriteOnly` 与手册一致
2. **驱动逻辑** — 通过 struct 字段访问 MMIO，用 trait 方法操作
   - 读：`reg.is_set(FIELD::VARIANT)`、`reg.read(FIELD)`
   - 写：`reg.modify(FIELD::VAL + …)`、`reg.set(val)`（W1C 等按现有驱动惯例）
3. **组装待发写的值** — 用 `LocalRegisterCopy` 在栈上拼寄存器，再一次性 `.set()`

### 参考实现

| 模式 | 文件 |
|------|------|
| bitfields + register_structs + 驱动 | `src/gpio.rs` |
| 大型外设 regs + W1C 注释 | `src/usb/host/dwc2/regs.rs` |
| `LocalRegisterCopy` + `modify` | `src/usb/host/dwc2/channel.rs`, `isoch.rs`, `isr.rs` |

### 推荐写法

```rust
use tock_registers::LocalRegisterCopy;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

// 读状态
if c.hcchar.is_set(HCCHAR::CHENA) { ... }

// 改若干位
c.hcchar.modify(HCCHAR::CHENA::SET + HCCHAR::CHDIS::SET);

// 先拼好再写 MMIO
let mut armed = LocalRegisterCopy::<u32, HCCHAR::Register>::new(hcchar);
armed.modify(HCCHAR::CHENA::SET);
c.hcchar.set(armed.get());
```

### 避免

```rust
// ❌ 裸指针 / read_volatile / write_volatile
unsafe { core::ptr::read_volatile(addr.add(0x10)) };

// ❌ 无命名的 magic mask（除非在 regs.rs 里已定义为 bitfield）
regs.some_reg.set(0x0000_0005);

// ❌ 在业务代码里重复定义已在 regs.rs 存在的位偏移
let en = (val >> 5) & 1;
```

### 例外（需注释说明）

- **协议 payload / USB 描述符 / DMA 缓冲区**：字节流解析，不必强行 tock-registers
- **已有 `regs.rs` 仍用 `pub const` 位常量**（如 `src/dma/regs.rs`）：新字段优先补 bitfield；改旧代码时可逐步迁移，不要在新文件复制 `const SHIFT` 风格
- **RISC-V 自定义 cache 指令、fence**：用 `asm!`，不属于 MMIO

## 新外设 checklist

1. 在 `src/<module>/regs.rs`（或模块内 `regs` 子模块）添加 `register_bitfields!` + `register_structs!`
2. 驱动通过 `&'static RegBlock` 或现有 `unsafe { RegBlock::new(base) }` 模式访问
3. 业务代码只 import bitfield 名（如 `HCCHAR::CHENA`），不散落字面量
4. W1C / R/W1C 在 regs 文件顶部或字段旁注释，写路径与 `channel.rs` / `isr.rs` 保持一致

## 其他约定

- `#![no_std]`，错误类型用 crate 内 `UsbResult` 等，不要引入 std
- 平台相关（计时、`rdtime`、ArceOS trait）**不要**写进 BSP；由上层传入闭包或调用 `axhal`
- Feature 见 `Cargo.toml`：`c906`（D-cache 指令）、`cv182x-host`、`ethernet` 等

## 构建

单独检查本 crate：

```bash
cargo check --target riscv64gc-unknown-none-elf
```
