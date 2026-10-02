//! 种子化确定性随机（M0 §23.1 / §48）。
//!
//! 测试禁止依赖真实随机数：same seed + same calls = same sequence。

/// xorshift64* 生成器：小、确定性、跨平台一致。
#[derive(Debug, Clone)]
pub struct DetRandom {
    state: u64,
}

impl DetRandom {
    /// 用任意种子构造；0 会被映射为非零状态。
    pub fn new(seed: u64) -> Self {
        Self {
            state: if seed == 0 {
                0x9E37_79B9_7F4A_7C15
            } else {
                seed
            },
        }
    }

    pub fn next_u64(&mut self) -> u64 {
        let mut x = self.state;
        x ^= x >> 12;
        x ^= x << 25;
        x ^= x >> 27;
        self.state = x;
        x.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// [0, 1) 均匀分布。
    pub fn next_f64(&mut self) -> f64 {
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }

    /// 用确定性字节填充缓冲区。
    pub fn fill_bytes(&mut self, dest: &mut [u8]) {
        for chunk in dest.chunks_mut(8) {
            let value = self.next_u64().to_le_bytes();
            chunk.copy_from_slice(&value[..chunk.len()]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_produces_same_sequence() {
        let mut a = DetRandom::new(42);
        let mut b = DetRandom::new(42);
        for _ in 0..16 {
            assert_eq!(a.next_u64(), b.next_u64());
        }
    }

    #[test]
    fn different_seeds_diverge() {
        let mut a = DetRandom::new(1);
        let mut b = DetRandom::new(2);
        assert_ne!(a.next_u64(), b.next_u64());
    }

    #[test]
    fn f64_stays_in_unit_interval() {
        let mut rng = DetRandom::new(7);
        for _ in 0..1000 {
            let v = rng.next_f64();
            assert!((0.0..1.0).contains(&v));
        }
    }

    #[test]
    fn fill_bytes_is_deterministic() {
        let mut a = DetRandom::new(9);
        let mut b = DetRandom::new(9);
        let mut buf_a = [0u8; 31]; // 非 8 倍数长度，覆盖尾块
        let mut buf_b = [0u8; 31];
        a.fill_bytes(&mut buf_a);
        b.fill_bytes(&mut buf_b);
        assert_eq!(buf_a, buf_b);
    }
}
