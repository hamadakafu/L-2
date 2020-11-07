use std::num::Wrapping;

use crate::tlwe;
use crate::torus;
use crate::torus::Torus01;
use crate::trgsw;
use crate::trlwe;

/// mu_bit: test_vector.bの係数の2の係数、3なら1/8
/// N: TRLWEの次元
/// b - a*sが[0, N)で1/8が復元
/// b - a*sが[N, 2N)で-1/8が復元
/// test_vectorの正負を逆にする必要がある
/// なぜなら b - a*s が0.125のとき固定浮動小数点の上11bitは[0, N)
/// なぜなら b - a*s が0.875のとき固定浮動小数点の上11bitは[N, 2N)
pub fn blind_rotate(
    tlwe: tlwe::TLWE,
    trgsws: Vec<trgsw::TRGSW>,
    mu_bit: u32,
    N: u32,
    N_bit: u32,
) -> trlwe::TRLWE {
    // TLWEのサイズ
    let n = tlwe.a.coef.len();

    let tmp_upper = vec![Wrapping(!(1 << (32 - mu_bit))); (N / 2) as usize];
    let mut tmp_lower = vec![Wrapping(1 << (32 - mu_bit)); (N / 2) as usize];
    tmp_lower.extend(tmp_upper.into_iter());

    let mut test_vector: trlwe::TRLWE = trlwe::TRLWE::new_obvious_with_fix(tmp_lower);

    // 1. bの分をrotate
    let mut _2n_b: u32 =
        2 * N - ((tlwe.b.fix + Wrapping(1 << (31 - N_bit - 1))).0 >> (32 - N_bit - 1));
    debug_assert!(
        _2n_b < 2 * N,
        "_2n_b({}) must be less than 2N({})",
        _2n_b,
        2 * N
    );
    if _2n_b >= N {
        _2n_b -= N;
        for i in 0..test_vector.b.coef.len() as usize {
            test_vector.b.coef[i] = -test_vector.b.coef[i];
        }
    }
    test_vector.b.coef.rotate_right(_2n_b as usize);
    for i in 0.._2n_b as usize {
        test_vector.b.coef[i] = -test_vector.b.coef[i];
    }

    // 2. s*aの分をrotate
    // 0の暗号文も回っているが問題ない
    for i in 0..n {
        let mut a_i = (tlwe.a.coef[i].fix + Wrapping(1 << (31 - N_bit - 1))).0 >> (32 - N_bit - 1);
        let zero = test_vector.clone();
        let mut one = test_vector.clone();
        if a_i >= N {
            a_i -= N;
            for i in 0..one.a.coef.len() as usize {
                one.a.coef[i] = -one.a.coef[i];
                one.b.coef[i] = -one.b.coef[i];
            }
        }

        one.a.coef.rotate_right(a_i as usize);
        one.b.coef.rotate_right(a_i as usize);
        for i in 0..a_i as usize {
            one.a.coef[i] = -one.a.coef[i];
            one.b.coef[i] = -one.b.coef[i];
        }
        test_vector = trgsws[i].clone().cmux(zero, one);
    }
    return test_vector;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{params, tlwe, torus, trgsw};
    use quickcheck_macros::quickcheck;
    #[quickcheck]
    fn test_blind_rotate(m: u32) -> bool {
        // 時間かかるのでちょっとだけテスト
        if (m > 10) {
            return true;
        }
        let l = params::l;
        let bg = params::bg as u32;
        let bgbit = params::bgbit;
        let n = params::n;
        let N = params::N;
        let N_bit = params::N_bit;
        let mu = params::mu;
        let mu_bit = params::mu_bit;
        let m = m % 2;
        let (tlwe_c, tlwe_s, e) = tlwe::encrypt(m, n, mu);
        let trlwe_s = trlwe::gen_s(N);
        let trgsws: Vec<trgsw::TRGSW> = tlwe_s
            .iter()
            .map(|sbit| {
                debug_assert!(*sbit == 0 || *sbit == 1);

                let zeros = (0..l * 2)
                    .map(|_| {
                        let m = vec![0; N];
                        let (c, _) = trlwe::encrypt_torus(m, &trlwe_s);
                        c
                    })
                    .collect();
                trgsw::TRGSW::new_with_bin(l, bg, bgbit, *sbit as u8, zeros)
            })
            .collect();
        let blind_trlwe = blind_rotate(tlwe_c, trgsws, mu_bit as u32, N as u32, N_bit as u32);
        let blind_msg = trlwe::decrypt_bin(blind_trlwe, &trlwe_s);
        let mut one_count = 0;
        let mut zero_count = 0;
        for bin in blind_msg {
            if bin == 0 {
                zero_count += 1;
            } else if bin == 1 {
                one_count += 1;
            } else {
                panic!("hgoeheog");
            }
        }
        dbg!(m, zero_count, one_count);
        if m == 0 {
            return 250 <= zero_count && zero_count <= 260;
        } else {
            return 250 <= one_count && one_count <= 260;
        }
    }
}