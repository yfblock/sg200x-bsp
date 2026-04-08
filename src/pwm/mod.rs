//! # SG2002 PWM 驱动模块
//!
//! 本模块提供 SG2002 芯片 PWM 控制器的 Rust 驱动实现。
//!
//! ## 功能特性
//!
//! - 支持 4 个 PWM 控制器，共 16 路 PWM 输出
//! - 支持连续输出模式和固定脉冲数输出模式
//! - 支持 4 路 PWM 同步输出模式
//! - 支持极性配置
//! - 支持动态更新 PWM 参数
//! - 30 位计数器，支持宽范围频率输出
//!
//! ## 硬件资源
//!
//! SG2002 芯片共有 4 个 PWM 控制器：
//! - PWM0: PWM[0], PWM[1], PWM[2], PWM[3]
//! - PWM1: PWM[4], PWM[5], PWM[6], PWM[7]
//! - PWM2: PWM[8], PWM[9], PWM[10], PWM[11]
//! - PWM3: PWM[12], PWM[13], PWM[14], PWM[15]
//!
//! PWM 时钟源为 100MHz (默认) 或 148.5MHz
//! - 最高输出频率: 50MHz (100MHz/2) 或 74.25MHz (148.5MHz/2)
//! - 最低输出频率: ~0.093Hz (100MHz/(2^30-1))
//!
//! ## 使用示例
//!
//! ### 连续输出模式
//!
//! ```rust,ignore
//! use sg200x_bsp::pwm::{Pwm, PwmInstance, PwmChannel, PwmMode, PwmPolarity};
//!
//! // 创建 PWM0 控制器驱动实例
//! let mut pwm = unsafe { Pwm::new(PwmInstance::Pwm0) };
//!
//! // 配置通道 0: 1KHz, 50% 占空比
//! pwm.configure_channel(
//!     PwmChannel::Channel0,
//!     1_000,      // 1KHz 频率
//!     50,         // 50% 占空比
//!     PwmPolarity::ActiveHigh,
//! )?;
//!
//! // 设置为连续输出模式
//! pwm.set_mode(PwmChannel::Channel0, PwmMode::Continuous);
//!
//! // 使能 IO 输出
//! pwm.enable_output(PwmChannel::Channel0);
//!
//! // 启动 PWM 输出
//! pwm.start(PwmChannel::Channel0);
//!
//! // ... 一段时间后 ...
//!
//! // 停止 PWM 输出
//! pwm.stop(PwmChannel::Channel0);
//! ```
//!
//! ### 固定脉冲数输出模式
//!
//! ```rust,ignore
//! use sg200x_bsp::pwm::{Pwm, PwmInstance, PwmChannel, PwmMode, PwmPolarity};
//!
//! let mut pwm = unsafe { Pwm::new(PwmInstance::Pwm0) };
//!
//! // 配置通道 0: 1MHz, 75% 占空比
//! pwm.configure_channel(
//!     PwmChannel::Channel0,
//!     1_000_000,  // 1MHz 频率
//!     75,         // 75% 占空比 (低电平占比)
//!     PwmPolarity::ActiveHigh,
//! )?;
//!
//! // 设置为固定脉冲数输出模式，输出 16 个脉冲
//! pwm.set_mode(PwmChannel::Channel0, PwmMode::PulseCount);
//! pwm.set_pulse_count(PwmChannel::Channel0, 16)?;
//!
//! // 使能 IO 输出并启动
//! pwm.enable_output(PwmChannel::Channel0);
//! pwm.start(PwmChannel::Channel0);
//!
//! // 等待输出完成
//! while !pwm.is_done(PwmChannel::Channel0) {}
//! ```
//!
//! ### 同步输出模式
//!
//! ```rust,ignore
//! use sg200x_bsp::pwm::{Pwm, PwmInstance, PwmChannel, PwmPolarity};
//!
//! let mut pwm = unsafe { Pwm::new(PwmInstance::Pwm0) };
//!
//! // 配置 4 路相同频率和占空比
//! let freq = 1_000; // 1KHz
//! let duty = 75;    // 75%
//!
//! for ch in 0..4 {
//!     let channel = PwmChannel::from_u8(ch).unwrap();
//!     pwm.configure_channel(channel, freq, duty, PwmPolarity::ActiveHigh)?;
//!     pwm.enable_output(channel);
//! }
//!
//! // 配置相位差 (每路错开 1/4 周期)
//! let period = 100_000_000 / freq; // 周期拍数
//! pwm.configure_shift_mode(
//!     0,                    // 通道 0 相位差
//!     period / 4,           // 通道 1 相位差
//!     period / 2,           // 通道 2 相位差
//!     period * 3 / 4,       // 通道 3 相位差
//! )?;
//!
//! // 启动同步输出
//! pwm.start_shift_mode();
//! ```

#![allow(dead_code)]

mod consts;
mod instances;

pub use consts::{
    PwmChannel, PwmClockSource, PwmError, PwmMode, PwmPolarity, PwmRegisters,
    PWM_DEFAULT_CLK_FREQ, PWM_MAX_PERIOD, PWM_MAX_PULSE_COUNT, PWM_MAX_SHIFT_COUNT,
};
pub use instances::{
    GlobalPwmChannel, PwmInstance, PWM0_BASE, PWM1_BASE, PWM2_BASE, PWM3_BASE,
};

use consts::*;
use tock_registers::interfaces::{ReadWriteable, Readable, Writeable};

/// PWM 驱动结构体
///
/// 提供对 PWM 控制器的访问接口
/// 每个 PWM 控制器包含 4 路独立的 PWM 输出
pub struct Pwm {
    /// PWM 寄存器组引用
    regs: &'static PwmRegisters,
    /// PWM 实例标识
    instance: PwmInstance,
    /// 时钟源
    clock_source: PwmClockSource,
}

impl Pwm {
    /// 创建新的 PWM 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保:
    /// - 寄存器地址有效且可访问
    /// - 不会创建多个相同实例导致数据竞争
    ///
    /// # 参数
    /// - `instance`: PWM 实例标识符
    pub fn new(instance: PwmInstance) -> Self {
        Self::new_with_offset(instance, 0)
    }

    pub fn new_with_offset(instance: PwmInstance, offset: usize) -> Self {
        let base = instance.base_address();
        Self {
            regs: unsafe { &*((base + offset) as *const PwmRegisters) },
            instance,
            clock_source: PwmClockSource::Clk100MHz,
        }
    }


    /// 从指定基地址创建 PWM 驱动实例
    ///
    /// # Safety
    ///
    /// 调用者必须确保基地址有效且可访问
    pub unsafe fn from_base_address(base: usize, instance: PwmInstance) -> Self {
        Self {
            regs: unsafe { &*(base as *const PwmRegisters) },
            instance,
            clock_source: PwmClockSource::Clk100MHz,
        }
    }

    /// 设置时钟源
    pub fn set_clock_source(&mut self, source: PwmClockSource) {
        self.clock_source = source;
    }

    /// 获取时钟源
    pub fn clock_source(&self) -> PwmClockSource {
        self.clock_source
    }

    /// 获取 PWM 实例标识
    pub fn instance(&self) -> PwmInstance {
        self.instance
    }

    /// 获取时钟频率
    pub fn clock_frequency(&self) -> u32 {
        self.clock_source.frequency()
    }

    // ========================================================================
    // 基本配置
    // ========================================================================

    /// 配置 PWM 通道
    ///
    /// # 参数
    /// - `channel`: PWM 通道
    /// - `frequency_hz`: 输出频率 (Hz)
    /// - `duty_percent`: 占空比百分比 (0-100)
    ///   - 当极性为 ActiveHigh 时，duty_percent 为低电平占比
    ///   - 当极性为 ActiveLow 时，duty_percent 为高电平占比
    /// - `polarity`: 信号极性
    ///
    /// # 返回值
    /// - `Ok(())`: 配置成功
    /// - `Err(PwmError)`: 配置失败
    pub fn configure_channel(
        &mut self,
        channel: PwmChannel,
        frequency_hz: u32,
        duty_percent: u8,
        polarity: PwmPolarity,
    ) -> Result<(), PwmError> {
        if frequency_hz == 0 {
            return Err(PwmError::InvalidPeriod);
        }
        if duty_percent > 100 {
            return Err(PwmError::InvalidDutyCycle);
        }

        // 计算周期拍数
        let period = self.clock_frequency() / frequency_hz;
        if period < 2 || period > PWM_MAX_PERIOD {
            return Err(PwmError::InvalidPeriod);
        }

        // 计算低电平/高电平拍数
        let hlperiod = (period as u64 * duty_percent as u64 / 100) as u32;

        // 设置周期和低电平拍数
        self.set_period_raw(channel, period);
        self.set_hlperiod_raw(channel, hlperiod);

        // 设置极性
        self.set_polarity(channel, polarity);

        Ok(())
    }

    /// 配置 PWM 通道 (使用原始拍数值)
    ///
    /// # 参数
    /// - `channel`: PWM 通道
    /// - `period`: 周期拍数 (必须 > hlperiod)
    /// - `hlperiod`: 低电平/高电平拍数
    /// - `polarity`: 信号极性
    pub fn configure_channel_raw(
        &mut self,
        channel: PwmChannel,
        period: u32,
        hlperiod: u32,
        polarity: PwmPolarity,
    ) -> Result<(), PwmError> {
        if period < 2 || period > PWM_MAX_PERIOD {
            return Err(PwmError::InvalidPeriod);
        }
        if hlperiod >= period {
            return Err(PwmError::InvalidDutyCycle);
        }

        self.set_period_raw(channel, period);
        self.set_hlperiod_raw(channel, hlperiod);
        self.set_polarity(channel, polarity);

        Ok(())
    }

    /// 设置周期拍数 (原始值)
    fn set_period_raw(&self, channel: PwmChannel, period: u32) {
        match channel {
            PwmChannel::Channel0 => self.regs.period0.write(PERIOD::PERIOD.val(period)),
            PwmChannel::Channel1 => self.regs.period1.write(PERIOD::PERIOD.val(period)),
            PwmChannel::Channel2 => self.regs.period2.write(PERIOD::PERIOD.val(period)),
            PwmChannel::Channel3 => self.regs.period3.write(PERIOD::PERIOD.val(period)),
        }
    }

    /// 获取周期拍数
    pub fn get_period_raw(&self, channel: PwmChannel) -> u32 {
        match channel {
            PwmChannel::Channel0 => self.regs.period0.read(PERIOD::PERIOD),
            PwmChannel::Channel1 => self.regs.period1.read(PERIOD::PERIOD),
            PwmChannel::Channel2 => self.regs.period2.read(PERIOD::PERIOD),
            PwmChannel::Channel3 => self.regs.period3.read(PERIOD::PERIOD),
        }
    }

    /// 设置低电平/高电平拍数 (原始值)
    fn set_hlperiod_raw(&self, channel: PwmChannel, hlperiod: u32) {
        match channel {
            PwmChannel::Channel0 => self.regs.hlperiod0.write(HLPERIOD::HLPERIOD.val(hlperiod)),
            PwmChannel::Channel1 => self.regs.hlperiod1.write(HLPERIOD::HLPERIOD.val(hlperiod)),
            PwmChannel::Channel2 => self.regs.hlperiod2.write(HLPERIOD::HLPERIOD.val(hlperiod)),
            PwmChannel::Channel3 => self.regs.hlperiod3.write(HLPERIOD::HLPERIOD.val(hlperiod)),
        }
    }

    /// 获取低电平/高电平拍数
    pub fn get_hlperiod_raw(&self, channel: PwmChannel) -> u32 {
        match channel {
            PwmChannel::Channel0 => self.regs.hlperiod0.read(HLPERIOD::HLPERIOD),
            PwmChannel::Channel1 => self.regs.hlperiod1.read(HLPERIOD::HLPERIOD),
            PwmChannel::Channel2 => self.regs.hlperiod2.read(HLPERIOD::HLPERIOD),
            PwmChannel::Channel3 => self.regs.hlperiod3.read(HLPERIOD::HLPERIOD),
        }
    }

    /// 设置信号极性
    pub fn set_polarity(&self, channel: PwmChannel, polarity: PwmPolarity) {
        let mask = channel.mask();
        let current = self.regs.polarity.read(POLARITY::POLARITY);
        let new_val = match polarity {
            PwmPolarity::ActiveHigh => current & !mask,
            PwmPolarity::ActiveLow => current | mask,
        };
        self.regs.polarity.modify(POLARITY::POLARITY.val(new_val));
    }

    /// 获取信号极性
    pub fn get_polarity(&self, channel: PwmChannel) -> PwmPolarity {
        let current = self.regs.polarity.read(POLARITY::POLARITY);
        if current & channel.mask() != 0 {
            PwmPolarity::ActiveLow
        } else {
            PwmPolarity::ActiveHigh
        }
    }

    // ========================================================================
    // 模式配置
    // ========================================================================

    /// 设置工作模式
    ///
    /// # 参数
    /// - `channel`: PWM 通道
    /// - `mode`: 工作模式 (连续输出或固定脉冲数)
    pub fn set_mode(&self, channel: PwmChannel, mode: PwmMode) {
        let mask = channel.mask();
        let current = self.regs.polarity.read(POLARITY::PWMMODE);
        let new_val = match mode {
            PwmMode::Continuous => current & !mask,
            PwmMode::PulseCount => current | mask,
        };
        self.regs.polarity.modify(POLARITY::PWMMODE.val(new_val));
    }

    /// 获取工作模式
    pub fn get_mode(&self, channel: PwmChannel) -> PwmMode {
        let current = self.regs.polarity.read(POLARITY::PWMMODE);
        if current & channel.mask() != 0 {
            PwmMode::PulseCount
        } else {
            PwmMode::Continuous
        }
    }

    /// 设置脉冲数 (仅在 PulseCount 模式下有效)
    ///
    /// # 参数
    /// - `channel`: PWM 通道
    /// - `count`: 脉冲数 (1 - 16777215)
    pub fn set_pulse_count(&self, channel: PwmChannel, count: u32) -> Result<(), PwmError> {
        if count == 0 || count > PWM_MAX_PULSE_COUNT {
            return Err(PwmError::InvalidPulseCount);
        }

        match channel {
            PwmChannel::Channel0 => self.regs.pcount0.write(PCOUNT::PCOUNT.val(count)),
            PwmChannel::Channel1 => self.regs.pcount1.write(PCOUNT::PCOUNT.val(count)),
            PwmChannel::Channel2 => self.regs.pcount2.write(PCOUNT::PCOUNT.val(count)),
            PwmChannel::Channel3 => self.regs.pcount3.write(PCOUNT::PCOUNT.val(count)),
        }

        Ok(())
    }

    /// 获取已输出的脉冲数
    pub fn get_pulse_count(&self, channel: PwmChannel) -> u32 {
        match channel {
            PwmChannel::Channel0 => self.regs.pulsecount0.read(PULSECOUNT::PULSECOUNT),
            PwmChannel::Channel1 => self.regs.pulsecount1.read(PULSECOUNT::PULSECOUNT),
            PwmChannel::Channel2 => self.regs.pulsecount2.read(PULSECOUNT::PULSECOUNT),
            PwmChannel::Channel3 => self.regs.pulsecount3.read(PULSECOUNT::PULSECOUNT),
        }
    }

    // ========================================================================
    // 启动/停止控制
    // ========================================================================

    /// 启动 PWM 输出
    ///
    /// 在连续模式下，PWM 持续输出直到调用 stop()
    /// 在脉冲计数模式下，输出指定数量脉冲后自动停止
    pub fn start(&self, channel: PwmChannel) {
        let mask = channel.mask();
        let current = self.regs.pwmstart.read(PWMSTART::PWMSTART);
        self.regs
            .pwmstart
            .write(PWMSTART::PWMSTART.val(current | mask));
    }

    /// 停止 PWM 输出
    pub fn stop(&self, channel: PwmChannel) {
        let mask = channel.mask();
        let current = self.regs.pwmstart.read(PWMSTART::PWMSTART);
        self.regs
            .pwmstart
            .write(PWMSTART::PWMSTART.val(current & !mask));
    }

    /// 重启 PWM 输出
    ///
    /// 先停止再启动，用于重置计数器和状态寄存器
    pub fn restart(&self, channel: PwmChannel) {
        let mask = channel.mask();
        let current = self.regs.pwmupdate.read(PWMUPDATE::PWMUPDATE);
        self.regs
            .pwmupdate
            .write(PWMUPDATE::PWMUPDATE.val(current | mask));
        self.regs
            .pwmupdate
            .write(PWMUPDATE::PWMUPDATE.val(current));
    }

    /// 检查 PWM 是否正在运行
    pub fn is_running(&self, channel: PwmChannel) -> bool {
        let current = self.regs.pwmstart.read(PWMSTART::PWMSTART);
        current & channel.mask() != 0
    }

    /// 检查 PWM 是否已完成输出 (仅在 PulseCount 模式下有意义)
    pub fn is_done(&self, channel: PwmChannel) -> bool {
        let done = self.regs.pwmdone.read(PWMDONE::PWMDONE);
        done & channel.mask() != 0
    }

    // ========================================================================
    // IO 输出使能
    // ========================================================================

    /// 使能 PWM IO 输出
    pub fn enable_output(&self, channel: PwmChannel) {
        let mask = channel.mask();
        let current = self.regs.pwm_oe.read(PWM_OE::PWM_OE);
        self.regs.pwm_oe.write(PWM_OE::PWM_OE.val(current | mask));
    }

    /// 禁用 PWM IO 输出
    pub fn disable_output(&self, channel: PwmChannel) {
        let mask = channel.mask();
        let current = self.regs.pwm_oe.read(PWM_OE::PWM_OE);
        self.regs.pwm_oe.write(PWM_OE::PWM_OE.val(current & !mask));
    }

    /// 检查 IO 输出是否使能
    pub fn is_output_enabled(&self, channel: PwmChannel) -> bool {
        let current = self.regs.pwm_oe.read(PWM_OE::PWM_OE);
        current & channel.mask() != 0
    }

    // ========================================================================
    // 动态更新
    // ========================================================================

    /// 动态更新 PWM 参数
    ///
    /// 在 PWM 输出过程中更新周期和占空比参数
    /// 需要先写入新的 HLPERIOD 和 PERIOD 值，然后调用此函数使其生效
    pub fn update(&self, channel: PwmChannel) {
        let mask = channel.mask();
        // 写 1 再写 0 使新值生效
        let current = self.regs.pwmupdate.get();
        self.regs.pwmupdate.set(current | mask);
        self.regs.pwmupdate.set(current & !mask);
    }

    /// 动态更新 PWM 频率和占空比
    ///
    /// # 参数
    /// - `channel`: PWM 通道
    /// - `frequency_hz`: 新的输出频率 (Hz)
    /// - `duty_percent`: 新的占空比百分比 (0-100)
    pub fn update_frequency_duty(
        &mut self,
        channel: PwmChannel,
        frequency_hz: u32,
        duty_percent: u8,
    ) -> Result<(), PwmError> {
        if frequency_hz == 0 {
            return Err(PwmError::InvalidPeriod);
        }
        if duty_percent > 100 {
            return Err(PwmError::InvalidDutyCycle);
        }

        let period = self.clock_frequency() / frequency_hz;
        if period < 2 || period > PWM_MAX_PERIOD {
            return Err(PwmError::InvalidPeriod);
        }

        let hlperiod = (period as u64 * duty_percent as u64 / 100) as u32;

        self.set_period_raw(channel, period);
        self.set_hlperiod_raw(channel, hlperiod);
        self.update(channel);

        Ok(())
    }

    // ========================================================================
    // 同步模式
    // ========================================================================

    /// 使能同步模式
    ///
    /// 在同步模式下，4 路 PWM 可以配置不同的相位差同步输出
    pub fn enable_shift_mode(&self) {
        self.regs.polarity.modify(POLARITY::SHIFTMODE::SET);
    }

    /// 禁用同步模式
    pub fn disable_shift_mode(&self) {
        self.regs.polarity.modify(POLARITY::SHIFTMODE::CLEAR);
    }

    /// 检查同步模式是否使能
    pub fn is_shift_mode_enabled(&self) -> bool {
        self.regs.polarity.is_set(POLARITY::SHIFTMODE)
    }

    /// 配置同步模式相位差
    ///
    /// # 参数
    /// - `shift0`: 通道 0 相位差 (clk_pwm 拍数)
    /// - `shift1`: 通道 1 相位差 (clk_pwm 拍数)
    /// - `shift2`: 通道 2 相位差 (clk_pwm 拍数)
    /// - `shift3`: 通道 3 相位差 (clk_pwm 拍数)
    pub fn configure_shift_mode(
        &self,
        shift0: u32,
        shift1: u32,
        shift2: u32,
        shift3: u32,
    ) -> Result<(), PwmError> {
        if shift0 > PWM_MAX_SHIFT_COUNT
            || shift1 > PWM_MAX_SHIFT_COUNT
            || shift2 > PWM_MAX_SHIFT_COUNT
            || shift3 > PWM_MAX_SHIFT_COUNT
        {
            return Err(PwmError::ConfigError);
        }

        self.regs.shiftcount0.write(SHIFTCOUNT::SHIFTCOUNT.val(shift0));
        self.regs.shiftcount1.write(SHIFTCOUNT::SHIFTCOUNT.val(shift1));
        self.regs.shiftcount2.write(SHIFTCOUNT::SHIFTCOUNT.val(shift2));
        self.regs.shiftcount3.write(SHIFTCOUNT::SHIFTCOUNT.val(shift3));

        self.enable_shift_mode();

        Ok(())
    }

    /// 设置单个通道的相位差
    pub fn set_shift_count(&self, channel: PwmChannel, shift: u32) -> Result<(), PwmError> {
        if shift > PWM_MAX_SHIFT_COUNT {
            return Err(PwmError::ConfigError);
        }

        match channel {
            PwmChannel::Channel0 => self.regs.shiftcount0.write(SHIFTCOUNT::SHIFTCOUNT.val(shift)),
            PwmChannel::Channel1 => self.regs.shiftcount1.write(SHIFTCOUNT::SHIFTCOUNT.val(shift)),
            PwmChannel::Channel2 => self.regs.shiftcount2.write(SHIFTCOUNT::SHIFTCOUNT.val(shift)),
            PwmChannel::Channel3 => self.regs.shiftcount3.write(SHIFTCOUNT::SHIFTCOUNT.val(shift)),
        }

        Ok(())
    }

    /// 获取通道的相位差
    pub fn get_shift_count(&self, channel: PwmChannel) -> u32 {
        match channel {
            PwmChannel::Channel0 => self.regs.shiftcount0.read(SHIFTCOUNT::SHIFTCOUNT),
            PwmChannel::Channel1 => self.regs.shiftcount1.read(SHIFTCOUNT::SHIFTCOUNT),
            PwmChannel::Channel2 => self.regs.shiftcount2.read(SHIFTCOUNT::SHIFTCOUNT),
            PwmChannel::Channel3 => self.regs.shiftcount3.read(SHIFTCOUNT::SHIFTCOUNT),
        }
    }

    /// 启动同步模式输出
    ///
    /// 在调用此函数前，需要:
    /// 1. 配置各通道的周期和占空比
    /// 2. 配置相位差
    /// 3. 使能各通道的 PWMSTART
    /// 4. 使能各通道的 IO 输出
    pub fn start_shift_mode(&self) {
        self.regs.shiftstart.write(SHIFTSTART::SHIFTSTART::SET);
    }

    /// 停止同步模式输出
    pub fn stop_shift_mode(&self) {
        self.regs.shiftstart.write(SHIFTSTART::SHIFTSTART::CLEAR);
    }

    /// 启动所有 4 路 PWM (用于同步模式)
    pub fn start_all(&self) {
        self.regs.pwmstart.write(PWMSTART::PWMSTART.val(0xF));
    }

    /// 停止所有 4 路 PWM
    pub fn stop_all(&self) {
        self.regs.pwmstart.write(PWMSTART::PWMSTART.val(0));
    }

    /// 使能所有 4 路 IO 输出
    pub fn enable_all_outputs(&self) {
        self.regs.pwm_oe.write(PWM_OE::PWM_OE.val(0xF));
    }

    /// 禁用所有 4 路 IO 输出
    pub fn disable_all_outputs(&self) {
        self.regs.pwm_oe.write(PWM_OE::PWM_OE.val(0));
    }

    // ========================================================================
    // APB 时钟门控
    // ========================================================================

    /// 强制 APB 时钟常开
    pub fn force_pclk_on(&self) {
        self.regs.polarity.modify(POLARITY::PCLK_FORCE_EN::SET);
    }

    /// 使能 APB 时钟门控 (空闲时自动关闭)
    pub fn enable_pclk_gating(&self) {
        self.regs.polarity.modify(POLARITY::PCLK_FORCE_EN::CLEAR);
    }
}

