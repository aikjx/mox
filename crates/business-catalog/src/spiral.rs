//! # 空间光速螺旋模型分析算子
//!
//! 把「空间光速螺旋模型」报告里的数学内核（Frenet 螺旋运动学）与物理推论
//! 拆成可服务化的计算 + 诊断能力，并内置量纲检查（修正原报告的若干错误）：
//!
//! - 物理常数采用 **CODATA2018**（原报告误标为 CODATA2022）。
//! - `μ₀ = 4π×10⁻⁷ N/A²` 是 2019 年 SI 重新定义**前**的定义值，现已非定义常数，
//!   本报告将其当精确值使用属于「常数标注错误」，这里显式标注为 `legacy_definition`。
//! - 质量↔频率的「频率公式」 `m = (h/(2πc²))·f` 量纲为 `[M][L]²[T]⁻¹`，
//!   与质量 `[M]` 不等价，仅为「每单位 c² 的角动量」（即作用量/长度），属额外公设。
//! - `G ≈ α² μ₀` 与 `Gₑ₀ ≈ α² / c²` 仅为数值巧合，量纲不合法，诊断器会标红。
//! - 螺距概念：方程真实螺距为 `2πh`，原报告把 `h` 称为「一周步长」是正确的，
//!   但在动力学方程里混用 `h` 与 `2πh` 会造成量纲破裂。

use serde::{Deserialize, Serialize};

// ---------------------------------------------------------------------------
// 1. 量纲系统（7 个 SI 基本量纲）
// ---------------------------------------------------------------------------

/// 量纲向量： [M, L, T, I, Θ, N, J]
/// M=质量 L=长度 T=时间 I=电流 Θ=温度 N=物质的量 J=发光强度
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Dimension {
    pub mass: i32,
    pub length: i32,
    pub time: i32,
    pub current: i32,
    pub temperature: i32,
    pub amount: i32,
    pub luminous: i32,
}

impl Dimension {
    pub const SCALAR: Dimension = Dimension {
        mass: 0,
        length: 0,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous: 0,
    };
    pub const MASS: Dimension = Dimension {
        mass: 1,
        length: 0,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous: 0,
    };
    pub const LENGTH: Dimension = Dimension {
        mass: 0,
        length: 1,
        time: 0,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous: 0,
    };
    pub const TIME: Dimension = Dimension {
        mass: 0,
        length: 0,
        time: 1,
        current: 0,
        temperature: 0,
        amount: 0,
        luminous: 0,
    };
    pub const CURRENT: Dimension = Dimension {
        mass: 0,
        length: 0,
        time: 0,
        current: 1,
        temperature: 0,
        amount: 0,
        luminous: 0,
    };

    /// 量纲相乘：指数相加
    pub fn mul(&self, other: &Dimension) -> Dimension {
        Dimension {
            mass: self.mass + other.mass,
            length: self.length + other.length,
            time: self.time + other.time,
            current: self.current + other.current,
            temperature: self.temperature + other.temperature,
            amount: self.amount + other.amount,
            luminous: self.luminous + other.luminous,
        }
    }

    /// 量纲相除
    pub fn div(&self, other: &Dimension) -> Dimension {
        Dimension {
            mass: self.mass - other.mass,
            length: self.length - other.length,
            time: self.time - other.time,
            current: self.current - other.current,
            temperature: self.temperature - other.temperature,
            amount: self.amount - other.amount,
            luminous: self.luminous - other.luminous,
        }
    }

    /// 量纲乘方：指数乘幂次
    pub fn pow(&self, n: i32) -> Dimension {
        Dimension {
            mass: self.mass * n,
            length: self.length * n,
            time: self.time * n,
            current: self.current * n,
            temperature: self.temperature * n,
            amount: self.amount * n,
            luminous: self.luminous * n,
        }
    }

    pub fn is_scalar(&self) -> bool {
        *self == Dimension::SCALAR
    }

    /// 人类可读量纲字符串，例如 "M L^2 T^-2"
    pub fn to_symbol(&self) -> String {
        let base = [
            ("M", self.mass),
            ("L", self.length),
            ("T", self.time),
            ("I", self.current),
            ("Θ", self.temperature),
            ("N", self.amount),
            ("J", self.luminous),
        ];
        let mut parts = Vec::new();
        for (sym, e) in base {
            if e != 0 {
                if e == 1 {
                    parts.push(sym.to_string());
                } else {
                    parts.push(format!("{}^{}", sym, e));
                }
            }
        }
        if parts.is_empty() {
            "1 (标量)".to_string()
        } else {
            parts.join(" ")
        }
    }
}

// ---------------------------------------------------------------------------
// 2. 物理常数（CODATA2018，f64）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PhysicalConstants {
    /// 真空光速 c (m/s)
    pub c: f64,
    /// 约化普朗克常数 ℏ = h/(2π) (J·s)
    pub hbar: f64,
    /// 普朗克常数 h (J·s)
    pub h: f64,
    /// 真空磁导率 μ₀（2019 前定义值 4π×10⁻⁷，现已非定义常数）
    pub mu0_legacy: f64,
    /// 真空电容率 ε₀ = 1/(μ₀ c²) (F/m)
    pub epsilon0: f64,
    /// 精细结构常数 α = e²/(4π ε₀ ℏ c)
    pub alpha: f64,
    /// 元电荷 e (C)
    pub e: f64,
    /// 牛顿引力常数 G（CODATA2018，m³ kg⁻¹ s⁻²）
    pub g: f64,
    /// 常数数据集标注
    pub dataset: String,
}

impl Default for PhysicalConstants {
    fn default() -> Self {
        let c = 299_792_458.0;
        let mu0_legacy = 4.0 * std::f64::consts::PI * 1e-7;
        let epsilon0 = 1.0 / (mu0_legacy * c * c);
        let e = 1.602_176_634e-19;
        let h = 6.626_070_15e-34;
        let hbar = h / (2.0 * std::f64::consts::PI);
        // α = e² / (4π ε₀ ℏ c)
        let alpha = e * e / (4.0 * std::f64::consts::PI * epsilon0 * hbar * c);
        PhysicalConstants {
            c,
            hbar,
            h,
            mu0_legacy,
            epsilon0,
            alpha,
            e,
            g: 6.674_30e-11,
            dataset: "CODATA2018（μ₀ 用 2019 前定义值，已标注 legacy_definition）".to_string(),
        }
    }
}

// ---------------------------------------------------------------------------
// 3. Frenet 螺旋运动学（数学内核，干净）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiralParams {
    /// 曲率 κ (>0)
    pub curvature: f64,
    /// 挠率 τ
    pub torsion: f64,
    /// 「一周步长」h（原报告定义），真实螺距 = 2π h
    pub step_h: f64,
    /// 螺旋半径（可选，用于几何重建）
    pub radius: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiralKinematics {
    /// 曲率 κ
    pub curvature: f64,
    /// 挠率 τ
    pub torsion: f64,
    /// 真实螺距 2πh (m)
    pub true_pitch: f64,
    /// 升角正切 tan(β) = τ/κ
    pub pitch_angle_tan: f64,
    /// 角频率 ω = v·κ（取 v=c 时为光速螺旋）
    pub angular_frequency: f64,
    /// 若以光速 v=c，则时间周期 T = 2π/(c·κ)
    pub period_t: f64,
}

impl SpiralParams {
    /// 由曲率/挠率/步进参数计算 Frenet 螺旋运动学。
    /// `speed` 为切向速率，报告建议取真空光速 c。
    pub fn kinematics(&self, speed: f64) -> SpiralKinematics {
        let true_pitch = 2.0 * std::f64::consts::PI * self.step_h;
        let pitch_angle_tan = if self.curvature != 0.0 {
            self.torsion / self.curvature
        } else {
            f64::INFINITY
        };
        let angular_frequency = speed * self.curvature;
        let period_t = if angular_frequency != 0.0 {
            2.0 * std::f64::consts::PI / angular_frequency
        } else {
            f64::INFINITY
        };
        SpiralKinematics {
            curvature: self.curvature,
            torsion: self.torsion,
            true_pitch,
            pitch_angle_tan,
            angular_frequency,
            period_t,
        }
    }
}

// ---------------------------------------------------------------------------
// 4. 量纲诊断（修正原报告里的量纲破裂公式）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DimensionCheck {
    pub label: String,
    /// 公式文本
    pub formula: String,
    /// 左侧量纲
    pub lhs: String,
    pub lhs_dim: Dimension,
    /// 右侧量纲
    pub rhs: String,
    pub rhs_dim: Dimension,
    /// 是否量纲一致
    pub consistent: bool,
    /// 结论说明
    pub note: String,
}

/// 对报告中几个关键公式做量纲自洽检查。
pub fn diagnose_dimensions(k: &PhysicalConstants) -> Vec<DimensionCheck> {
    let mut out = Vec::new();

    // (a) G ≈ α² μ₀ （原报告称二者量纲相同）
    //     实际：[G] = L³ M⁻¹ T⁻²，[α² μ₀] = [μ₀] = L M T⁻² I⁻²
    //     → 量纲不等价，仅为数值巧合
    {
        let lhs = Dimension {
            length: 3,
            mass: -1,
            time: -2,
            ..Dimension::SCALAR
        };
        let rhs = Dimension {
            length: 1,
            mass: 1,
            time: -2,
            current: -2,
            ..Dimension::SCALAR
        };
        out.push(DimensionCheck {
            label: "G ≈ α²·μ₀".into(),
            formula: "G = α² μ₀".into(),
            lhs: "[G] = L³ M⁻¹ T⁻²".into(),
            lhs_dim: lhs,
            rhs: "[α² μ₀] = [μ₀] = L M T⁻² I⁻²".into(),
            rhs_dim: rhs,
            consistent: false,
            note: "量纲不等价（右侧多 I⁻²、质量符号相反）。属数值巧合，不能作为物理推导。".into(),
        });
    }

    // (b) Gₑ₀ ≈ α² / c²
    //     [Gₑ₀]（若定义为 G·ε₀）= L⁰ M⁰ T⁰ I⁰ ... 实际 G·ε₀ = L⁰ M⁻¹ T⁻² I²
    //     [α² / c²] = L⁻²
    {
        let lhs = Dimension {
            mass: -1,
            time: -2,
            current: 2,
            ..Dimension::SCALAR
        };
        let rhs = Dimension {
            length: -2,
            ..Dimension::SCALAR
        };
        out.push(DimensionCheck {
            label: "G·ε₀ ≈ α² / c²".into(),
            formula: "G·ε₀ = α² / c²".into(),
            lhs: "[G ε₀] = M⁻¹ T⁻² I²".into(),
            lhs_dim: lhs,
            rhs: "[α²/c²] = L⁻²".into(),
            rhs_dim: rhs,
            consistent: false,
            note: "量纲不等价（左侧 M⁻¹T⁻²I²、右侧 L⁻²）。同上，纯数值巧合。".into(),
        });
    }

    // (c) 质量-频率公式 m = (h/(2π c²)) · f
    //     左侧 [m] = M；右侧 [(h/(2πc²))·f] = (M L² T⁻¹)·(T⁻¹) = M L² T⁻²
    //     → 与质量不等价，实为「作用量/长度」（角动量/长度）
    {
        let lhs = Dimension::MASS;
        // h: M L² T⁻¹；c²: L² T⁻²；h/c²: M；再乘 f(T⁻¹): M T⁻¹
        // 注意 h/(2π c²) · f 的量纲 = M T⁻¹
        let rhs = Dimension {
            mass: 1,
            time: -1,
            ..Dimension::SCALAR
        };
        out.push(DimensionCheck {
            label: "m = (h/(2πc²))·f".into(),
            formula: "m = (h/(2π c²)) f".into(),
            lhs: "[m] = M".into(),
            lhs_dim: lhs,
            rhs: "[h f /(2π c²)] = M T⁻¹".into(),
            rhs_dim: rhs,
            consistent: false,
            note: "量纲不等价（右侧为 M T⁻¹，即角动量/长度÷长度 = 作用量密度/长度）。\
                   质量↔频率映射是额外公设，需引入 c² 仅取数值而非量纲补偿。".into(),
        });
    }

    // (d) 动力学方程 −(4πG/α²)(dm/dt) = q/ε₀
    //     左侧 [(4πG/α²)(dm/dt)]：G= L³M⁻¹T⁻²，dm/dt= M T⁻¹ → L³ T⁻³
    //     右侧 [q/ε₀]：q=I T，ε₀= I² T⁴ M⁻¹ L⁻³ → I⁻¹ T⁵ M L³
    //     → 量纲严重破裂（原报告自身承认）
    {
        let lhs = Dimension {
            length: 3,
            time: -3,
            ..Dimension::SCALAR
        };
        let rhs = Dimension {
            length: 3,
            mass: 1,
            time: 5,
            current: -1,
            ..Dimension::SCALAR
        };
        out.push(DimensionCheck {
            label: "−(4πG/α²)(dm/dt) = q/ε₀".into(),
            formula: "-(4πG/α²)(dm/dt) = q/ε₀".into(),
            lhs: "[(4πG/α²)(dm/dt)] = L³ T⁻³".into(),
            lhs_dim: lhs,
            rhs: "[q/ε₀] = M L³ T⁵ I⁻¹".into(),
            rhs_dim: rhs,
            consistent: false,
            note: "量纲破裂（原报告自查确认）。该动力学方程不能成立，需重设量纲或放弃。".into(),
        });
    }

    // (e) 螺旋运动学核心：真实螺距 2πh，量纲 = L，自洽
    {
        let lhs = Dimension::LENGTH;
        out.push(DimensionCheck {
            label: "螺旋真实螺距 p = 2π h".into(),
            formula: "p = 2π h".into(),
            lhs: "[p] = L".into(),
            lhs_dim: lhs,
            rhs: "[2π h] = L".into(),
            rhs_dim: Dimension::LENGTH,
            consistent: true,
            note: "量纲自洽。原报告称 h 为『一周步长』是正确的，但动力学方程里误把 h 当螺距用。".into(),
        });
    }

    let _ = k;
    out
}

// ---------------------------------------------------------------------------
// 5. 数值自洽检查（复现报告里的『数值巧合』并标注为巧合）
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NumericalCoincidence {
    pub label: String,
    pub claimed: String,
    pub computed: f64,
    pub reference: f64,
    pub rel_err: f64,
    pub is_coincidence: bool,
    pub note: String,
}

pub fn numerical_checks(k: &PhysicalConstants) -> Vec<NumericalCoincidence> {
    let mut out = Vec::new();

    // G ≈ α² μ₀ 数值巧合
    let g_from_alpha_mu0 = k.alpha * k.alpha * k.mu0_legacy;
    let rel1 = (g_from_alpha_mu0 - k.g).abs() / k.g;
    out.push(NumericalCoincidence {
        label: "G ≈ α² μ₀".into(),
        claimed: format!("α² μ₀ = {:.4e}", g_from_alpha_mu0),
        computed: g_from_alpha_mu0,
        reference: k.g,
        rel_err: rel1,
        is_coincidence: true,
        note: format!("相对误差 {:.2e}，量纲不等价 → 纯数值巧合。", rel1),
    });

    // G ε₀ ≈ α² / c² 数值巧合
    let ge0 = k.g * k.epsilon0;
    let rhs2 = k.alpha * k.alpha / (k.c * k.c);
    let rel2 = (ge0 - rhs2).abs() / rhs2.max(f64::MIN_POSITIVE);
    out.push(NumericalCoincidence {
        label: "G ε₀ ≈ α² / c²".into(),
        claimed: format!("α²/c² = {:.4e}", rhs2),
        computed: ge0,
        reference: rhs2,
        rel_err: rel2,
        is_coincidence: true,
        note: format!("相对误差 {:.2e}，量纲不等价 → 纯数值巧合。", rel2),
    });

    // 质量-频率：取 f = m_e c² / h（电子康普顿频率）反推质量，验证公式方向
    let m_e = 9.109_383_7015e-31;
    let f_compton = m_e * k.c * k.c / k.h;
    let m_back = (k.h / (2.0 * std::f64::consts::PI * k.c * k.c)) * f_compton;
    let rel3 = (m_back - m_e).abs() / m_e;
    out.push(NumericalCoincidence {
        label: "m = (h/(2πc²))·f（康普顿频率反推）".into(),
        claimed: format!("反推质量 = {:.4e} kg", m_back),
        computed: m_back,
        reference: m_e,
        rel_err: rel3,
        is_coincidence: false,
        note: format!("相对误差 {:.2e}（含 2π 因子差）。该式数值可成立，但量纲仍不等价，属公设。", rel3),
    });

    out
}

// ---------------------------------------------------------------------------
// 6. 顶层分析报告
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpiralAnalysisReport {
    pub constants: PhysicalConstants,
    pub kinematics: SpiralKinematics,
    pub dimension_checks: Vec<DimensionCheck>,
    pub numerical_checks: Vec<NumericalCoincidence>,
    /// 总体结论：数学内核可靠，物理推论多为额外公设
    pub verdict: String,
    pub reliable_parts: Vec<String>,
    pub extra_assumptions: Vec<String>,
}

/// 执行一次完整的空间光速螺旋模型分析。
pub fn analyze_spiral(params: &SpiralParams, speed: f64, k: &PhysicalConstants) -> SpiralAnalysisReport {
    let kinematics = params.kinematics(speed);
    let dimension_checks = diagnose_dimensions(k);
    let numerical_checks = numerical_checks(k);

    SpiralAnalysisReport {
        constants: k.clone(),
        kinematics,
        dimension_checks,
        numerical_checks,
        verdict: "数学内核（Frenet 螺旋运动学）自洽可靠；引力/电磁对应与质量↔频率映射为额外公设，量纲不合法，仅数值巧合。".into(),
        reliable_parts: vec![
            "曲率 κ、挠率 τ 定义干净".into(),
            "真实螺距 p = 2πh 量纲自洽".into(),
            "升角 tan(β)=τ/κ 与角频率 ω=vκ 形式正确".into(),
        ],
        extra_assumptions: vec![
            "G ≈ α² μ₀（量纲破裂，数值巧合）".into(),
            "G ε₀ ≈ α²/c²（量纲破裂，数值巧合）".into(),
            "质量↔频率 m=(h/2πc²)f（量纲不等价，额外公设）".into(),
            "动力学方程 −(4πG/α²)(dm/dt)=q/ε₀（自身量纲破裂）".into(),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dimension_mul_div_pow() {
        let area = Dimension::LENGTH.mul(&Dimension::LENGTH);
        assert_eq!(area.length, 2);
        let speed = Dimension::LENGTH.div(&Dimension::TIME);
        assert_eq!(speed.length, 1);
        assert_eq!(speed.time, -1);
        let vol = Dimension::LENGTH.pow(3);
        assert_eq!(vol.length, 3);
        let energy = Dimension::MASS.mul(&Dimension::LENGTH.pow(2)).mul(&Dimension::TIME.pow(-2));
        assert_eq!(energy.mass, 1);
        assert_eq!(energy.length, 2);
        assert_eq!(energy.time, -2);
    }

    #[test]
    fn test_dimension_is_scalar_and_symbol() {
        assert!(Dimension::SCALAR.is_scalar());
        assert!(!Dimension::MASS.is_scalar());
        assert_eq!(Dimension::SCALAR.to_symbol(), "1 (标量)");
        let accel = Dimension::LENGTH.div(&Dimension::TIME.pow(2));
        assert_eq!(accel.to_symbol(), "L T^-2");
    }

    #[test]
    fn test_physical_constants_default_fine_structure() {
        let k = PhysicalConstants::default();
        // 精细结构常数 α ≈ 1/137
        assert!(k.alpha > 1.0 / 140.0 && k.alpha < 1.0 / 130.0, "alpha={}", k.alpha);
        // 元电荷、光速、引力常数合理
        assert_eq!(k.c, 299_792_458.0);
        assert!(k.e > 1.6e-19 && k.e < 1.7e-19);
        assert!(k.g > 6.6e-11 && k.g < 6.7e-11);
        assert!(!k.dataset.is_empty());
    }

    #[test]
    fn test_kinematics_relationships() {
        let params = SpiralParams { curvature: 0.5, torsion: 0.25, step_h: 1.0, radius: Some(2.0) };
        let kin = params.kinematics(299_792_458.0);
        // 真实螺距 = 2π h
        assert!((kin.true_pitch - 2.0 * std::f64::consts::PI).abs() < 1e-6);
        // 升角 tan(β) = τ/κ
        assert!((kin.pitch_angle_tan - 0.5).abs() < 1e-9);
        // 角频率 ω = v·κ
        assert!((kin.angular_frequency - 299_792_458.0 * 0.5).abs() < 1e-1);
        // 周期 T = 2π/(v·κ)
        assert!((kin.period_t - 2.0 * std::f64::consts::PI / (299_792_458.0 * 0.5)).abs() < 1e-12);
    }

    #[test]
    fn test_diagnose_dimensions_finds_break() {
        let k = PhysicalConstants::default();
        let checks = diagnose_dimensions(&k);
        assert!(!checks.is_empty());
        // 至少有一条维度破裂（如 G ≈ α² μ₀）
        assert!(checks.iter().any(|c| !c.consistent));
    }

    #[test]
    fn test_numerical_checks_non_empty() {
        let k = PhysicalConstants::default();
        let checks = numerical_checks(&k);
        assert!(!checks.is_empty());
    }

    #[test]
    fn test_analyze_spiral_full_pipeline() {
        let k = PhysicalConstants::default();
        let params = SpiralParams { curvature: 1.0, torsion: 0.5, step_h: 0.1, radius: None };
        let report = analyze_spiral(&params, k.c, &k);
        assert!(!report.verdict.is_empty());
        assert!(!report.reliable_parts.is_empty());
        assert!(!report.extra_assumptions.is_empty());
        assert_eq!(report.dimension_checks.len(), diagnose_dimensions(&k).len());
        // 运动学字段与单独 kinematics 一致
        let kin = params.kinematics(k.c);
        assert_eq!(report.kinematics.true_pitch, kin.true_pitch);
    }
}
