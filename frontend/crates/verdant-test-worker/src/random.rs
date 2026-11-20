//! CPython's integer-seeded MT19937 and 53-bit `random()` conversion. Keeping
//! the seed algorithm preserves the retained fixture's deterministic dither.

pub(crate) struct Random {
    state: [u32; 624],
    index: usize,
}

impl Random {
    pub(crate) fn new(seed: u32) -> Self {
        let mut random = Self { state: [0; 624], index: 624 };
        random.state[0] = 19_650_218;
        for index in 1..624 {
            let previous = random.state[index - 1];
            random.state[index] = 1_812_433_253u32.wrapping_mul(previous ^ (previous >> 30)).wrapping_add(index as u32);
        }
        let mut index = 1;
        for _ in 0..624 {
            let previous = random.state[index - 1];
            random.state[index] = (random.state[index] ^ (previous ^ (previous >> 30)).wrapping_mul(1_664_525)).wrapping_add(seed);
            index += 1;
            if index == 624 {
                random.state[0] = random.state[623];
                index = 1;
            }
        }
        for _ in 0..623 {
            let previous = random.state[index - 1];
            random.state[index] = (random.state[index] ^ (previous ^ (previous >> 30)).wrapping_mul(1_566_083_941)).wrapping_sub(index as u32);
            index += 1;
            if index == 624 {
                random.state[0] = random.state[623];
                index = 1;
            }
        }
        random.state[0] = 0x8000_0000;
        random
    }

    fn next(&mut self) -> u32 {
        if self.index == 624 {
            for index in 0..624 {
                let joined = (self.state[index] & 0x8000_0000) | (self.state[(index + 1) % 624] & 0x7fff_ffff);
                self.state[index] = self.state[(index + 397) % 624] ^ (joined >> 1) ^ if joined & 1 == 0 { 0 } else { 0x9908_b0df };
            }
            self.index = 0;
        }
        let mut value = self.state[self.index];
        self.index += 1;
        value ^= value >> 11;
        value ^= (value << 7) & 0x9d2c_5680;
        value ^= (value << 15) & 0xefc6_0000;
        value ^= value >> 18;
        value
    }

    pub(crate) fn uniform_signed(&mut self) -> f64 {
        let high = self.next() >> 5;
        let low = self.next() >> 6;
        let unit = (high as f64 * 67_108_864.0 + low as f64) / 9_007_199_254_740_992.0;
        -1.0 + 2.0 * unit
    }
}
