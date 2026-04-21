//! GMAC MDIO（IEEE 802.3 clause 22）读写。
//!
//! 通过 [`super::regs::Dwc3GmacRegs`] 的 `mii_addr` / `mii_data` 寄存器，
//! 配合 [`super::regs::MII_CLK_CSR_60_100M_DIV42`] 选择 MDC 分频。
//!
//! 所有访问都是 polling，调用方需自行决定超时策略（轮询 ~100k 圈足够 PHY
//! 几十微秒级别的事务完成）。

use tock_registers::interfaces::{Readable, Writeable};

use super::regs::{Dwc3GmacRegs, MII_CLK_CSR_60_100M_DIV42, MiiAddr};

const MDIO_POLL_LIMIT: u32 = 100_000;

#[inline]
fn mdio_wait(gmac: &Dwc3GmacRegs) {
    let mut t = MDIO_POLL_LIMIT;
    while gmac.mii_addr.is_set(MiiAddr::MII_BUSY) {
        t = t.wrapping_sub(1);
        if t == 0 {
            break;
        }
    }
}

/// MDIO clause 22 读：返回 PHY 寄存器的低 16 bit。
pub fn mdio_read(gmac: &Dwc3GmacRegs, phy: u32, reg: u32) -> u16 {
    mdio_wait(gmac);
    gmac.mii_addr.write(
        MiiAddr::MII_PHY.val(phy)
            + MiiAddr::MII_REG.val(reg)
            + MiiAddr::MII_CLK_CSR.val(MII_CLK_CSR_60_100M_DIV42)
            + MiiAddr::MII_BUSY::SET,
    );
    mdio_wait(gmac);
    gmac.mii_data.get() as u16
}

/// MDIO clause 22 写。
pub fn mdio_write(gmac: &Dwc3GmacRegs, phy: u32, reg: u32, data: u16) {
    mdio_wait(gmac);
    gmac.mii_data.set(data as u32);
    gmac.mii_addr.write(
        MiiAddr::MII_PHY.val(phy)
            + MiiAddr::MII_REG.val(reg)
            + MiiAddr::MII_CLK_CSR.val(MII_CLK_CSR_60_100M_DIV42)
            + MiiAddr::MII_WRITE::SET
            + MiiAddr::MII_BUSY::SET,
    );
    mdio_wait(gmac);
}
