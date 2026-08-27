// Copyright (c) 2026 璇玑 RelGraph · 算子统一系统 (OUS) · 三联盟
// Licensed under the MIT License.
// 项目仓库: https://gitcode.com/aikjx/mox

//! A2 — Miji 密级 4 级 × Bell-LaPadula 裁决矩阵 (64 tests)
//!
//! 4 user clearances × 4 object levels × 2 read/write × 2 enforce modes = 64

use mox_platform_test_harness::miji::{Clearance, MijiLevel, judge_read, judge_write};

const LABELS: [(u8, &str, MijiLevel); 4] = [
    (1, "Internal",     MijiLevel::Internal),
    (2, "Secret",       MijiLevel::Secret),
    (3, "Confidential", MijiLevel::Confidential),
    (4, "TopSecret",    MijiLevel::TopSecret),
];

fn run_miji_case(user_u8: u8, obj_lvl: MijiLevel, enforce: bool, is_read: bool) {
    let user = Clearance(user_u8);
    let user_ge_obj = user_u8 >= obj_lvl.as_u8();
    if is_read {
        let r = judge_read(user, obj_lvl, enforce);
        if enforce && !user_ge_obj {
            // Simple-Security: no read-up
            assert!(r.is_err(), "read MUST be denied when enforce=true and clearance({}) < obj({:?})",
                    user_u8, obj_lvl);
        } else {
            assert!(r.is_ok(), "read should pass (enforce={}, user={}, obj={:?}): {:?}",
                    enforce, user_u8, obj_lvl, r);
        }
    } else {
        let r = judge_write(user, obj_lvl, enforce);
        let user_le_obj = user_u8 <= obj_lvl.as_u8();
        if enforce && !user_le_obj {
            // *-Property: no write-down (write-up is OK under classical BLP)
            assert!(r.is_err(), "write MUST be denied when enforce=true and clearance({}) > obj({:?})",
                    user_u8, obj_lvl);
        } else {
            assert!(r.is_ok(), "write should pass (enforce={}, user={}, obj={:?}): {:?}",
                    enforce, user_u8, obj_lvl, r);
        }
    }
}

macro_rules! miji_case {
    ($name:ident, $u:expr, $obj:expr, $enf:expr, $r:expr) => {
        #[test] fn $name() { run_miji_case($u, $obj, $enf, $r); }
    };
}

// generate 4 users * 4 objects * 2 enforce * 2 (read/write) — exactly 64
miji_case!(a2_r_u1_obj_i_enf_t_read, 1, MijiLevel::Internal, true, true);
miji_case!(a2_r_u1_obj_i_enf_f_read, 1, MijiLevel::Internal, false, true);
miji_case!(a2_r_u1_obj_s_enf_t_read, 1, MijiLevel::Secret, true, true);
miji_case!(a2_r_u1_obj_s_enf_f_read, 1, MijiLevel::Secret, false, true);
miji_case!(a2_r_u1_obj_c_enf_t_read, 1, MijiLevel::Confidential, true, true);
miji_case!(a2_r_u1_obj_c_enf_f_read, 1, MijiLevel::Confidential, false, true);
miji_case!(a2_r_u1_obj_t_enf_t_read, 1, MijiLevel::TopSecret, true, true);
miji_case!(a2_r_u1_obj_t_enf_f_read, 1, MijiLevel::TopSecret, false, true);

miji_case!(a2_r_u2_obj_i_enf_t_read, 2, MijiLevel::Internal, true, true);
miji_case!(a2_r_u2_obj_i_enf_f_read, 2, MijiLevel::Internal, false, true);
miji_case!(a2_r_u2_obj_s_enf_t_read, 2, MijiLevel::Secret, true, true);
miji_case!(a2_r_u2_obj_s_enf_f_read, 2, MijiLevel::Secret, false, true);
miji_case!(a2_r_u2_obj_c_enf_t_read, 2, MijiLevel::Confidential, true, true);
miji_case!(a2_r_u2_obj_c_enf_f_read, 2, MijiLevel::Confidential, false, true);
miji_case!(a2_r_u2_obj_t_enf_t_read, 2, MijiLevel::TopSecret, true, true);
miji_case!(a2_r_u2_obj_t_enf_f_read, 2, MijiLevel::TopSecret, false, true);

miji_case!(a2_r_u3_obj_i_enf_t_read, 3, MijiLevel::Internal, true, true);
miji_case!(a2_r_u3_obj_i_enf_f_read, 3, MijiLevel::Internal, false, true);
miji_case!(a2_r_u3_obj_s_enf_t_read, 3, MijiLevel::Secret, true, true);
miji_case!(a2_r_u3_obj_s_enf_f_read, 3, MijiLevel::Secret, false, true);
miji_case!(a2_r_u3_obj_c_enf_t_read, 3, MijiLevel::Confidential, true, true);
miji_case!(a2_r_u3_obj_c_enf_f_read, 3, MijiLevel::Confidential, false, true);
miji_case!(a2_r_u3_obj_t_enf_t_read, 3, MijiLevel::TopSecret, true, true);
miji_case!(a2_r_u3_obj_t_enf_f_read, 3, MijiLevel::TopSecret, false, true);

miji_case!(a2_r_u4_obj_i_enf_t_read, 4, MijiLevel::Internal, true, true);
miji_case!(a2_r_u4_obj_i_enf_f_read, 4, MijiLevel::Internal, false, true);
miji_case!(a2_r_u4_obj_s_enf_t_read, 4, MijiLevel::Secret, true, true);
miji_case!(a2_r_u4_obj_s_enf_f_read, 4, MijiLevel::Secret, false, true);
miji_case!(a2_r_u4_obj_c_enf_t_read, 4, MijiLevel::Confidential, true, true);
miji_case!(a2_r_u4_obj_c_enf_f_read, 4, MijiLevel::Confidential, false, true);
miji_case!(a2_r_u4_obj_t_enf_t_read, 4, MijiLevel::TopSecret, true, true);
miji_case!(a2_r_u4_obj_t_enf_f_read, 4, MijiLevel::TopSecret, false, true);

// WRITE cases (32)
miji_case!(a2_w_u1_obj_i_enf_t_write, 1, MijiLevel::Internal, true, false);
miji_case!(a2_w_u1_obj_i_enf_f_write, 1, MijiLevel::Internal, false, false);
miji_case!(a2_w_u1_obj_s_enf_t_write, 1, MijiLevel::Secret, true, false);
miji_case!(a2_w_u1_obj_s_enf_f_write, 1, MijiLevel::Secret, false, false);
miji_case!(a2_w_u1_obj_c_enf_t_write, 1, MijiLevel::Confidential, true, false);
miji_case!(a2_w_u1_obj_c_enf_f_write, 1, MijiLevel::Confidential, false, false);
miji_case!(a2_w_u1_obj_t_enf_t_write, 1, MijiLevel::TopSecret, true, false);
miji_case!(a2_w_u1_obj_t_enf_f_write, 1, MijiLevel::TopSecret, false, false);

miji_case!(a2_w_u2_obj_i_enf_t_write, 2, MijiLevel::Internal, true, false);
miji_case!(a2_w_u2_obj_i_enf_f_write, 2, MijiLevel::Internal, false, false);
miji_case!(a2_w_u2_obj_s_enf_t_write, 2, MijiLevel::Secret, true, false);
miji_case!(a2_w_u2_obj_s_enf_f_write, 2, MijiLevel::Secret, false, false);
miji_case!(a2_w_u2_obj_c_enf_t_write, 2, MijiLevel::Confidential, true, false);
miji_case!(a2_w_u2_obj_c_enf_f_write, 2, MijiLevel::Confidential, false, false);
miji_case!(a2_w_u2_obj_t_enf_t_write, 2, MijiLevel::TopSecret, true, false);
miji_case!(a2_w_u2_obj_t_enf_f_write, 2, MijiLevel::TopSecret, false, false);

miji_case!(a2_w_u3_obj_i_enf_t_write, 3, MijiLevel::Internal, true, false);
miji_case!(a2_w_u3_obj_i_enf_f_write, 3, MijiLevel::Internal, false, false);
miji_case!(a2_w_u3_obj_s_enf_t_write, 3, MijiLevel::Secret, true, false);
miji_case!(a2_w_u3_obj_s_enf_f_write, 3, MijiLevel::Secret, false, false);
miji_case!(a2_w_u3_obj_c_enf_t_write, 3, MijiLevel::Confidential, true, false);
miji_case!(a2_w_u3_obj_c_enf_f_write, 3, MijiLevel::Confidential, false, false);
miji_case!(a2_w_u3_obj_t_enf_t_write, 3, MijiLevel::TopSecret, true, false);
miji_case!(a2_w_u3_obj_t_enf_f_write, 3, MijiLevel::TopSecret, false, false);

miji_case!(a2_w_u4_obj_i_enf_t_write, 4, MijiLevel::Internal, true, false);
miji_case!(a2_w_u4_obj_i_enf_f_write, 4, MijiLevel::Internal, false, false);
miji_case!(a2_w_u4_obj_s_enf_t_write, 4, MijiLevel::Secret, true, false);
miji_case!(a2_w_u4_obj_s_enf_f_write, 4, MijiLevel::Secret, false, false);
miji_case!(a2_w_u4_obj_c_enf_t_write, 4, MijiLevel::Confidential, true, false);
miji_case!(a2_w_u4_obj_c_enf_f_write, 4, MijiLevel::Confidential, false, false);
miji_case!(a2_w_u4_obj_t_enf_t_write, 4, MijiLevel::TopSecret, true, false);
miji_case!(a2_w_u4_obj_t_enf_f_write, 4, MijiLevel::TopSecret, false, false);

#[test]
fn a2_z_total_labels_match_4levels() {
    assert_eq!(LABELS.len(), 4);
}
