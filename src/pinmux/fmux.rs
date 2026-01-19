//! FMUX (Function Mux) 寄存器定义
//!
//! FMUX 寄存器用于选择每个引脚的功能模式。
//! 基地址: 0x0300_1000
//!
//! 每个引脚有一个 32 位寄存器，低 3 位用于功能选择 (最多 8 种功能)。

use tock_registers::{register_bitfields, register_structs, registers::ReadWrite};

// ============================================================================
// FMUX 寄存器位域定义
// ============================================================================

register_bitfields! [
    u32,

    /// SD0_CLK 功能选择
    pub FMUX_SD0_CLK [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_CLK (默认)
            SDIO0_CLK = 0,
            /// IIC1_SDA
            IIC1_SDA = 1,
            /// SPI0_SCK
            SPI0_SCK = 2,
            /// XGPIOA[7]
            XGPIOA_7 = 3,
            /// PWM[15]
            PWM_15 = 5,
            /// EPHY_LNK_LED
            EPHY_LNK_LED = 6,
            /// DBG[0]
            DBG_0 = 7
        ]
    ],

    /// SD0_CMD 功能选择
    pub FMUX_SD0_CMD [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_CMD (默认)
            SDIO0_CMD = 0,
            /// IIC1_SCL
            IIC1_SCL = 1,
            /// SPI0_SDO
            SPI0_SDO = 2,
            /// XGPIOA[8]
            XGPIOA_8 = 3,
            /// PWM[14]
            PWM_14 = 5,
            /// EPHY_SPD_LED
            EPHY_SPD_LED = 6,
            /// DBG[1]
            DBG_1 = 7
        ]
    ],

    /// SD0_D0 功能选择
    pub FMUX_SD0_D0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_D[0] (默认)
            SDIO0_D0 = 0,
            /// CAM_MCLK1
            CAM_MCLK1 = 1,
            /// SPI0_SDI
            SPI0_SDI = 2,
            /// XGPIOA[9]
            XGPIOA_9 = 3,
            /// UART3_TX
            UART3_TX = 4,
            /// PWM[13]
            PWM_13 = 5,
            /// WG0_D0
            WG0_D0 = 6,
            /// DBG[2]
            DBG_2 = 7
        ]
    ],

    /// SD0_D1 功能选择
    pub FMUX_SD0_D1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_D[1] (默认)
            SDIO0_D1 = 0,
            /// IIC1_SDA
            IIC1_SDA = 1,
            /// AUX0
            AUX0 = 2,
            /// XGPIOA[10]
            XGPIOA_10 = 3,
            /// UART1_TX
            UART1_TX = 4,
            /// PWM[12]
            PWM_12 = 5,
            /// WG0_D1
            WG0_D1 = 6,
            /// DBG[3]
            DBG_3 = 7
        ]
    ],

    /// SD0_D2 功能选择
    pub FMUX_SD0_D2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_D[2] (默认)
            SDIO0_D2 = 0,
            /// IIC1_SCL
            IIC1_SCL = 1,
            /// AUX1
            AUX1 = 2,
            /// XGPIOA[11]
            XGPIOA_11 = 3,
            /// UART1_RX
            UART1_RX = 4,
            /// PWM[11]
            PWM_11 = 5,
            /// WG1_D0
            WG1_D0 = 6,
            /// DBG[4]
            DBG_4 = 7
        ]
    ],

    /// SD0_D3 功能选择
    pub FMUX_SD0_D3 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_D[3] (默认)
            SDIO0_D3 = 0,
            /// CAM_MCLK0
            CAM_MCLK0 = 1,
            /// SPI0_CS_X
            SPI0_CS_X = 2,
            /// XGPIOA[12]
            XGPIOA_12 = 3,
            /// UART3_RX
            UART3_RX = 4,
            /// PWM[10]
            PWM_10 = 5,
            /// WG1_D1
            WG1_D1 = 6,
            /// DBG[5]
            DBG_5 = 7
        ]
    ],

    /// SD0_CD 功能选择
    pub FMUX_SD0_CD [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_CD (默认)
            SDIO0_CD = 0,
            /// XGPIOA[13]
            XGPIOA_13 = 3
        ]
    ],

    /// SD0_PWR_EN 功能选择
    pub FMUX_SD0_PWR_EN [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// SDIO0_PWR_EN
            SDIO0_PWR_EN = 0,
            /// XGPIOA[14] (默认)
            XGPIOA_14 = 3
        ]
    ],

    /// SPK_EN 功能选择
    pub FMUX_SPK_EN [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// XGPIOA[15] (默认)
            XGPIOA_15 = 3
        ]
    ],

    /// UART0_TX 功能选择
    pub FMUX_UART0_TX [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART0_TX (默认)
            UART0_TX = 0,
            /// CAM_MCLK1
            CAM_MCLK1 = 1,
            /// PWM[4]
            PWM_4 = 2,
            /// XGPIOA[16]
            XGPIOA_16 = 3,
            /// UART1_TX
            UART1_TX = 4,
            /// AUX1
            AUX1 = 5,
            /// DBG[6]
            DBG_6 = 7
        ]
    ],

    /// UART0_RX 功能选择
    pub FMUX_UART0_RX [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART0_RX (默认)
            UART0_RX = 0,
            /// CAM_MCLK0
            CAM_MCLK0 = 1,
            /// PWM[5]
            PWM_5 = 2,
            /// XGPIOA[17]
            XGPIOA_17 = 3,
            /// UART1_RX
            UART1_RX = 4,
            /// AUX0
            AUX0 = 5,
            /// DBG[7]
            DBG_7 = 7
        ]
    ],

    /// EMMC_DAT2 功能选择
    pub FMUX_EMMC_DAT2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_DAT[2]
            EMMC_DAT2 = 0,
            /// SPINOR_HOLD_X (默认)
            SPINOR_HOLD_X = 1,
            /// SPINAND_HOLD
            SPINAND_HOLD = 2,
            /// XGPIOA[26]
            XGPIOA_26 = 3
        ]
    ],

    /// EMMC_CLK 功能选择
    pub FMUX_EMMC_CLK [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_CLK
            EMMC_CLK = 0,
            /// SPINOR_SCK (默认)
            SPINOR_SCK = 1,
            /// SPINAND_CLK
            SPINAND_CLK = 2,
            /// XGPIOA[22]
            XGPIOA_22 = 3
        ]
    ],

    /// EMMC_DAT0 功能选择
    pub FMUX_EMMC_DAT0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_DAT[0]
            EMMC_DAT0 = 0,
            /// SPINOR_MOSI (默认)
            SPINOR_MOSI = 1,
            /// SPINAND_MOSI
            SPINAND_MOSI = 2,
            /// XGPIOA[25]
            XGPIOA_25 = 3
        ]
    ],

    /// EMMC_DAT3 功能选择
    pub FMUX_EMMC_DAT3 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_DAT[3]
            EMMC_DAT3 = 0,
            /// SPINOR_WP_X (默认)
            SPINOR_WP_X = 1,
            /// SPINAND_WP
            SPINAND_WP = 2,
            /// XGPIOA[27]
            XGPIOA_27 = 3
        ]
    ],

    /// EMMC_CMD 功能选择
    pub FMUX_EMMC_CMD [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_CMD
            EMMC_CMD = 0,
            /// SPINOR_MISO (默认)
            SPINOR_MISO = 1,
            /// SPINAND_MISO
            SPINAND_MISO = 2,
            /// XGPIOA[23]
            XGPIOA_23 = 3
        ]
    ],

    /// EMMC_DAT1 功能选择
    pub FMUX_EMMC_DAT1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// EMMC_DAT[1]
            EMMC_DAT1 = 0,
            /// SPINOR_CS_X (默认)
            SPINOR_CS_X = 1,
            /// SPINAND_CS
            SPINAND_CS = 2,
            /// XGPIOA[24]
            XGPIOA_24 = 3
        ]
    ],

    /// JTAG_CPU_TMS 功能选择
    pub FMUX_JTAG_CPU_TMS [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_4WTMS (默认)
            CR_4WTMS = 0,
            /// CAM_MCLK0
            CAM_MCLK0 = 1,
            /// PWM[7]
            PWM_7 = 2,
            /// XGPIOA[19]
            XGPIOA_19 = 3,
            /// UART1_RTS
            UART1_RTS = 4,
            /// AUX0
            AUX0 = 5,
            /// UART1_TX
            UART1_TX = 6,
            /// VO_D[28]
            VO_D28 = 7
        ]
    ],

    /// JTAG_CPU_TCK 功能选择
    pub FMUX_JTAG_CPU_TCK [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_4WTCK (默认)
            CR_4WTCK = 0,
            /// CAM_MCLK1
            CAM_MCLK1 = 1,
            /// PWM[6]
            PWM_6 = 2,
            /// XGPIOA[18]
            XGPIOA_18 = 3,
            /// UART1_CTS
            UART1_CTS = 4,
            /// AUX1
            AUX1 = 5,
            /// UART1_RX
            UART1_RX = 6,
            /// VO_D[29]
            VO_D29 = 7
        ]
    ],

    /// IIC0_SCL 功能选择
    pub FMUX_IIC0_SCL [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_4WTDI (默认)
            CR_4WTDI = 0,
            /// UART1_TX
            UART1_TX = 1,
            /// UART2_TX
            UART2_TX = 2,
            /// XGPIOA[28]
            XGPIOA_28 = 3,
            /// WG0_D0
            WG0_D0 = 5,
            /// DBG[10]
            DBG_10 = 7
        ]
    ],

    /// IIC0_SDA 功能选择
    pub FMUX_IIC0_SDA [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_4WTDO (默认)
            CR_4WTDO = 0,
            /// UART1_RX
            UART1_RX = 1,
            /// UART2_RX
            UART2_RX = 2,
            /// XGPIOA[29]
            XGPIOA_29 = 3,
            /// WG0_D1
            WG0_D1 = 5,
            /// WG1_D0
            WG1_D0 = 6,
            /// DBG[11]
            DBG_11 = 7
        ]
    ],

    /// AUX0 功能选择
    pub FMUX_AUX0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// AUX0
            AUX0 = 0,
            /// XGPIOA[30] (默认)
            XGPIOA_30 = 3,
            /// IIS1_MCLK
            IIS1_MCLK = 4,
            /// VO_D[31]
            VO_D31 = 5,
            /// WG1_D1
            WG1_D1 = 6,
            /// DBG[12]
            DBG_12 = 7
        ]
    ],

    /// PWR_VBAT_DET 功能选择
    pub FMUX_PWR_VBAT_DET [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_VBAT_DET (默认)
            PWR_VBAT_DET = 0
        ]
    ],

    /// PWR_RSTN 功能选择
    pub FMUX_PWR_RSTN [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_RSTN (默认)
            PWR_RSTN = 0
        ]
    ],

    /// PWR_SEQ1 功能选择
    pub FMUX_PWR_SEQ1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_SEQ1 (默认)
            PWR_SEQ1 = 0,
            /// PWR_GPIO[3]
            PWR_GPIO_3 = 3
        ]
    ],

    /// PWR_SEQ2 功能选择
    pub FMUX_PWR_SEQ2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_SEQ2 (默认)
            PWR_SEQ2 = 0,
            /// PWR_GPIO[4]
            PWR_GPIO_4 = 3
        ]
    ],

    /// PWR_WAKEUP0 功能选择
    pub FMUX_PWR_WAKEUP0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_WAKEUP0 (默认)
            PWR_WAKEUP0 = 0,
            /// PWR_IR0
            PWR_IR0 = 1,
            /// PWR_UART0_TX
            PWR_UART0_TX = 2,
            /// PWR_GPIO[6]
            PWR_GPIO_6 = 3,
            /// UART1_TX
            UART1_TX = 4,
            /// IIC4_SCL
            IIC4_SCL = 5,
            /// EPHY_LNK_LED
            EPHY_LNK_LED = 6,
            /// WG2_D0
            WG2_D0 = 7
        ]
    ],

    /// PWR_BUTTON1 功能选择
    pub FMUX_PWR_BUTTON1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_BUTTON1 (默认)
            PWR_BUTTON1 = 0,
            /// PWR_GPIO[8]
            PWR_GPIO_8 = 3,
            /// UART1_RX
            UART1_RX = 4,
            /// IIC4_SDA
            IIC4_SDA = 5,
            /// EPHY_SPD_LED
            EPHY_SPD_LED = 6,
            /// WG2_D1
            WG2_D1 = 7
        ]
    ],

    /// XTAL_XIN 功能选择
    pub FMUX_XTAL_XIN [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_XTAL_CLKIN (默认)
            PWR_XTAL_CLKIN = 0
        ]
    ],

    /// PWR_GPIO0 功能选择
    pub FMUX_PWR_GPIO0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_GPIO[0] (默认)
            PWR_GPIO_0 = 0,
            /// UART2_TX
            UART2_TX = 1,
            /// PWR_UART0_RX
            PWR_UART0_RX = 2,
            /// PWM[8]
            PWM_8 = 4
        ]
    ],

    /// PWR_GPIO1 功能选择
    pub FMUX_PWR_GPIO1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_GPIO[1] (默认)
            PWR_GPIO_1 = 0,
            /// UART2_RX
            UART2_RX = 1,
            /// PWM[9]
            PWM_9 = 4,
            /// EPHY_LNK_LED
            EPHY_LNK_LED = 6,
            /// WG0_D0
            WG0_D0 = 7
        ]
    ],

    /// PWR_GPIO2 功能选择
    pub FMUX_PWR_GPIO2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_GPIO[2] (默认)
            PWR_GPIO_2 = 0,
            /// PWR_SECTICK
            PWR_SECTICK = 2,
            /// PWM[10]
            PWM_10 = 4,
            /// EPHY_SPD_LED
            EPHY_SPD_LED = 6,
            /// WG0_D1
            WG0_D1 = 7
        ]
    ],

    /// ADC1 功能选择
    pub FMUX_ADC1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// ADC1 (默认)
            ADC1 = 0
        ]
    ],

    /// USB_VBUS_DET 功能选择
    pub FMUX_USB_VBUS_DET [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// USB_VBUS_DET (默认)
            USB_VBUS_DET = 0,
            /// XGPIOB[6]
            XGPIOB_6 = 3,
            /// CAM_MCLK0
            CAM_MCLK0 = 4,
            /// CAM_MCLK1
            CAM_MCLK1 = 5
        ]
    ],

    /// MUX_SPI1_MISO 功能选择
    pub FMUX_MUX_SPI1_MISO [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_RTS
            UART3_RTS = 1,
            /// IIC1_SDA
            IIC1_SDA = 2,
            /// XGPIOB[8] (默认)
            XGPIOB_8 = 3,
            /// PWM[9]
            PWM_9 = 4,
            /// KEY_COL1
            KEY_COL1 = 5,
            /// SPI1_SDI
            SPI1_SDI = 6,
            /// DBG[14]
            DBG_14 = 7
        ]
    ],

    /// MUX_SPI1_MOSI 功能选择
    pub FMUX_MUX_SPI1_MOSI [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_RX
            UART3_RX = 1,
            /// IIC1_SCL
            IIC1_SCL = 2,
            /// XGPIOB[7] (默认)
            XGPIOB_7 = 3,
            /// PWM[8]
            PWM_8 = 4,
            /// KEY_COL0
            KEY_COL0 = 5,
            /// SPI1_SDO
            SPI1_SDO = 6,
            /// DBG[13]
            DBG_13 = 7
        ]
    ],

    /// MUX_SPI1_CS 功能选择
    pub FMUX_MUX_SPI1_CS [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_CTS
            UART3_CTS = 1,
            /// CAM_MCLK0
            CAM_MCLK0 = 2,
            /// XGPIOB[10] (默认)
            XGPIOB_10 = 3,
            /// PWM[11]
            PWM_11 = 4,
            /// KEY_ROW3
            KEY_ROW3 = 5,
            /// SPI1_CS_X
            SPI1_CS_X = 6,
            /// DBG[16]
            DBG_16 = 7
        ]
    ],

    /// MUX_SPI1_SCK 功能选择
    pub FMUX_MUX_SPI1_SCK [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_TX
            UART3_TX = 1,
            /// CAM_MCLK1
            CAM_MCLK1 = 2,
            /// XGPIOB[9] (默认)
            XGPIOB_9 = 3,
            /// PWM[10]
            PWM_10 = 4,
            /// KEY_ROW2
            KEY_ROW2 = 5,
            /// SPI1_SCK
            SPI1_SCK = 6,
            /// DBG[15]
            DBG_15 = 7
        ]
    ],

    /// PAD_ETH_TXP 功能选择
    pub FMUX_PAD_ETH_TXP [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_RX
            UART3_RX = 1,
            /// IIC1_SCL
            IIC1_SCL = 2,
            /// XGPIOB[25] (默认)
            XGPIOB_25 = 3,
            /// PWM[13]
            PWM_13 = 4,
            /// CAM_MCLK0
            CAM_MCLK0 = 5,
            /// SPI1_SDO
            SPI1_SDO = 6,
            /// IIS2_LRCK
            IIS2_LRCK = 7
        ]
    ],

    /// PAD_ETH_RXP 功能选择
    pub FMUX_PAD_ETH_RXP [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_TX
            UART3_TX = 1,
            /// CAM_MCLK1
            CAM_MCLK1 = 2,
            /// XGPIOB[27] (默认)
            XGPIOB_27 = 3,
            /// PWM[15]
            PWM_15 = 4,
            /// CAM_HS0
            CAM_HS0 = 5,
            /// SPI1_SCK
            SPI1_SCK = 6,
            /// IIS2_DO
            IIS2_DO = 7
        ]
    ],

    /// PAD_ETH_RXM 功能选择
    pub FMUX_PAD_ETH_RXM [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// UART3_CTS
            UART3_CTS = 1,
            /// CAM_MCLK0
            CAM_MCLK0 = 2,
            /// XGPIOB[26] (默认)
            XGPIOB_26 = 3,
            /// PWM[14]
            PWM_14 = 4,
            /// CAM_VS0
            CAM_VS0 = 5,
            /// SPI1_CS_X
            SPI1_CS_X = 6,
            /// IIS2_DI
            IIS2_DI = 7
        ]
    ],

    /// PAD_MIPIRX4N 功能选择
    pub FMUX_PAD_MIPIRX4N [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_SCL0 (默认)
            CR_SCL0 = 0,
            /// VI0_CLK
            VI0_CLK = 1,
            /// VI1_D[13]
            VI1_D13 = 2,
            /// XGPIOC[2]
            XGPIOC_2 = 3,
            /// IIC1_SDA
            IIC1_SDA = 4,
            /// CAM_MCLK0
            CAM_MCLK0 = 5,
            /// KEY_ROW0
            KEY_ROW0 = 6,
            /// MUX_SPI1_SCK
            MUX_SPI1_SCK = 7
        ]
    ],

    /// PAD_MIPIRX4P 功能选择
    pub FMUX_PAD_MIPIRX4P [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_SDA0 (默认)
            CR_SDA0 = 0,
            /// VI0_D[0]
            VI0_D0 = 1,
            /// VI1_D[14]
            VI1_D14 = 2,
            /// XGPIOC[3]
            XGPIOC_3 = 3,
            /// IIC1_SCL
            IIC1_SCL = 4,
            /// CAM_MCLK1
            CAM_MCLK1 = 5,
            /// KEY_ROW1
            KEY_ROW1 = 6,
            /// MUX_SPI1_CS
            MUX_SPI1_CS = 7
        ]
    ],

    /// PAD_MIPIRX3N 功能选择
    pub FMUX_PAD_MIPIRX3N [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_2WTMS
            CR_2WTMS = 0,
            /// VI0_D[1]
            VI0_D1 = 1,
            /// VI1_D[15]
            VI1_D15 = 2,
            /// XGPIOC[4] (默认)
            XGPIOC_4 = 3,
            /// CAM_MCLK0
            CAM_MCLK0 = 4,
            /// MUX_SPI1_MISO
            MUX_SPI1_MISO = 7
        ]
    ],

    /// PAD_MIPIRX3P 功能选择
    pub FMUX_PAD_MIPIRX3P [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_2WTCK
            CR_2WTCK = 0,
            /// VI0_D[2]
            VI0_D2 = 1,
            /// VI1_D[16]
            VI1_D16 = 2,
            /// XGPIOC[5] (默认)
            XGPIOC_5 = 3,
            /// MUX_SPI1_MOSI
            MUX_SPI1_MOSI = 7
        ]
    ],

    /// PAD_MIPIRX2N 功能选择
    pub FMUX_PAD_MIPIRX2N [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[3]
            VI0_D3 = 1,
            /// VO_D[10]
            VO_D10 = 2,
            /// XGPIOC[6] (默认)
            XGPIOC_6 = 3,
            /// VI1_D[17]
            VI1_D17 = 4,
            /// IIC4_SCL
            IIC4_SCL = 5,
            /// DBG[6]
            DBG_6 = 7
        ]
    ],

    /// PAD_MIPIRX2P 功能选择
    pub FMUX_PAD_MIPIRX2P [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[4]
            VI0_D4 = 1,
            /// VO_D[9]
            VO_D9 = 2,
            /// XGPIOC[7] (默认)
            XGPIOC_7 = 3,
            /// VI1_D[18]
            VI1_D18 = 4,
            /// IIC4_SDA
            IIC4_SDA = 5,
            /// DBG[7]
            DBG_7 = 7
        ]
    ],

    /// PAD_MIPIRX1N 功能选择
    pub FMUX_PAD_MIPIRX1N [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[5]
            VI0_D5 = 1,
            /// VO_D[8]
            VO_D8 = 2,
            /// XGPIOC[8] (默认)
            XGPIOC_8 = 3,
            /// KEY_ROW3
            KEY_ROW3 = 6,
            /// DBG[8]
            DBG_8 = 7
        ]
    ],

    /// PAD_MIPIRX1P 功能选择
    pub FMUX_PAD_MIPIRX1P [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[6]
            VI0_D6 = 1,
            /// VO_D[7]
            VO_D7 = 2,
            /// XGPIOC[9] (默认)
            XGPIOC_9 = 3,
            /// IIC1_SDA
            IIC1_SDA = 4,
            /// KEY_ROW2
            KEY_ROW2 = 6,
            /// DBG[9]
            DBG_9 = 7
        ]
    ],

    /// PAD_MIPIRX0N 功能选择
    pub FMUX_PAD_MIPIRX0N [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[7]
            VI0_D7 = 1,
            /// VO_D[6]
            VO_D6 = 2,
            /// XGPIOC[10] (默认)
            XGPIOC_10 = 3,
            /// IIC1_SCL
            IIC1_SCL = 4,
            /// CAM_MCLK1
            CAM_MCLK1 = 5,
            /// DBG[10]
            DBG_10 = 7
        ]
    ],

    /// PAD_MIPIRX0P 功能选择
    pub FMUX_PAD_MIPIRX0P [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[8]
            VI0_D8 = 1,
            /// VO_D[5]
            VO_D5 = 2,
            /// XGPIOC[11] (默认)
            XGPIOC_11 = 3,
            /// CAM_MCLK0
            CAM_MCLK0 = 4,
            /// DBG[11]
            DBG_11 = 7
        ]
    ],

    /// PAD_MIPI_TXM2 功能选择
    pub FMUX_PAD_MIPI_TXM2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_SDA0
            CR_SDA0 = 0,
            /// VI0_D[13]
            VI0_D13 = 1,
            /// VO_D[0]
            VO_D0 = 2,
            /// XGPIOC[16] (默认)
            XGPIOC_16 = 3,
            /// IIC1_SDA
            IIC1_SDA = 4,
            /// PWM[8]
            PWM_8 = 5,
            /// SPI0_SCK
            SPI0_SCK = 6,
            /// SD1_D2
            SD1_D2 = 7
        ]
    ],

    /// PAD_MIPI_TXP2 功能选择
    pub FMUX_PAD_MIPI_TXP2 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_SCL0
            CR_SCL0 = 0,
            /// VI0_D[14]
            VI0_D14 = 1,
            /// VO_CLK0
            VO_CLK0 = 2,
            /// XGPIOC[17] (默认)
            XGPIOC_17 = 3,
            /// IIC1_SCL
            IIC1_SCL = 4,
            /// PWM[9]
            PWM_9 = 5,
            /// SPI0_CS_X
            SPI0_CS_X = 6,
            /// SD1_D3
            SD1_D3 = 7
        ]
    ],

    /// PAD_MIPI_TXM1 功能选择
    pub FMUX_PAD_MIPI_TXM1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_2WTMS
            CR_2WTMS = 0,
            /// VI0_D[11]
            VI0_D11 = 1,
            /// VO_D[2]
            VO_D2 = 2,
            /// XGPIOC[14] (默认)
            XGPIOC_14 = 3,
            /// IIC2_SDA
            IIC2_SDA = 4,
            /// PWM[10]
            PWM_10 = 5,
            /// SPI0_SDO
            SPI0_SDO = 6,
            /// DBG[14]
            DBG_14 = 7
        ]
    ],

    /// PAD_MIPI_TXP1 功能选择
    pub FMUX_PAD_MIPI_TXP1 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// CR_2WTCK
            CR_2WTCK = 0,
            /// VI0_D[12]
            VI0_D12 = 1,
            /// VO_D[1]
            VO_D1 = 2,
            /// XGPIOC[15] (默认)
            XGPIOC_15 = 3,
            /// IIC2_SCL
            IIC2_SCL = 4,
            /// PWM[11]
            PWM_11 = 5,
            /// SPI0_SDI
            SPI0_SDI = 6,
            /// DBG[15]
            DBG_15 = 7
        ]
    ],

    /// PAD_MIPI_TXM0 功能选择
    pub FMUX_PAD_MIPI_TXM0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[9]
            VI0_D9 = 1,
            /// VO_D[4]
            VO_D4 = 2,
            /// XGPIOC[12] (默认)
            XGPIOC_12 = 3,
            /// CAM_MCLK1
            CAM_MCLK1 = 4,
            /// PWM[14]
            PWM_14 = 5,
            /// CAM_VS0
            CAM_VS0 = 6,
            /// DBG[12]
            DBG_12 = 7
        ]
    ],

    /// PAD_MIPI_TXP0 功能选择
    pub FMUX_PAD_MIPI_TXP0 [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// VI0_D[10]
            VI0_D10 = 1,
            /// VO_D[3]
            VO_D3 = 2,
            /// XGPIOC[13] (默认)
            XGPIOC_13 = 3,
            /// CAM_MCLK0
            CAM_MCLK0 = 4,
            /// PWM[15]
            PWM_15 = 5,
            /// CAM_HS0
            CAM_HS0 = 6,
            /// DBG[13]
            DBG_13 = 7
        ]
    ],

    /// PAD_AUD_AINL_MIC 功能选择
    pub FMUX_PAD_AUD_AINL_MIC [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// XGPIOC[23] (默认)
            XGPIOC_23 = 3,
            /// IIS1_BCLK
            IIS1_BCLK = 4,
            /// IIS2_BCLK
            IIS2_BCLK = 5
        ]
    ],

    /// PAD_AUD_AOUTR 功能选择
    pub FMUX_PAD_AUD_AOUTR [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// XGPIOC[24] (默认)
            XGPIOC_24 = 3,
            /// IIS1_DI
            IIS1_DI = 4,
            /// IIS2_DO
            IIS2_DO = 5,
            /// IIS1_DO
            IIS1_DO = 6
        ]
    ],

    /// GPIO_RTX 功能选择
    pub FMUX_GPIO_RTX [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// XGPIOB[23] (默认)
            XGPIOB_23 = 3,
            /// PWM[1]
            PWM_1 = 4,
            /// CAM_MCLK0
            CAM_MCLK0 = 5
        ]
    ],

    /// GPIO_ZQ 功能选择
    pub FMUX_GPIO_ZQ [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) [
            /// PWR_GPIO[24] (默认)
            PWR_GPIO_24 = 3,
            /// PWM[2]
            PWM_2 = 4
        ]
    ],

    /// PKG_TYPE 功能选择 (通用)
    pub FMUX_PKG_TYPE [
        /// 功能选择
        FSEL OFFSET(0) NUMBITS(3) []
    ]
];

// ============================================================================
// FMUX 寄存器结构体定义
// ============================================================================

register_structs! {
    /// FMUX 功能选择寄存器组
    ///
    /// 用于配置 SG2002 芯片各引脚的功能复用
    pub FmuxRegisters {
        /// 偏移 0x00-0x18: 保留
        (0x000 => _reserved0),

        /// SD0_CLK 功能选择寄存器 (偏移 0x1C)
        (0x01C => pub sd0_clk: ReadWrite<u32, FMUX_SD0_CLK::Register>),

        /// SD0_CMD 功能选择寄存器 (偏移 0x20)
        (0x020 => pub sd0_cmd: ReadWrite<u32, FMUX_SD0_CMD::Register>),

        /// SD0_D0 功能选择寄存器 (偏移 0x24)
        (0x024 => pub sd0_d0: ReadWrite<u32, FMUX_SD0_D0::Register>),

        /// SD0_D1 功能选择寄存器 (偏移 0x28)
        (0x028 => pub sd0_d1: ReadWrite<u32, FMUX_SD0_D1::Register>),

        /// SD0_D2 功能选择寄存器 (偏移 0x2C)
        (0x02C => pub sd0_d2: ReadWrite<u32, FMUX_SD0_D2::Register>),

        /// SD0_D3 功能选择寄存器 (偏移 0x30)
        (0x030 => pub sd0_d3: ReadWrite<u32, FMUX_SD0_D3::Register>),

        /// SD0_CD 功能选择寄存器 (偏移 0x34)
        (0x034 => pub sd0_cd: ReadWrite<u32, FMUX_SD0_CD::Register>),

        /// SD0_PWR_EN 功能选择寄存器 (偏移 0x38)
        (0x038 => pub sd0_pwr_en: ReadWrite<u32, FMUX_SD0_PWR_EN::Register>),

        /// SPK_EN 功能选择寄存器 (偏移 0x3C)
        (0x03C => pub spk_en: ReadWrite<u32, FMUX_SPK_EN::Register>),

        /// UART0_TX 功能选择寄存器 (偏移 0x40)
        (0x040 => pub uart0_tx: ReadWrite<u32, FMUX_UART0_TX::Register>),

        /// UART0_RX 功能选择寄存器 (偏移 0x44)
        (0x044 => pub uart0_rx: ReadWrite<u32, FMUX_UART0_RX::Register>),

        /// 保留 (偏移 0x48)
        (0x048 => _reserved1),

        /// EMMC_DAT2 功能选择寄存器 (偏移 0x4C)
        (0x04C => pub emmc_dat2: ReadWrite<u32, FMUX_EMMC_DAT2::Register>),

        /// EMMC_CLK 功能选择寄存器 (偏移 0x50)
        (0x050 => pub emmc_clk: ReadWrite<u32, FMUX_EMMC_CLK::Register>),

        /// EMMC_DAT0 功能选择寄存器 (偏移 0x54)
        (0x054 => pub emmc_dat0: ReadWrite<u32, FMUX_EMMC_DAT0::Register>),

        /// EMMC_DAT3 功能选择寄存器 (偏移 0x58)
        (0x058 => pub emmc_dat3: ReadWrite<u32, FMUX_EMMC_DAT3::Register>),

        /// EMMC_CMD 功能选择寄存器 (偏移 0x5C)
        (0x05C => pub emmc_cmd: ReadWrite<u32, FMUX_EMMC_CMD::Register>),

        /// EMMC_DAT1 功能选择寄存器 (偏移 0x60)
        (0x060 => pub emmc_dat1: ReadWrite<u32, FMUX_EMMC_DAT1::Register>),

        /// JTAG_CPU_TMS 功能选择寄存器 (偏移 0x64)
        (0x064 => pub jtag_cpu_tms: ReadWrite<u32, FMUX_JTAG_CPU_TMS::Register>),

        /// JTAG_CPU_TCK 功能选择寄存器 (偏移 0x68)
        (0x068 => pub jtag_cpu_tck: ReadWrite<u32, FMUX_JTAG_CPU_TCK::Register>),

        /// 保留 (偏移 0x6C)
        (0x06C => _reserved2),

        /// IIC0_SCL 功能选择寄存器 (偏移 0x70)
        (0x070 => pub iic0_scl: ReadWrite<u32, FMUX_IIC0_SCL::Register>),

        /// IIC0_SDA 功能选择寄存器 (偏移 0x74)
        (0x074 => pub iic0_sda: ReadWrite<u32, FMUX_IIC0_SDA::Register>),

        /// AUX0 功能选择寄存器 (偏移 0x78)
        (0x078 => pub aux0: ReadWrite<u32, FMUX_AUX0::Register>),

        /// PWR_VBAT_DET 功能选择寄存器 (偏移 0x7C)
        (0x07C => pub pwr_vbat_det: ReadWrite<u32, FMUX_PWR_VBAT_DET::Register>),

        /// PWR_RSTN 功能选择寄存器 (偏移 0x80)
        (0x080 => pub pwr_rstn: ReadWrite<u32, FMUX_PWR_RSTN::Register>),

        /// PWR_SEQ1 功能选择寄存器 (偏移 0x84)
        (0x084 => pub pwr_seq1: ReadWrite<u32, FMUX_PWR_SEQ1::Register>),

        /// PWR_SEQ2 功能选择寄存器 (偏移 0x88)
        (0x088 => pub pwr_seq2: ReadWrite<u32, FMUX_PWR_SEQ2::Register>),

        /// 保留 (偏移 0x8C)
        (0x08C => _reserved3),

        /// PWR_WAKEUP0 功能选择寄存器 (偏移 0x90)
        (0x090 => pub pwr_wakeup0: ReadWrite<u32, FMUX_PWR_WAKEUP0::Register>),

        /// 保留 (偏移 0x94)
        (0x094 => _reserved4),

        /// PWR_BUTTON1 功能选择寄存器 (偏移 0x98)
        (0x098 => pub pwr_button1: ReadWrite<u32, FMUX_PWR_BUTTON1::Register>),

        /// 保留 (偏移 0x9C)
        (0x09C => _reserved5),

        /// XTAL_XIN 功能选择寄存器 (偏移 0xA0)
        (0x0A0 => pub xtal_xin: ReadWrite<u32, FMUX_XTAL_XIN::Register>),

        /// PWR_GPIO0 功能选择寄存器 (偏移 0xA4)
        (0x0A4 => pub pwr_gpio0: ReadWrite<u32, FMUX_PWR_GPIO0::Register>),

        /// PWR_GPIO1 功能选择寄存器 (偏移 0xA8)
        (0x0A8 => pub pwr_gpio1: ReadWrite<u32, FMUX_PWR_GPIO1::Register>),

        /// PWR_GPIO2 功能选择寄存器 (偏移 0xAC)
        (0x0AC => pub pwr_gpio2: ReadWrite<u32, FMUX_PWR_GPIO2::Register>),

        /// 保留 (偏移 0xB0-0xF4)
        (0x0B0 => _reserved6),

        /// ADC1 功能选择寄存器 (偏移 0xF8)
        (0x0F8 => pub adc1: ReadWrite<u32, FMUX_ADC1::Register>),

        /// USB_VBUS_DET 功能选择寄存器 (偏移 0xFC)
        (0x0FC => pub usb_vbus_det: ReadWrite<u32, FMUX_USB_VBUS_DET::Register>),

        /// 保留 (偏移 0x100)
        (0x100 => _reserved7),

        /// PKG_TYPE0 功能选择寄存器 (偏移 0x104)
        (0x104 => pub pkg_type0: ReadWrite<u32, FMUX_PKG_TYPE::Register>),

        /// 保留 (偏移 0x108)
        (0x108 => _reserved8),

        /// PKG_TYPE1 功能选择寄存器 (偏移 0x10C)
        (0x10C => pub pkg_type1: ReadWrite<u32, FMUX_PKG_TYPE::Register>),

        /// PKG_TYPE2 功能选择寄存器 (偏移 0x110)
        (0x110 => pub pkg_type2: ReadWrite<u32, FMUX_PKG_TYPE::Register>),

        /// MUX_SPI1_MISO 功能选择寄存器 (偏移 0x114)
        (0x114 => pub mux_spi1_miso: ReadWrite<u32, FMUX_MUX_SPI1_MISO::Register>),

        /// MUX_SPI1_MOSI 功能选择寄存器 (偏移 0x118)
        (0x118 => pub mux_spi1_mosi: ReadWrite<u32, FMUX_MUX_SPI1_MOSI::Register>),

        /// MUX_SPI1_CS 功能选择寄存器 (偏移 0x11C)
        (0x11C => pub mux_spi1_cs: ReadWrite<u32, FMUX_MUX_SPI1_CS::Register>),

        /// MUX_SPI1_SCK 功能选择寄存器 (偏移 0x120)
        (0x120 => pub mux_spi1_sck: ReadWrite<u32, FMUX_MUX_SPI1_SCK::Register>),

        /// 保留 (偏移 0x124-0x128)
        (0x124 => _reserved9),

        /// PAD_ETH_TXP 功能选择寄存器 (偏移 0x128)
        (0x128 => pub pad_eth_txp: ReadWrite<u32, FMUX_PAD_ETH_TXP::Register>),

        /// PAD_ETH_RXP 功能选择寄存器 (偏移 0x12C)
        (0x12C => pub pad_eth_rxp: ReadWrite<u32, FMUX_PAD_ETH_RXP::Register>),

        /// PAD_ETH_RXM 功能选择寄存器 (偏移 0x130)
        (0x130 => pub pad_eth_rxm: ReadWrite<u32, FMUX_PAD_ETH_RXM::Register>),

        /// 保留 (偏移 0x134-0x168)
        (0x134 => _reserved10),

        /// PAD_MIPIRX4N 功能选择寄存器 (偏移 0x16C)
        (0x16C => pub pad_mipirx4n: ReadWrite<u32, FMUX_PAD_MIPIRX4N::Register>),

        /// PAD_MIPIRX4P 功能选择寄存器 (偏移 0x170)
        (0x170 => pub pad_mipirx4p: ReadWrite<u32, FMUX_PAD_MIPIRX4P::Register>),

        /// PAD_MIPIRX3N 功能选择寄存器 (偏移 0x174)
        (0x174 => pub pad_mipirx3n: ReadWrite<u32, FMUX_PAD_MIPIRX3N::Register>),

        /// PAD_MIPIRX3P 功能选择寄存器 (偏移 0x178)
        (0x178 => pub pad_mipirx3p: ReadWrite<u32, FMUX_PAD_MIPIRX3P::Register>),

        /// PAD_MIPIRX2N 功能选择寄存器 (偏移 0x17C)
        (0x17C => pub pad_mipirx2n: ReadWrite<u32, FMUX_PAD_MIPIRX2N::Register>),

        /// PAD_MIPIRX2P 功能选择寄存器 (偏移 0x180)
        (0x180 => pub pad_mipirx2p: ReadWrite<u32, FMUX_PAD_MIPIRX2P::Register>),

        /// PAD_MIPIRX1N 功能选择寄存器 (偏移 0x184)
        (0x184 => pub pad_mipirx1n: ReadWrite<u32, FMUX_PAD_MIPIRX1N::Register>),

        /// PAD_MIPIRX1P 功能选择寄存器 (偏移 0x188)
        (0x188 => pub pad_mipirx1p: ReadWrite<u32, FMUX_PAD_MIPIRX1P::Register>),

        /// PAD_MIPIRX0N 功能选择寄存器 (偏移 0x18C)
        (0x18C => pub pad_mipirx0n: ReadWrite<u32, FMUX_PAD_MIPIRX0N::Register>),

        /// PAD_MIPIRX0P 功能选择寄存器 (偏移 0x190)
        (0x190 => pub pad_mipirx0p: ReadWrite<u32, FMUX_PAD_MIPIRX0P::Register>),

        /// 保留 (偏移 0x194-0x1A0)
        (0x194 => _reserved11),

        /// PAD_MIPI_TXM2 功能选择寄存器 (偏移 0x1A4)
        (0x1A4 => pub pad_mipi_txm2: ReadWrite<u32, FMUX_PAD_MIPI_TXM2::Register>),

        /// PAD_MIPI_TXP2 功能选择寄存器 (偏移 0x1A8)
        (0x1A8 => pub pad_mipi_txp2: ReadWrite<u32, FMUX_PAD_MIPI_TXP2::Register>),

        /// PAD_MIPI_TXM1 功能选择寄存器 (偏移 0x1AC)
        (0x1AC => pub pad_mipi_txm1: ReadWrite<u32, FMUX_PAD_MIPI_TXM1::Register>),

        /// PAD_MIPI_TXP1 功能选择寄存器 (偏移 0x1B0)
        (0x1B0 => pub pad_mipi_txp1: ReadWrite<u32, FMUX_PAD_MIPI_TXP1::Register>),

        /// PAD_MIPI_TXM0 功能选择寄存器 (偏移 0x1B4)
        (0x1B4 => pub pad_mipi_txm0: ReadWrite<u32, FMUX_PAD_MIPI_TXM0::Register>),

        /// PAD_MIPI_TXP0 功能选择寄存器 (偏移 0x1B8)
        (0x1B8 => pub pad_mipi_txp0: ReadWrite<u32, FMUX_PAD_MIPI_TXP0::Register>),

        /// PAD_AUD_AINL_MIC 功能选择寄存器 (偏移 0x1BC)
        (0x1BC => pub pad_aud_ainl_mic: ReadWrite<u32, FMUX_PAD_AUD_AINL_MIC::Register>),

        /// 保留 (偏移 0x1C0-0x1C4)
        (0x1C0 => _reserved12),

        /// PAD_AUD_AOUTR 功能选择寄存器 (偏移 0x1C8)
        (0x1C8 => pub pad_aud_aoutr: ReadWrite<u32, FMUX_PAD_AUD_AOUTR::Register>),

        /// GPIO_RTX 功能选择寄存器 (偏移 0x1CC)
        (0x1CC => pub gpio_rtx: ReadWrite<u32, FMUX_GPIO_RTX::Register>),

        /// GPIO_ZQ 功能选择寄存器 (偏移 0x1D0)
        (0x1D0 => pub gpio_zq: ReadWrite<u32, FMUX_GPIO_ZQ::Register>),

        /// 结束标记
        (0x1D4 => @END),
    }
}
